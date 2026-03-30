# Error Handling

Veld provides a structured error system with diagnostic codes, contextual information, and colorized output.

## Architecture

```
crates/veld-error/
  src/
    lib.rs            # VeldError struct, Display impl, display_errors()
    errorcodes.rs     # ErrorCode enum, Diagnostic trait
    error_context.rs  # ErrorContext builder
    macros.rs         # veld_error!, map_veld_error! macros
```

## `VeldError`

The core error type:

```rust
pub struct VeldError {
    pub code: ErrorCode,
    pub context: ErrorContext,
}
```

### Display Format

Errors are rendered with colorized output:

```
error[E004]: missing required field
  --> veld.toml:1:1
   |
 1 | [package]
   | ^^^^^^^^^ field 'name' is required in [package]
   |
   = hint: add 'name = "my-package"' to the [package] section
```

Components:
- **Error code**: `error[E###]` with the error message.
- **Location**: File path, line, and column if available.
- **Source line**: The offending line from the source file with a caret indicator.
- **Hint**: Optional suggestion for fixing the error.

### `display_errors()`

A batch display function for showing multiple errors:

```
+---------------------------+
| veld found 3 error(s)     |
+---------------------------+

error[E001]: invalid version string
...

.......

error[E004]: missing required field
...

.......

error[E025]: path not found
...
```

## `ErrorCode`

Every error has a unique code for programmatic reference:

### Manifest Errors

| Code | Name | Description |
|------|------|-------------|
| E001 | `InvalidVersion` | Invalid semver version string |
| E002 | `InvalidPackageName` | Package name is empty |
| E003 | `InvalidDependencyVersion` | Invalid dependency version constraint |
| E004 | `MissingField` | Required field is missing |
| E005 | `InvalidCxxStandard` | Unknown C++ standard value |
| E006 | `InvalidBuildType` | Unknown build type |
| E007 | `InvalidOptLevel` | Unknown optimization level |
| E008 | `InvalidSanitizer` | Unknown sanitizer name |

### Profile Errors

| Code | Name | Description |
|------|------|-------------|
| E009 | `ProfileNotFound` | Referenced profile does not exist |
| E010 | `InvalidTargetTriple` | Target triple must have 3 parts |

### I/O Errors

| Code | Name | Description |
|------|------|-------------|
| E011 | `ManifestNotFound` | `veld.toml` not found in directory |
| E012 | `ManifestReadError` | Failed to read manifest file |
| E013 | `ManifestParseError` | Failed to parse manifest TOML |

### Git Source Errors

| Code | Name | Description |
|------|------|-------------|
| E014 | `GitMissingRev` | No branch, tag, or commit specified |
| E015 | `GitConflictingRevs` | Multiple revision specifiers provided |
| E016 | `GitEmptyUrl` | Git URL is empty |
| E017 | `GitInvalidUrl` | Git URL has invalid scheme |
| E018 | `GitInvalidCommit` | Commit hash is not 40 hex characters |
| E019 | `GitInvalidFetchDepth` | Fetch depth must be >= 1 |

### Path Source Errors

| Code | Name | Description |
|------|------|-------------|
| E024 | `PathEmpty` | Path is empty |
| E025 | `PathNotFound` | Path does not exist on filesystem |

### File Errors

| Code | Name | Description |
|------|------|-------------|
| E026 | `FileError` | Generic file operation error |

### Profile Validation Errors

| Code | Name | Description |
|------|------|-------------|
| E020 | `SanitizersInRelease` | Sanitizers cannot be used in release mode |
| E021 | `LtoWithSanitizers` | LTO and sanitizers are incompatible |
| E022 | `ConflictingSanitizers` | Thread sanitizer conflicts with address/memory |

### Git Runtime Errors

| Code | Name | Description |
|------|------|-------------|
| E030 | `GitCloneFailed` | Git clone operation failed |
| E031 | `GitRefNotFound` | Git reference (tag/branch/commit) not found |
| E032 | `GitCheckoutFailed` | Git checkout operation failed |
| E033 | `GitFetchFailed` | Git fetch operation failed |
| E034 | `GitOpenFailed` | Failed to open existing git repository |
| E035 | `GitPeelingFailed` | Failed to peel reference to commit |

### Lockfile Errors

| Code | Name | Description |
|------|------|-------------|
| E040 | `LockfileNotFound` | `veld.lock` not found |
| E041 | `LockfileReadError` | Failed to read lockfile |
| E042 | `LockfileParseError` | Failed to parse lockfile TOML |
| E043 | `LockfileCheckFailed` | Lockfile staleness check failed |
| E044 | `LockfileCorrupted` | Lockfile integrity checksum mismatch |

### Build Errors

| Code | Name | Description |
|------|------|-------------|
| E050 | `BuildError` | Build process failed |

## `Diagnostic` Trait

Each `ErrorCode` implements the `Diagnostic` trait:

```rust
pub trait Diagnostic {
    fn code(&self) -> &'static str;       // e.g., "E004"
    fn message(&self) -> String;           // e.g., "missing required field"
    fn hint(&self) -> Option<String>;      // e.g., "add 'name = ...' to [package]"
}
```

The `hint()` method provides actionable suggestions for fixing the error.

## `ErrorContext`

Provides source location and value context:

```rust
pub struct ErrorContext {
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub raw_value: Option<String>,
    pub source_line: Option<String>,
}
```

### Builder Methods

```rust
VeldError::new(ErrorCode::MissingField)
    .with_file(Path::new("veld.toml"))
    .with_location(1, 1)
    .with_source_line("[package]")
    .with_value("name")
```

| Method | Sets |
|--------|------|
| `with_file(path)` | `file` |
| `with_location(line, col)` | `line` and `column` |
| `with_value(value)` | `raw_value` |
| `with_source_line(line)` | `source_line` |

## Macros

### `veld_error!`

Creates a `VeldError` from an error code:

```rust
// Without context
let err = veld_error!(ErrorCode::GitEmptyUrl);

// With context
let ctx = ErrorContext::new().with_file(Path::new("veld.toml"));
let err = veld_error!(ErrorCode::GitEmptyUrl, ctx);
```

### `map_veld_error!`

Maps a `Result`'s error to a `VeldError`:

```rust
// Simple mapping
let value = map_veld_error!(some_operation(), ErrorCode::ManifestReadError);

// With context
let ctx = ErrorContext::new().with_file(path);
let value = map_veld_error!(some_operation(), ErrorCode::ManifestReadError, ctx);
```

### `map_veld_errin!`

Maps a `Result`'s error using a custom function:

```rust
let value = map_veld_errin!(some_operation(), |e| VeldError::new(ErrorCode::BuildError));
```

## `VeldResult<T>`

A convenience type alias:

```rust
pub type VeldResult<T> = Result<T, VeldError>;
```

Used throughout the codebase as the standard return type for fallible operations.
