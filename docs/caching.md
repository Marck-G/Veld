# Dependency Cache

Veld maintains a local cache of fetched git dependencies to avoid re-cloning on every install.

## Cache Location

The cache directory is determined by:

1. `VELD_DEPS_DIR` environment variable (if set).
2. `~/.local/shared/veld/deps` (default).

The `CacheManager` in `crates/veld-resolver/src/cache.rs` manages all cache operations.

## Directory Structure

```
~/.local/shared/veld/deps/
  fm/                          # 2-char prefix (first 2 chars of lowercase name)
    Fmt/                       # Capitalized package name
      abc1234567890abcdef.../  # Commit hash directory
        src/                   # Source files
  ga/
    Spdlog/
      def678901234567890.../
        src/
```

### Path Construction

The cache path is: `{base}/{prefix}/{CapitalizedName}/{commit}/src/`

Where:
- **`base`**: The cache root directory.
- **`prefix`**: First 2 characters of the package name (lowercase). Single-character names are padded (e.g., `"a"` becomes `"aa"`).
- **`CapitalizedName`**: Package name with the first character uppercased (e.g., `"fmt"` becomes `"Fmt"`).
- **`commit`**: The 40-character SHA-1 commit hash.

This structure distributes dependencies across subdirectories for filesystem performance and provides isolation per commit.

## `CacheManager`

### Construction

```rust
// Uses default cache directory
let cache = CacheManager::new()?;

// Custom base (useful for testing)
let cache = CacheManager::with_base_dir(PathBuf::from("/tmp/test-cache"));
```

### Methods

| Method | Description |
|--------|-------------|
| `ensure_base()` | Creates the cache base directory if it doesn't exist |
| `dependency_path(name)` | Returns `{base}/{prefix}/{CapitalizedName}/` |
| `commit_path(name, commit)` | Returns `{base}/{prefix}/{CapitalizedName}/{commit}/` |
| `has_dependency(name)` | Checks if the dependency directory exists |
| `has_commit(name, commit)` | Checks if the commit directory exists |
| `list_dependencies()` | Lists all cached dependency names (sorted) |
| `list_commits(name)` | Lists all cached commits for a dependency (sorted) |
| `src_dir(name, commit)` | Returns `{commit_dir}/src` |
| `manifest_path(name, commit)` | Returns `{commit_dir}/manifest.toml` |
| `prefix_from_name(name)` | Computes the 2-char prefix |
| `capitalize_name(name)` | Capitalizes the first character |

### Prefix Calculation

```rust
fn prefix_from_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.len() >= 2 {
        lower[..2].to_string()
    } else {
        // Pad single-char names
        format!("{:0<2}", lower)
    }
}
```

Examples:
- `"fmt"` -> `"fm"`
- `"spdlog"` -> `"sp"`
- `"a"` -> `"aa"`
- `"AB"` -> `"ab"`

### Name Capitalization

```rust
fn capitalize_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}
```

Examples:
- `"fmt"` -> `"Fmt"`
- `"spdlog"` -> `"Spdlog"`
- `"gtest"` -> `"Gtest"`

## Cache in the Resolution Pipeline

During dependency resolution:

1. The resolver calls `CacheManager::dependency_path(name)` to get the target directory.
2. `LibGit2Fetcher::fetch()` clones/fetches into this directory at the commit-specific subdirectory.
3. The `src_dir()` path is stored in the resolved package and written to the lockfile.
4. The CMake generator reads the lockfile and sets `VELD_<NAME>_SOURCE_DIR` to this path.

## Cache Cleanup

```bash
veld cache clean
```

This removes the entire cache directory. All git dependencies will be re-fetched on the next `veld install`.

## Filesystem Module

The `crates/veld-resolver/src/filesystem.rs` module provides standalone functions for cache path construction:

| Function | Description |
|----------|-------------|
| `get_deps_base_dir()` | Returns the cache base directory |
| `ensure_deps_dir()` | Creates the cache directory |
| `build_dependency_path(name)` | Constructs the dependency path |
| `build_commit_path(dep_dir, commit)` | Constructs the commit path |
| `dependency_exists(name)` | Checks if a dependency is cached |
| `commit_exists(name, commit)` | Checks if a specific commit is cached |
| `capitalize_name(name)` | Capitalizes the first character |

These functions mirror the `CacheManager` methods and are used in contexts where a full `CacheManager` instance is not needed.
