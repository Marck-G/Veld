use std::path::PathBuf;

use veld_error::{ErrorCode, ErrorContext, VeldResult, veld_error};

/// Base directory for the local source cache.
///
/// Default: `~/.local/shared/veld/deps`
/// Can be overridden with `VELD_DEPS_DIR` environment variable.
pub const DEPS_BASE_DIR_DEFAULT: &str = ".local/shared/veld/deps";

/// Get the base directory for the local source cache.
///
/// Checks `VELD_DEPS_DIR` environment variable first, then falls back to default.
pub fn get_deps_base_dir() -> PathBuf {
    std::env::var("VELD_DEPS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.join(DEPS_BASE_DIR_DEFAULT)
        })
}

/// Ensure the dependencies base directory exists.
/// Creates it (and parent directories) if it doesn't exist.
pub fn ensure_deps_dir() -> VeldResult<PathBuf> {
    let base_dir = get_deps_base_dir();

    if !base_dir.exists() {
        let base_dir_clone = base_dir.clone();
        std::fs::create_dir_all(&base_dir).map_err(|e| {
            veld_error!(
                ErrorCode::FileError(
                    base_dir.to_string_lossy().into_owned(),
                    format!("Failed to create dependencies directory: {}", e)
                ),
                ErrorContext::new()
                    .with_file(base_dir_clone)
                    .with_value("create_deps_dir")
            )
        })?;
    }

    Ok(base_dir)
}

/// Build the path to a dependency's directory in the local store.
///
/// Structure: `{base}/{prefix}/{CapitalizedName}/`
///
/// - `prefix`: first 2 characters of package name (lowercase)
/// - `CapitalizedName`: package name with first char uppercase, rest as given
pub fn build_dependency_path(dep_name: &str) -> VeldResult<PathBuf> {
    let base_dir = get_deps_base_dir();

    // Get the two-character prefix (lowercase)
    let prefix = if dep_name.len() >= 2 {
        dep_name[0..2].to_lowercase()
    } else {
        // For single-char names (unlikely), pad with second char as same
        let mut s = dep_name.to_lowercase();
        s.push(s.chars().next().unwrap_or_default());
        s
    };

    // Capitalize: first char uppercase, rest unchanged
    let capitalized = capitalize_name(dep_name);

    let dep_dir = base_dir.join(prefix).join(capitalized);

    Ok(dep_dir)
}

/// Check if a dependency exists in the local source cache.
///
/// Returns `true` if the package directory exists (may or may not contain commits).
pub fn dependency_exists(dep_name: &str) -> VeldResult<bool> {
    let path = build_dependency_path(dep_name)?;
    Ok(path.exists())
}

/// Build the full path to a specific commit of a dependency.
///
/// Structure: `{base}/{prefix}/{CapitalizedName}/{commit}/`
///
/// This is where the actual source code for a specific version is stored.
pub fn build_commit_path(dep_name: &str, commit: &str) -> VeldResult<PathBuf> {
    let dep_dir = build_dependency_path(dep_name)?;
    Ok(dep_dir.join(commit))
}

/// Check if a specific commit of a dependency exists in the local cache.
///
/// Returns `true` if the commit directory exists.
pub fn commit_exists(dep_name: &str, commit: &str) -> VeldResult<bool> {
    let path = build_commit_path(dep_name, commit)?;
    Ok(path.exists())
}

/// Capitalize a package name: first character uppercase, rest unchanged.
///
/// Examples:
/// - "zlib" → "Zlib"
/// - "curl" → "Curl"
/// - "OpenSSL" → "OpenSSL" (already capitalized)
fn capitalize_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let capitalized_first = first.to_uppercase().collect::<String>();
            capitalized_first + chars.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_capitalize_name() {
        assert_eq!(capitalize_name("zlib"), "Zlib");
        assert_eq!(capitalize_name("curl"), "Curl");
        assert_eq!(capitalize_name("OpenSSL"), "OpenSSL");
        assert_eq!(capitalize_name("a"), "A");
        assert_eq!(capitalize_name(""), "");
    }

    #[test]
    fn test_build_dependency_path() {
        // Using a temp dir to avoid touching real home
        let temp_base = PathBuf::from("/tmp/veld-test");
        unsafe { env::set_var("VELD_DEPS_DIR", temp_base.to_str().unwrap()) };

        let path = build_dependency_path("zlib").unwrap();
        assert_eq!(path, temp_base.join("zl").join("Zlib"));

        let path = build_dependency_path("curl").unwrap();
        assert_eq!(path, temp_base.join("cu").join("Curl"));

        let path = build_dependency_path("openssl").unwrap();
        assert_eq!(path, temp_base.join("op").join("Openssl"));

        unsafe { env::remove_var("VELD_DEPS_DIR") };
    }

    #[test]
    fn test_build_commit_path() {
        let temp_base = PathBuf::from("/tmp/veld-test");
        unsafe { env::set_var("VELD_DEPS_DIR", temp_base.to_str().unwrap()) };

        let commit = "09155eaa2f9270dc4ed1fa13e2b4b2613e6e4851";
        let path = build_commit_path("zlib", commit).unwrap();
        assert_eq!(path, temp_base.join("zl").join("Zlib").join(commit));

        unsafe { env::remove_var("VELD_DEPS_DIR") };
    }
}
