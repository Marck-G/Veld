mod constants;

use std::{fs, path::Path};

use chrono::Utc;
use petgraph::{Graph, visit::EdgeRef};
use serde::{Deserialize, Serialize};
use veld_error::{ErrorCode, VeldResult, map_veld_error, veld_error};

use crate::constants::{FILE_LOCK_NAME, SELF_VERSION};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockMetadata {
    /// version of veld that make the lock
    pub generated_by: String,
    /// ISO8601 timestamp
    pub generated_at: String,
    /// manifest (veld.toml) hash for check any manual check of the file
    pub manifest_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockedPackage {
    pub name: String,
    pub version: semver::Version,
    pub repository: String,
    pub commit: String,
    pub artifact_id: String,
    pub required_by: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    pub name: String,
    pub version: semver::Version,
    pub repository: String,
    pub commit: String,
    pub artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// Metadata de generación
    pub metadata: LockMetadata,

    /// Hash del build profile usado para generar este lock
    pub profile_hash: String,

    /// Paquetes resueltos (ordenados por name para serialización determinista)
    #[serde(rename = "package")]
    pub packages: Vec<LockedPackage>,

    /// Hash de integridad del lockfile (opcional, calculado al escribir)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

impl Lockfile {
    pub fn new(packages: Vec<LockedPackage>, profile_hash: String, manifest_hash: String) -> Self {
        let mut packages = packages;
        packages.sort(); // Ensure deterministic ordering
        Self {
            metadata: LockMetadata {
                generated_by: SELF_VERSION.to_string(),
                generated_at: chrono::Utc::now().to_rfc3339(),
                manifest_hash,
            },
            profile_hash,
            packages,
            checksum: None,
        }
    }

    /// Crear lockfile desde un DAG resuelto
    pub fn from_resolved_graph(
        dag: &Graph<ResolvedPackage, ()>,
        profile_hash: String,
        manifest_hash: String,
    ) -> VeldResult<Self> {
        use petgraph::visit::IntoNodeReferences;

        // Convertir nodos del DAG a LockedPackages
        let mut packages: Vec<LockedPackage> = dag
            .node_references()
            .map(|(idx, pkg)| {
                // Encontrar qué paquetes requieren este
                let required_by: Vec<String> = dag
                    .edges_directed(idx, petgraph::Direction::Incoming)
                    .map(|edge| {
                        let parent = &dag[edge.source()];
                        parent.name.clone()
                    })
                    .collect();

                LockedPackage {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                    repository: pkg.repository.clone(),
                    commit: pkg.commit.clone(),
                    artifact_id: pkg.artifact_id.clone(),
                    required_by: if required_by.is_empty() {
                        vec!["root".to_string()]
                    } else {
                        required_by
                    },
                    content_hash: None, // Computed later if needed
                }
            })
            .collect();

        // Ordenar para serialización determinista
        packages.sort();

        let metadata = LockMetadata {
            generated_by: format!("veld {}", env!("CARGO_PKG_VERSION")),
            generated_at: Utc::now().to_rfc3339(),
            manifest_hash,
        };

        Ok(Self {
            metadata,
            profile_hash,
            packages,
            checksum: None, // Computed when writing
        })
    }

    /// Escribir lockfile a disco con checksum
    pub fn write(&self, path: impl AsRef<Path>) -> VeldResult<()> {
        // Serializar sin checksum primero
        let mut lock = self.clone();
        lock.checksum = None;

        let toml_content = map_veld_error!(
            toml::to_string_pretty(&lock),
            ErrorCode::FileError(FILE_LOCK_NAME.into(), "Can't write lock file".into())
        )?;

        // Computar checksum del contenido
        let checksum = blake3::hash(toml_content.as_bytes()).to_hex().to_string();

        // Re-serializar con checksum
        lock.checksum = Some(checksum);
        let final_content = map_veld_error!(
            toml::to_string_pretty(&lock),
            ErrorCode::FileError(
                FILE_LOCK_NAME.into(),
                "Failed to serialize lockfile with checksum".into()
            )
        )?;

        map_veld_error!(
            fs::write(path.as_ref(), final_content),
            ErrorCode::FileError(FILE_LOCK_NAME.into(), "Failed to write lockfile".into())
        )?;

        Ok(())
    }

    /// Leer y validar lockfile
    pub fn read(path: impl AsRef<Path>) -> VeldResult<Self> {
        let content = map_veld_error!(
            fs::read_to_string(path.as_ref()),
            ErrorCode::FileError(FILE_LOCK_NAME.into(), "Failed to read lockfile".into())
        )?;

        let mut lock: Lockfile = map_veld_error!(
            toml::from_str(&content),
            ErrorCode::LockfileParseError("Failed to parse lockfile".into())
        )?;

        // Validar checksum si existe
        if let Some(stored_checksum) = &lock.checksum {
            let mut w_lock = lock.clone();
            w_lock.checksum = None;
            let toml_without_checksum = map_veld_error!(
                toml::to_string_pretty(&w_lock),
                ErrorCode::FileError(
                    FILE_LOCK_NAME.into(),
                    "Failed to re-serialize for checksum validation".into()
                )
            )?;

            let computed = blake3::hash(toml_without_checksum.as_bytes())
                .to_hex()
                .to_string();

            if &computed != stored_checksum {
                return Err(veld_error!(ErrorCode::LockfileCorrupted(format!(
                    "Lockfile checksum mismatch: expected {}, got {}",
                    stored_checksum, computed
                ))));
            }

            lock.checksum = Some(stored_checksum.clone());
        }

        Ok(lock)
    }

    /// Validar que el profile hash coincide
    pub fn validate_profile(&self, current_profile_hash: &str) -> VeldResult<()> {
        if self.profile_hash != current_profile_hash {
            return Err(veld_error!(ErrorCode::LockfileCorrupted(format!(
                "Build profile changed. Lock was created with profile hash {}, \
                but current profile is {}. Run `veld install` to update the lock.",
                self.profile_hash, current_profile_hash
            ))));
        }
        Ok(())
    }

    pub fn validate_version(&self) -> VeldResult<()> {
        if self.metadata.generated_by != SELF_VERSION {
            return Err(veld_error!(ErrorCode::LockfileCorrupted(format!(
                "Lockfile version mismatch: expected {}, \
                but current version is {}. Run `veld install` to update the lock.",
                self.metadata.generated_by, SELF_VERSION
            ))));
        }
        Ok(())
    }

    pub fn direct_dependencies(&self) -> Vec<LockedPackage> {
        self.packages
            .iter()
            .filter(|pkg| pkg.required_by.contains(&"root".to_string()))
            .cloned()
            .collect()
    }

    pub fn find_package(&self, name: &str) -> Option<LockedPackage> {
        self.packages.iter().find(|pkg| pkg.name == name).cloned()
    }

    pub fn find_package_by_artifact_id(&self, artifact_id: &str) -> Option<LockedPackage> {
        self.packages
            .iter()
            .find(|pkg| pkg.artifact_id == artifact_id)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::fs;
    use tempfile::NamedTempFile;

    // ============================================================================
    // Fixtures
    // ============================================================================

    #[fixture]
    fn test_profile_hash() -> String {
        "test_profile_hash_123".to_string()
    }

    #[fixture]
    fn test_manifest_hash() -> String {
        "test_manifest_hash_456".to_string()
    }

    #[fixture]
    fn sample_package() -> LockedPackage {
        LockedPackage {
            name: "zlib".to_string(),
            version: semver::Version::new(1, 3, 1),
            repository: "https://github.com/madler/zlib".to_string(),
            commit: "0".repeat(40),
            artifact_id: format!("{:0<34}", "zlib"),
            required_by: vec!["root".to_string()],
            content_hash: None,
        }
    }

    #[fixture]
    fn sample_lockfile(
        sample_package: LockedPackage,
        test_profile_hash: String,
        test_manifest_hash: String,
    ) -> Lockfile {
        Lockfile::new(vec![sample_package], test_profile_hash, test_manifest_hash)
    }

    // Helper function to create test packages
    fn pkg(name: &str, version: semver::Version, required_by: Vec<&str>) -> LockedPackage {
        LockedPackage {
            name: name.to_string(),
            version: version,
            repository: format!("https://github.com/test/{}", name),
            commit: "0".repeat(40),
            artifact_id: format!("{:0<34}", name),
            required_by: required_by.iter().map(|s| s.to_string()).collect(),
            content_hash: None,
        }
    }

    fn lock(packages: Vec<LockedPackage>) -> Lockfile {
        Lockfile::new(
            packages,
            "test_profile_hash_123".to_string(),
            "test_manifest_hash_456".to_string(),
        )
    }

    // ============================================================================
    // Tests de Creación y Construcción
    // ============================================================================

    #[rstest]
    fn test_lockfile_creation_sorts_packages() {
        let packages = vec![
            pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"]),
            pkg("curl", semver::Version::new(7, 88, 0), vec!["root"]),
            pkg("openssl", semver::Version::new(3, 1, 0), vec!["curl"]),
        ];

        let lockfile = lock(packages);

        assert_eq!(lockfile.packages[0].name, "curl");
        assert_eq!(lockfile.packages[1].name, "openssl");
        assert_eq!(lockfile.packages[2].name, "zlib");
    }

    #[rstest]
    fn test_lockfile_empty_packages(test_profile_hash: String, test_manifest_hash: String) {
        let lockfile = Lockfile::new(vec![], test_profile_hash, test_manifest_hash);

        assert!(lockfile.packages.is_empty());
        assert!(lockfile.direct_dependencies().is_empty());
    }

    // ============================================================================
    // Tests de I/O y Serialización
    // ============================================================================

    #[rstest]
    fn test_lockfile_roundtrip(sample_lockfile: Lockfile) {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");

        sample_lockfile
            .write(temp_file.path())
            .expect("Failed to write lockfile");

        let loaded = Lockfile::read(temp_file.path()).expect("Failed to read lockfile");

        assert_eq!(sample_lockfile.profile_hash, loaded.profile_hash);
        assert_eq!(
            sample_lockfile.metadata.manifest_hash,
            loaded.metadata.manifest_hash
        );
        assert_eq!(sample_lockfile.packages, loaded.packages);
        assert!(loaded.checksum.is_some());
    }

    #[rstest]
    fn test_lockfile_checksum_validation() {
        let packages = vec![pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"])];
        let lockfile = lock(packages);

        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        lockfile
            .write(temp_file.path())
            .expect("Failed to write lockfile");

        // Tamper with the file
        let mut content = fs::read_to_string(temp_file.path()).expect("Failed to read");
        content = content.replace("1.3.1", "1.3.2");
        fs::write(temp_file.path(), content).expect("Failed to write tampered file");

        // Reading should fail
        let result = Lockfile::read(temp_file.path());
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("checksum mismatch")
        );
    }

    #[rstest]
    fn test_lockfile_serialization_is_deterministic() {
        let packages1 = vec![
            pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"]),
            pkg("curl", semver::Version::new(7, 88, 0), vec!["root"]),
        ];
        let packages2 = vec![
            pkg("curl", semver::Version::new(7, 88, 0), vec!["root"]),
            pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"]),
        ];

        let lock1 = lock(packages1);
        let lock2 = lock(packages2);

        let mut temp_lock1 = lock1.clone();
        let mut temp_lock2 = lock2.clone();
        temp_lock1.checksum = None;
        temp_lock2.checksum = None;
        temp_lock1.metadata.generated_at = "2024-01-01T00:00:00Z".to_string();
        temp_lock2.metadata.generated_at = "2024-01-01T00:00:00Z".to_string();

        let toml1 = toml::to_string_pretty(&temp_lock1).unwrap();
        let toml2 = toml::to_string_pretty(&temp_lock2).unwrap();

        assert_eq!(toml1, toml2);
    }

    #[rstest]
    fn test_lockfile_read_nonexistent_file() {
        let result = Lockfile::read("/nonexistent/path/veld.lock");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to read lockfile")
        );
    }

    #[rstest]
    fn test_lockfile_write_invalid_path(sample_lockfile: Lockfile) {
        let result = sample_lockfile.write("/invalid/path/that/does/not/exist/veld.lock");
        assert!(result.is_err());
    }

    #[rstest]
    fn test_toml_format_matches_spec() {
        let packages = vec![pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"])];
        let mut lockfile = lock(packages);
        lockfile.checksum = None;
        lockfile.metadata.generated_at = "2026-03-14T10:00:00Z".to_string();
        lockfile.metadata.generated_by = "veld 0.1.0".to_string();

        let toml = toml::to_string_pretty(&lockfile).unwrap();

        assert!(toml.contains("[metadata]"));
        assert!(toml.contains("generated_by = \"veld 0.1.0\""));
        assert!(toml.contains("profile_hash = \"test_profile_hash_123\""));
        assert!(toml.contains("[[package]]"));
        assert!(toml.contains("name = \"zlib\""));
        assert!(toml.contains("version = \"1.3.1\""));
        assert!(toml.contains("artifact_id"));
        assert!(toml.contains("required_by = [\"root\"]"));
    }

    // ============================================================================
    // Tests de Integridad y Validación
    // ============================================================================

    #[rstest]
    fn test_validate_profile_hash_match(sample_lockfile: Lockfile) {
        let result = sample_lockfile.validate_profile("test_profile_hash_123");
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_validate_profile_hash_mismatch(sample_lockfile: Lockfile) {
        let result = sample_lockfile.validate_profile("different_profile_hash");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Build profile changed")
        );
    }

    #[rstest]
    fn test_lockfile_with_content_hash() {
        let mut package = pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"]);
        package.content_hash = Some("abc123def456".to_string());

        let lockfile = lock(vec![package]);
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");

        lockfile.write(temp_file.path()).expect("Failed to write");
        let loaded = Lockfile::read(temp_file.path()).expect("Failed to read");

        assert_eq!(
            loaded.packages[0].content_hash,
            Some("abc123def456".to_string())
        );
    }

    // ============================================================================
    // Tests de Búsqueda y Consulta
    // ============================================================================

    #[rstest]
    #[case("curl", Some("7.88.0"))]
    #[case("zlib", Some("1.3.1"))]
    #[case("nonexistent", None)]
    fn test_find_package(#[case] name: &str, #[case] expected_version: Option<&str>) {
        let packages = vec![
            pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"]),
            pkg("curl", semver::Version::new(7, 88, 0), vec!["root"]),
        ];
        let lockfile = lock(packages);

        let found = lockfile.find_package(name);

        match expected_version {
            Some(version) => {
                assert!(found.is_some());
                assert_eq!(
                    found.unwrap().version,
                    semver::Version::parse(version).unwrap()
                );
            }
            None => assert!(found.is_none()),
        }
    }

    #[rstest]
    fn test_direct_dependencies() {
        let packages = vec![
            pkg("zlib", semver::Version::new(1, 3, 1), vec!["root"]),
            pkg("openssl", semver::Version::new(3, 1, 0), vec!["curl"]),
            pkg("curl", semver::Version::new(7, 88, 0), vec!["root"]),
        ];
        let lockfile = lock(packages);

        let direct_deps = lockfile.direct_dependencies();

        assert_eq!(direct_deps.len(), 2);

        let names: Vec<&str> = direct_deps.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"zlib"));
        assert!(names.contains(&"curl"));
        assert!(!names.contains(&"openssl"));
    }

    #[rstest]
    fn test_lockfile_multiple_required_by() {
        let mut package = pkg("zlib", semver::Version::new(1, 3, 1), vec![]);
        package.required_by = vec!["curl".to_string(), "openssl".to_string()];

        let lockfile = lock(vec![package]);

        let zlib = lockfile.find_package("zlib").unwrap();
        assert_eq!(zlib.required_by.len(), 2);
        assert!(zlib.required_by.contains(&"curl".to_string()));
        assert!(zlib.required_by.contains(&"openssl".to_string()));
    }
}
