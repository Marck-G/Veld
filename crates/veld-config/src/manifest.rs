use std::path::Path;

use veld_error::{ErrorCode, ErrorContext, VeldError, VeldResult};

use crate::{constants::FILE_BUILD_NAME, Manifest};

/// Load and parse a `veld.toml` manifest from disk.
///
/// Reads the file, parses the TOML content, validates the manifest,
/// and returns the parsed `Manifest` struct.
///
/// # Arguments
/// * `dir` - The directory containing the `veld.toml` file.
///
/// # Errors
/// Returns `VeldError` if:
/// - The manifest file doesn't exist
/// - The file cannot be read
/// - The TOML content is invalid
/// - The manifest fails validation
pub fn load_manifest(dir: &Path) -> VeldResult<Manifest> {
    let manifest_path = dir.join(FILE_BUILD_NAME);

    if !manifest_path.exists() {
        return Err(VeldError::new(
            ErrorCode::ManifestNotFound(manifest_path.clone()),
            ErrorContext::new().with_file(manifest_path),
        ));
    }

    let content = std::fs::read_to_string(&manifest_path).map_err(|e| {
        VeldError::new(
            ErrorCode::ManifestReadError(format!(
                "Failed to read '{}': {}",
                manifest_path.display(),
                e
            )),
            ErrorContext::new().with_file(manifest_path.clone()),
        )
    })?;

    let manifest: Manifest = toml::from_str(&content).map_err(|e| {
        VeldError::new(
            ErrorCode::ManifestParseError(e.to_string()),
            ErrorContext::new().with_file(manifest_path.clone()),
        )
    })?;

    // Run validation
    if let Err(errors) = manifest.validate() {
        // Return the first validation error
        return Err(errors.into_iter().next().unwrap());
    }

    Ok(manifest)
}

/// Compute a blake3 hash of the manifest for lockfile invalidation.
///
/// This hash is deterministic: it serializes the manifest to TOML
/// (which produces deterministic output due to BTreeMap ordering)
/// and hashes the result.
pub fn hash_manifest(manifest: &Manifest) -> VeldResult<String> {
    let toml_str = toml::to_string(manifest).map_err(|e| {
        VeldError::new(
            ErrorCode::ManifestParseError(format!(
                "Failed to serialize manifest for hashing: {}",
                e
            )),
            ErrorContext::new(),
        )
    })?;

    Ok(blake3::hash(toml_str.as_bytes()).to_hex().to_string())
}

/// Compute a hash of the build profile configuration.
///
/// Used to detect when build profiles change, which should invalidate the lockfile.
pub fn hash_profile(manifest: &Manifest, profile_name: &str) -> VeldResult<String> {
    let profile = manifest.profiles.get(profile_name).ok_or_else(|| {
        VeldError::new(
            ErrorCode::ProfileNotFound(profile_name.to_string()),
            ErrorContext::new(),
        )
    })?;

    let toml_str = toml::to_string(profile).map_err(|e| {
        VeldError::new(
            ErrorCode::ManifestParseError(format!(
                "Failed to serialize profile for hashing: {}",
                e
            )),
            ErrorContext::new(),
        )
    })?;

    Ok(blake3::hash(toml_str.as_bytes()).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use tempfile::tempdir;

    use crate::structs::*;

    fn make_test_manifest() -> Manifest {
        Manifest {
            package: PackageMetadata {
                name: "myproject".to_string(),
                version: semver::Version::new(1, 0, 0),
                authors: Some(vec!["Test Author".to_string()]),
                license: Some("MIT".to_string()),
                description: Some("A test project".to_string()),
            },
            profiles: {
                let mut m = BTreeMap::new();
                m.insert(
                    "default".to_string(),
                    ProfileConfig {
                        cxx_std: CxxStandard::Cxx17,
                        build: BuildType::Debug,
                        optimization: OptimizationLevel::O0,
                        pic: true,
                        lto: false,
                        sanitizers: BTreeSet::new(),
                        target: None,
                    },
                );
                m
            },
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
        }
    }

    #[test]
    fn test_load_manifest_success() {
        let dir = tempdir().unwrap();
        let manifest = make_test_manifest();
        let content = toml::to_string_pretty(&manifest).unwrap();
        fs::write(dir.path().join("veld.toml"), content).unwrap();

        let loaded = load_manifest(dir.path()).unwrap();
        assert_eq!(loaded.package.name, "myproject");
        assert_eq!(loaded.package.version, semver::Version::new(1, 0, 0));
    }

    #[test]
    fn test_load_manifest_not_found() {
        let dir = tempdir().unwrap();
        let result = load_manifest(dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("veld.toml"));
    }

    #[test]
    fn test_hash_manifest_deterministic() {
        let manifest = make_test_manifest();
        let hash1 = hash_manifest(&manifest).unwrap();
        let hash2 = hash_manifest(&manifest).unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // blake3 hex hash length
    }

    #[test]
    fn test_hash_manifest_different_for_different_manifests() {
        let mut manifest1 = make_test_manifest();
        let manifest2 = make_test_manifest();
        manifest1.package.name = "other".to_string();

        let hash1 = hash_manifest(&manifest1).unwrap();
        let hash2 = hash_manifest(&manifest2).unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_hash_profile() {
        let manifest = make_test_manifest();
        let hash = hash_profile(&manifest, "default").unwrap();
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn test_hash_profile_not_found() {
        let manifest = make_test_manifest();
        let result = hash_profile(&manifest, "nonexistent");
        assert!(result.is_err());
    }
}
