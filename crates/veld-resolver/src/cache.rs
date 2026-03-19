use std::path::{Path, PathBuf};

use veld_error::{VeldResult, veld_error, ErrorCode, ErrorContext};

use crate::filesystem::{get_deps_base_dir, ensure_deps_dir};

/// Manages the local source cache for git dependencies.
///
/// The cache is located at `~/.local/shared/veld/deps/` by default,
/// but can be overridden with the `VELD_DEPS_DIR` environment variable.
///
/// The cache structure:
/// ```text
/// {base}/
///   {prefix}/              # first 2 chars of package name (lowercase)
///     {CapitalizedName}/   # package name with first char uppercase
///       {commit}/         # full 40-char SHA-1 hash
///         src/            # working tree (after checkout)
///         manifest.toml   # package metadata
/// ```
#[derive(Debug, Clone)]
pub struct CacheManager {
    base_dir: PathBuf,
}

impl CacheManager {
    /// Create a new CacheManager with the default base directory.
    ///
    /// The base is determined by `get_deps_base_dir()` which checks
    /// `VELD_DEPS_DIR` env var or falls back to `~/.local/shared/veld/deps`.
    pub fn new() -> Self {
        Self {
            base_dir: get_deps_base_dir(),
        }
    }

    /// Create a new CacheManager with a custom base directory.
    ///
    /// Useful for testing or custom configurations.
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Get the base directory path.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Ensure the cache base directory exists.
    ///
    /// Creates the directory (and all parent directories) if it doesn't exist.
    pub fn ensure_base(&self) -> VeldResult<()> {
        if !self.base_dir.exists() {
            std::fs::create_dir_all(&self.base_dir).map_err(|e| {
                veld_error!(
                    ErrorCode::FileError(
                        self.base_dir.to_string_lossy().into_owned(),
                        format!("Failed to create cache base directory: {}", e)
                    ),
                    ErrorContext::new()
                        .with_file(self.base_dir.clone())
                        .with_value("ensure_base")
                )
            })?;
        }
        Ok(())
    }

    /// Get the full path to a dependency's directory in the cache.
    ///
    /// Structure: `{base}/{prefix}/{CapitalizedName}/`
    pub fn dependency_path(&self, dep_name: &str) -> VeldResult<PathBuf> {
        let prefix = self.prefix_from_name(dep_name);
        let capitalized = self.capitalize_name(dep_name);
        Ok(self.base_dir.join(prefix).join(capitalized))
    }

    /// Get the full path to a specific commit of a dependency.
    ///
    /// Structure: `{base}/{prefix}/{CapitalizedName}/{commit}/`
    pub fn commit_path(&self, dep_name: &str, commit: &str) -> VeldResult<PathBuf> {
        let dep_dir = self.dependency_path(dep_name)?;
        Ok(dep_dir.join(commit))
    }

    /// Check if a dependency exists in the cache.
    ///
    /// Returns `true` if the package directory exists (may have zero or more commits).
    pub fn has_dependency(&self, dep_name: &str) -> VeldResult<bool> {
        let path = self.dependency_path(dep_name)?;
        Ok(path.exists())
    }

    /// Check if a specific commit of a dependency exists in the cache.
    ///
    /// Returns `true` if the commit directory exists.
    pub fn has_commit(&self, dep_name: &str, commit: &str) -> VeldResult<bool> {
        let path = self.commit_path(dep_name, commit)?;
        Ok(path.exists())
    }

    /// List all dependencies (package names) currently in the cache.
    ///
    /// Iterates through the prefix directories and returns capitalized package names.
    pub fn list_dependencies(&self) -> VeldResult<Vec<String>> {
        self.ensure_base()?;

        let mut deps = Vec::new();

        if !self.base_dir.exists() {
            return Ok(deps);
        }

        for entry in std::fs::read_dir(&self.base_dir).map_err(|e| {
            veld_error!(
                ErrorCode::FileError(
                    self.base_dir.to_string_lossy().into_owned(),
                    format!("Failed to read cache directory: {}", e)
                ),
                ErrorContext::new().with_file(self.base_dir.clone())
            )
        })? {
            let entry = entry.map_err(|e| {
                veld_error!(
                    ErrorCode::FileError(
                        self.base_dir.to_string_lossy().into_owned(),
                        format!("Failed to read cache entry: {}", e)
                    ),
                    ErrorContext::new()
                )
            })?;
            let prefix_dir = entry.path();
            if let Some(_prefix_name) = prefix_dir.file_name().and_then(|n| n.to_str()) {
                // Read capitalized package names from this prefix directory
                if let Ok(iter) = std::fs::read_dir(prefix_dir) {
                    for pkg_entry in iter.flatten() {
                        if let Some(pkg_name) = pkg_entry.file_name().to_str() {
                            deps.push(pkg_name.to_string());
                        }
                    }
                }
            }
        }

        // Sort for deterministic output
        deps.sort();
        Ok(deps)
    }

    /// List all commits of a given dependency in the cache.
    ///
    /// Returns commit SHA-1 strings (sorted).
    pub fn list_commits(&self, dep_name: &str) -> VeldResult<Vec<String>> {
        let dep_dir = self.dependency_path(dep_name)?;

        if !dep_dir.exists() {
            return Ok(Vec::new());
        }

        let mut commits = Vec::new();

        for entry in std::fs::read_dir(&dep_dir).map_err(|e| {
            veld_error!(
                ErrorCode::FileError(
                    dep_dir.to_string_lossy().into_owned(),
                    format!("Failed to read dependency directory: {}", e)
                ),
                ErrorContext::new().with_file(dep_dir.clone())
            )
        })? {
            let entry = entry.map_err(|e| {
                veld_error!(
                    ErrorCode::FileError(
                        dep_dir.to_string_lossy().into_owned(),
                        format!("Failed to read commit entry: {}", e)
                    ),
                    ErrorContext::new()
                )
            })?;
            if let Some(commit_str) = entry.file_name().to_str() {
                // Validate it looks like a SHA-1 (40 hex chars) or at least a directory
                commits.push(commit_str.to_string());
            }
        }

        // Sort for deterministic output
        commits.sort();
        Ok(commits)
    }

    /// Get the path to the src directory of a specific commit.
    ///
    /// This is where the actual source code files are after checkout.
    /// Returns `{base}/{prefix}/{CapitalizedName}/{commit}/src`
    pub fn src_dir(&self, dep_name: &str, commit: &str) -> VeldResult<PathBuf> {
        let commit_dir = self.commit_path(dep_name, commit)?;
        Ok(commit_dir.join("src"))
    }

    /// Get the path to the manifest.toml of a specific commit.
    ///
    /// Returns `{base}/{prefix}/{CapitalizedName}/{commit}/manifest.toml`
    pub fn manifest_path(&self, dep_name: &str, commit: &str) -> VeldResult<PathBuf> {
        let commit_dir = self.commit_path(dep_name, commit)?;
        Ok(commit_dir.join("manifest.toml"))
    }

    /// Compute the two-character prefix from a package name.
    ///
    /// This is the first 2 characters in lowercase.
    fn prefix_from_name(&self, name: &str) -> String {
        if name.len() >= 2 {
            name[0..2].to_lowercase()
        } else {
            let mut s = name.to_lowercase();
            s.push(s.chars().next().unwrap_or_default());
            s
        }
    }

    /// Capitalize a package name: first character uppercase, rest unchanged.
    ///
    /// Examples: "zlib" → "Zlib", "curl" → "Curl", "OpenSSL" → "OpenSSL"
    fn capitalize_name(&self, name: &str) -> String {
        let mut chars = name.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => {
                let capitalized_first = first.to_uppercase().collect::<String>();
                capitalized_first + chars.as_str()
            }
        }
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_new() {
        let manager = CacheManager::new();
        assert!(manager.base_dir().is_absolute());
    }

    #[test]
    fn test_with_base_dir() {
        let custom = PathBuf::from("/tmp/custom-cache");
        let manager = CacheManager::with_base_dir(custom.clone());
        assert_eq!(manager.base_dir(), &custom);
    }

    #[test]
    fn test_prefix_from_name() {
        let manager = CacheManager::new();
        assert_eq!(manager.prefix_from_name("zlib"), "zl");
        assert_eq!(manager.prefix_from_name("curl"), "cu");
        assert_eq!(manager.prefix_from_name("openssl"), "op");
        assert_eq!(manager.prefix_from_name("a"), "aa");
        assert_eq!(manager.prefix_from_name("ab"), "ab");
    }

    #[test]
    fn test_capitalize_name() {
        let manager = CacheManager::new();
        assert_eq!(manager.capitalize_name("zlib"), "Zlib");
        assert_eq!(manager.capitalize_name("curl"), "Curl");
        assert_eq!(manager.capitalize_name("OpenSSL"), "OpenSSL");
        assert_eq!(manager.capitalize_name("a"), "A");
        assert_eq!(manager.capitalize_name(""), "");
    }

    #[test]
    fn test_dependency_path() {
        let temp_base = PathBuf::from("/tmp/veld-test");
        let manager = CacheManager::with_base_dir(temp_base.clone());

        let path = manager.dependency_path("zlib").unwrap();
        assert_eq!(path, temp_base.join("zl").join("Zlib"));

        let path = manager.dependency_path("curl").unwrap();
        assert_eq!(path, temp_base.join("cu").join("Curl"));

        let path = manager.dependency_path("openssl").unwrap();
        assert_eq!(path, temp_base.join("op").join("Openssl"));
    }

    #[test]
    fn test_commit_path() {
        let temp_base = PathBuf::from("/tmp/veld-test");
        let manager = CacheManager::with_base_dir(temp_base.clone());

        let commit = "09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851";
        let path = manager.commit_path("zlib", commit).unwrap();
        assert_eq!(
            path,
            temp_base.join("zl").join("Zlib").join(commit)
        );
    }

    #[test]
    fn test_src_dir() {
        let temp_base = PathBuf::from("/tmp/veld-test");
        let manager = CacheManager::with_base_dir(temp_base.clone());

        let commit = "09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851";
        let path = manager.src_dir("zlib", commit).unwrap();
        assert_eq!(
            path,
            temp_base.join("zl").join("Zlib").join(commit).join("src")
        );
    }

    #[test]
    fn test_manifest_path() {
        let temp_base = PathBuf::from("/tmp/veld-test");
        let manager = CacheManager::with_base_dir(temp_base.clone());

        let commit = "09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851";
        let path = manager.manifest_path("zlib", commit).unwrap();
        assert_eq!(
            path,
            temp_base.join("zl").join("Zlib").join(commit).join("manifest.toml")
        );
    }

    #[test]
    fn test_has_dependency() {
        let temp_base = PathBuf::from("/tmp/veld-test-nonexistent");
        // Ensure it doesn't exist
        let _ = std::fs::remove_dir_all(&temp_base);

        let manager = CacheManager::with_base_dir(temp_base.clone());

        // Should return false for non-existent base
        assert!(!manager.has_dependency("zlib").unwrap_or(false));

        // Create base and a dependency
        std::fs::create_dir_all(&temp_base).unwrap();
        let dep_dir = temp_base.join("zl").join("Zlib");
        std::fs::create_dir_all(&dep_dir).unwrap();

        assert!(manager.has_dependency("zlib").unwrap());
    }

    #[test]
    fn test_has_commit() {
        let temp_base = PathBuf::from("/tmp/veld-test-commit");
        let _ = std::fs::remove_dir_all(&temp_base);

        let manager = CacheManager::with_base_dir(temp_base.clone());

        // Setup: create base, dep, and a commit
        std::fs::create_dir_all(&temp_base).unwrap();
        let dep_dir = temp_base.join("zl").join("Zlib");
        std::fs::create_dir_all(&dep_dir).unwrap();
        let commit_dir = dep_dir.join("09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851");
        std::fs::create_dir_all(&commit_dir).unwrap();

        assert!(manager.has_commit("zlib", "09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851").unwrap());
        assert!(!manager.has_commit("zlib", "deadbeef").unwrap());
    }

    #[test]
    fn test_list_dependencies() {
        let temp_base = PathBuf::from("/tmp/veld-test-list");
        let _ = std::fs::remove_dir_all(&temp_base);

        let manager = CacheManager::with_base_dir(temp_base.clone());
        std::fs::create_dir_all(&temp_base).unwrap();

        // Create some fake dependencies
        let dep1 = temp_base.join("zl").join("Zlib");
        let dep2 = temp_base.join("cu").join("Curl");
        std::fs::create_dir_all(&dep1).unwrap();
        std::fs::create_dir_all(&dep2).unwrap();

        let deps = manager.list_dependencies().unwrap();
        assert_eq!(deps, vec!["Curl", "Zlib"]); // sorted alphabetically
    }

    #[test]
    fn test_list_commits() {
        let temp_base = PathBuf::from("/tmp/veld-test-commits");
        let _ = std::fs::remove_dir_all(&temp_base);

        let manager = CacheManager::with_base_dir(temp_base.clone());
        std::fs::create_dir_all(&temp_base).unwrap();

        let dep_dir = temp_base.join("zl").join("Zlib");
        std::fs::create_dir_all(&dep_dir).unwrap();

        // Create some commit directories
        std::fs::create_dir_all(dep_dir.join("09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851")).unwrap();
        std::fs::create_dir_all(dep_dir.join("b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1")).unwrap();

        let commits = manager.list_commits("zlib").unwrap();
        assert_eq!(commits.len(), 2);
        assert!(commits.contains(&"09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851".to_string()));
        assert!(commits.contains(&"b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1".to_string()));
    }
}
