# veld

A C/C++ package manager written in Rust that manages dependencies via git and path sources and generates CMake build files.

## Installation

```bash
cargo install --path crates/veld-cli
```

Or build from source:

```bash
git clone <repo-url>
cd veld
cargo build --release
# Binary at target/release/veld
```

## Quickstart

```bash
# 1. Create a new project
mkdir myproject && cd myproject
veld init

# 2. Add dependencies
veld add fmt --git https://github.com/fmtlib/fmt.git --tag 10.2.1
veld add mylib --path ../libs/mylib

# 3. Resolve dependencies and generate CMake files
veld install

# 4. Build
veld build --run
```

This creates `veld.toml`, resolves dependencies into `veld.lock`, and generates CMake files (`veld.cmake`, `veld_deps.cmake`, `CMakeLists.txt`).

## CLI Commands

### `veld init`

Create a new `veld.toml` manifest in the current directory.

```bash
veld init                      # Uses directory name as package name
veld init --name mylib         # Custom package name
veld init --name mylib --version 1.0.0
```

### `veld add`

Add a dependency to `veld.toml`.

**Git dependency:**

```bash
veld add fmt --git https://github.com/fmtlib/fmt.git --tag 10.2.1
veld add spdlog --git https://github.com/gabime/spdlog.git --branch v1.x
veld add mylib --git https://github.com/user/mylib.git --commit a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2
```

**Path dependency:**

```bash
veld add mylib --path ../libs/mylib
```

**Dev dependency:**

```bash
veld add gtest --git https://github.com/google/googletest.git --tag v1.14.0 --dev
```

Options:

- `--git <url>` — Git repository URL (conflicts with `--path`)
- `--path <path>` — Local filesystem path (conflicts with `--git`)
- `--branch <name>` — Git branch (mutually exclusive with `--tag`, `--commit`)
- `--tag <name>` — Git tag (mutually exclusive with `--branch`, `--commit`)
- `--commit <hash>` — Full 40-character commit SHA (mutually exclusive with `--branch`, `--tag`)
- `--dev` — Add as a dev dependency

### `veld install`

Resolve dependencies, fetch sources, generate the lockfile (`veld.lock`), and produce CMake files.

```bash
veld install
veld install --force   # Re-resolve even if lockfile is up to date
```

Skips resolution if `veld.lock` is still in sync with the manifest hash. Pass `--force` to bypass this check.

### `veld build`

Generate CMake files and optionally run cmake configure + build.

```bash
veld build                          # Generate CMake files only
veld build --run                    # Generate + cmake configure + build
veld build --release --run          # Release build
veld build --run -j 8               # Parallel build with 8 jobs
veld build --build-dir out --run    # Custom build directory
```

Options:

- `--release` — Build in release mode (overrides profile build type)
- `--run` — Run `cmake -B <dir>` and `cmake --build <dir>` after generating files
- `--build-dir <name>` — Build directory name (default: `build`)
- `-j, --jobs <N>` — Number of parallel jobs for `cmake --build`

### `veld remove`

Remove a dependency from `veld.toml`.

```bash
veld remove fmt
```

### `veld tree`

Display the resolved dependency tree.

```bash
veld tree
# myapp v0.1.0
# ├── fmt v10.2.1 [abc12345]
# └── mylib v0.1.0 [path: ../libs/mylib]
```

Requires `veld.lock` to exist (run `veld install` first).

### `veld cache clean`

Remove the local veld cache directory.

```bash
veld cache clean
```

## `veld.toml` Manifest Reference

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

### `[package]`

| Field           | Type            | Required | Description             |
| --------------- | --------------- | -------- | ----------------------- |
| `name`          | string          | yes      | Package name            |
| `version`       | semver string   | yes      | Package version         |
| `authors`       | array of string | no       | Author list             |
| `license`       | string          | no       | SPDX license identifier |
| `description`   | string          | no       | Package description     |

### `[profiles.<name>]`

| Field          | Type              | Required | Description                        |
| -------------- | ----------------- | -------- | ---------------------------------- |
| `cxx_std`      | string            | yes      | C/C++ standard (see below)         |
| `build`        | string            | yes      | `debug` or `release`               |
| `optimization` | string            | yes      | Optimization level (see below)     |
| `pic`          | bool              | yes      | Position-independent code          |
| `lto`          | bool              | yes      | Link-time optimization             |
| `sanitizers`   | array of string   | yes      | Sanitizer list (debug only)        |
| `target`       | string            | no       | Target triple (`arch-os-abi`)      |

### `[dependencies.<name>]` / `[dev_dependencies.<name>]`

Each dependency has a `source` table and optional fields:

| Field      | Type   | Description                           |
| ---------- | ------ | ------------------------------------- |
| `optional` | bool   | Whether the dependency is optional    |
| `features` | array  | Feature flags (reserved for future)   |
| `source`   | table  | One of: git, path, or registry source |

**Git source:**

```toml
[dependencies.mydep.source]
repo = "https://github.com/user/repo.git"
branch = "main"        # optional, one of branch/tag/commit required
tag = "v1.0.0"         # optional
commit = "abc123..."   # optional, must be 40 hex chars
subdir = "sub/dir"     # optional, path inside repo
fetch_depth = 1        # optional, shallow clone depth
```

**Path source:**

```toml
[dependencies.mydep.source]
path = "../libs/mydep"
```

## Build Profiles

Veld supports build profiles that map directly to CMake build types and compiler flags.

### Debug (default)

```toml
[profiles.default]
cxx_std = "cxx17"
build = "debug"
optimization = "o0"
pic = true
lto = false
sanitizers = []
```

- No optimizations (`-O0`)
- Debug symbols included
- Sanitizers available

### Release

```toml
[profiles.release]
cxx_std = "cxx17"
build = "release"
optimization = "o3"
pic = true
lto = true
sanitizers = []
```

- Full optimizations (`-O3`)
- LTO enabled
- No sanitizers (enforced by validation)

### C++ Standards

`c99`, `c11`, `cxx11`, `cxx14`, `cxx17`, `cxx20`

### Optimization Levels

`o0`, `o1`, `o2`, `o3`, `os` (size), `oz` (aggressive size)

### Sanitizers

`address`, `undefined`, `thread`, `memory`

Rules enforced at validation:

- Sanitizers are only allowed in debug profiles
- LTO and sanitizers are mutually exclusive
- Thread sanitizer conflicts with address and memory sanitizers

## How It Works

1. **Manifest parsing** — Reads `veld.toml` and validates the schema (field types, git URL formats, path existence, profile consistency)
2. **Dependency resolution** — Resolves the full dependency graph, detecting cycles and conflicts
3. **Source fetching** — Git dependencies are cloned to a local cache; path dependencies are used in place
4. **Lockfile generation** — Writes `veld.lock` with pinned versions, commit hashes, and a manifest hash for staleness detection
5. **CMake generation** — Produces three files:
   - `veld.cmake` — Sets compiler flags, C++ standard, build type, sanitizers, LTO, and PIC from the active profile
   - `veld_deps.cmake` — Adds each dependency via `add_subdirectory()` with its source path
   - `CMakeLists.txt` — Project-level CMakeLists (skipped if already present)

To build without `veld build --run`, use the generated CMake files directly:

```bash
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build
```

## Known Limitations

- **Registry support is planned** — Currently only git and path sources are supported. A centralized registry for published C/C++ packages is not yet available.
- Git dependencies require exactly one revision specifier (branch, tag, or commit)
- Commit hashes must be the full 40-character SHA-1
