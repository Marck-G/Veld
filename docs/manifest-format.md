# Manifest Format (`veld.toml`)

The `veld.toml` file is the central configuration for a veld-managed project. It uses TOML format.

## Structure

```toml
[package]
name = "myapp"
version = "0.1.0"
authors = ["Alice <alice@example.com>"]
license = "MIT"
description = "My C++ application"

[profiles.default]
# ...

[dependencies.dep_name]
# ...

[dev_dependencies.test_dep]
# ...
```

---

## `[package]`

Metadata about the project.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | **yes** | Package identifier. Must be non-empty. Used as the CMake project name. |
| `version` | semver string | **yes** | Semantic version (e.g., `"0.1.0"`, `"1.2.3"`). |
| `authors` | array of string | no | List of authors (informational). |
| `license` | string | no | SPDX license identifier (informational). |
| `description` | string | no | Short description (informational). |

### Validation Rules

- `name` must be a non-empty string.
- `version` must be a valid semver string.

---

## `[profiles.<name>]`

Build configuration profiles. See [Build Profiles](build-profiles.md) for full details.

```toml
[profiles.default]
cxx_std = "cxx17"
build = "debug"
optimization = "o0"
pic = true
lto = false
sanitizers = []
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `cxx_std` | string | **yes** | C/C++ standard. One of: `c99`, `c11`, `cxx11`, `cxx14`, `cxx17`, `cxx20`. |
| `build` | string | **yes** | Build type. One of: `debug`, `release`. |
| `optimization` | string | **yes** | Optimization level. One of: `o0`, `o1`, `o2`, `o3`, `os`, `oz`. |
| `pic` | bool | **yes** | Enable position-independent code (`-fPIC`). |
| `lto` | bool | **yes** | Enable link-time optimization. |
| `sanitizers` | array of string | **yes** | List of sanitizers. One or more of: `address`, `undefined`, `thread`, `memory`. |
| `target` | string | no | Target triple in `arch-os-abi` format (e.g., `"x86_64-linux-gnu"`). |

### Validation Rules

- `target` must have exactly 3 dash-separated parts if specified.
- `sanitizers` can only be used in debug profiles.
- `lto` and `sanitizers` are mutually exclusive.
- `thread` sanitizer conflicts with `address` and `memory` sanitizers.

---

## `[dependencies.<name>]`

Direct runtime dependencies.

### Common Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `optional` | bool | `false` | Whether the dependency is optional (reserved for future use). |
| `features` | array of string | `[]` | Feature flags (reserved for future use). |
| `source` | table | **required** | The dependency source (git, path, or registry). |

### Git Source

```toml
[dependencies.fmt]
optional = false
features = []

[dependencies.fmt.source]
repo = "https://github.com/fmtlib/fmt.git"
tag = "10.2.1"           # exactly one of branch/tag/commit
# branch = "main"
# commit = "abc123..."
subdir = "src"           # optional: path inside the repo
fetch_depth = 1          # optional: shallow clone depth
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `repo` | string | **yes** | Git repository URL. Must use `https://`, `http://`, `ssh://`, or `git@` scheme. |
| `branch` | string | no | Track a branch. Mutually exclusive with `tag` and `commit`. |
| `tag` | string | no | Pin to a tag. Mutually exclusive with `branch` and `commit`. |
| `commit` | string | no | Pin to a 40-character SHA-1 commit hash. Mutually exclusive with `branch` and `tag`. |
| `subdir` | string | no | Subdirectory within the repo to use as the source root. |
| `fetch_depth` | uint | no | Shallow clone depth (must be >= 1 if specified). |

**Exactly one of `branch`, `tag`, or `commit` must be specified.**

### Path Source

```toml
[dependencies.mylib]
optional = false
features = []

[dependencies.mylib.source]
path = "../libs/mylib"
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `path` | string | **yes** | Relative or absolute path to the dependency. Must exist on the filesystem. |

### Registry Source (Planned)

```toml
[dependencies.mydep.source]
version = "^1.0.0"     # semver version requirement
```

Registry support is planned but not yet implemented. Dependencies with registry sources are skipped with a warning.

---

## `[dev_dependencies.<name>]`

Development-only dependencies (testing frameworks, benchmarks, etc.).

The format is identical to `[dependencies.<name>]`. Dev dependencies are resolved and included in the lockfile but are tracked separately.

```toml
[dev_dependencies.gtest]
optional = false
features = []

[dev_dependencies.gtest.source]
repo = "https://github.com/google/googletest.git"
tag = "v1.14.0"
```

---

## Complete Example

```toml
[package]
name = "myapp"
version = "0.1.0"
authors = ["Alice <alice@example.com>"]
license = "MIT"
description = "My C++ application"

[profiles.default]
cxx_std = "cxx17"
build = "debug"
optimization = "o0"
pic = true
lto = false
sanitizers = []

[profiles.release]
cxx_std = "cxx17"
build = "release"
optimization = "o3"
pic = true
lto = true
sanitizers = []

[dependencies.fmt]
optional = false
features = []

[dependencies.fmt.source]
repo = "https://github.com/fmtlib/fmt.git"
tag = "10.2.1"

[dependencies.mylib]
optional = false
features = []

[dependencies.mylib.source]
path = "../libs/mylib"

[dev_dependencies.gtest]
optional = false
features = []

[dev_dependencies.gtest.source]
repo = "https://github.com/google/googletest.git"
tag = "v1.14.0"
```
