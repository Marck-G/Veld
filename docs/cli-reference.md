# CLI Reference

Complete reference for all `veld` commands.

## Global Options

| Flag | Description |
|------|-------------|
| `--version` | Print version and exit |
| `--help` | Print help information |

---

## `veld init`

Create a new `veld.toml` manifest in the current directory.

### Usage

```bash
veld init
veld init --name <name>
veld init --name <name> --version <version>
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--name` | Current directory name | Package name |
| `--version` | `0.1.0` | Package version (semver) |

### Behavior

- Creates a `veld.toml` file with `[package]` and `[profiles.default]` sections.
- The default profile sets C++17, debug mode, no optimization, PIC enabled, LTO disabled.
- If `veld.toml` already exists, the command **does not overwrite** it (prints a message and exits).

### Example

```bash
mkdir mylib && cd mylib
veld init --name mylib --version 1.2.0
```

---

## `veld add`

Add a dependency to `veld.toml`.

### Usage

```bash
veld add <name> --git <url> [--branch <b> | --tag <t> | --commit <c>]
veld add <name> --path <path>
veld add <name> --git <url> --tag <t> --dev
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Dependency name (used as the key in `veld.toml`) |

### Options

| Flag | Description |
|------|-------------|
| `--git <url>` | Git repository URL (conflicts with `--path`) |
| `--path <path>` | Local filesystem path (conflicts with `--git`) |
| `--branch <name>` | Track a git branch (mutually exclusive with `--tag`, `--commit`) |
| `--tag <name>` | Pin to a git tag (mutually exclusive with `--branch`, `--commit`) |
| `--commit <hash>` | Pin to a 40-character commit SHA (mutually exclusive with `--branch`, `--tag`) |
| `--dev` | Add as a dev dependency (writes to `[dev_dependencies]`) |

### Validation

- Exactly one of `--git` or `--path` must be provided.
- For git dependencies, exactly one of `--branch`, `--tag`, or `--commit` must be provided.
- Git URLs must use `https://`, `http://`, `ssh://`, or `git@` scheme.
- Commit hashes must be exactly 40 hexadecimal characters.
- Path dependencies: the path must exist on the filesystem.

### Examples

```bash
# Tag-pinned git dependency
veld add fmt --git https://github.com/fmtlib/fmt.git --tag 10.2.1

# Branch-tracking git dependency
veld add spdlog --git https://github.com/gabime/spdlog.git --branch v1.x

# Commit-pinned git dependency
veld add mylib --git https://github.com/user/mylib.git --commit a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2

# Local path dependency
veld add mylib --path ../libs/mylib

# Dev dependency
veld add gtest --git https://github.com/google/googletest.git --tag v1.14.0 --dev
```

### Manifest Changes

For a git dependency with tag:

```toml
[dependencies.fmt]
optional = false
features = []

[dependencies.fmt.source]
repo = "https://github.com/fmtlib/fmt.git"
tag = "10.2.1"
```

For a path dependency:

```toml
[dependencies.mylib]
optional = false
features = []

[dependencies.mylib.source]
path = "../libs/mylib"
```

---

## `veld remove`

Remove a dependency from `veld.toml`.

### Usage

```bash
veld remove <name>
veld remove <name> --dev
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<name>` | Dependency name to remove |

### Options

| Flag | Description |
|------|-------------|
| `--dev` | Remove from `[dev_dependencies]` instead of `[dependencies]` |

### Behavior

- Removes the entire `[dependencies.<name>]` (or `[dev_dependencies.<name>]`) section from `veld.toml`.
- Does **not** regenerate the lockfile or CMake files. Run `veld install` afterward.

---

## `veld install`

Resolve dependencies, fetch sources, generate the lockfile, and produce CMake files.

### Usage

```bash
veld install
veld install --force
veld install --profile <name>
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--force` | `false` | Re-resolve even if the lockfile is up to date |
| `--profile` | `default` | Profile name from `veld.toml` |

### Pipeline

1. **Staleness check** -- Computes a blake3 hash of the manifest and compares it with the `manifest_hash` in `veld.lock`. If they match and `--force` is not set, resolution is skipped.
2. **Resolution** -- Builds the full dependency graph (direct + transitive) using a first-wins strategy.
3. **Cycle detection** -- Checks for circular dependencies. Returns an error if a cycle is found.
4. **Lockfile generation** -- Writes `veld.lock` with pinned versions, commit hashes, and a blake3 integrity checksum.
5. **CMake generation** -- Produces `veld.cmake`, `veld_deps.cmake`, `veld-build.cmake`, and `CMakeLists.txt` (if absent).

### Example

```bash
veld install
# Output: Resolved N dependencies, wrote veld.lock, generated CMake files

veld install --force
# Output: (re-resolves even if manifest unchanged)
```

---

## `veld build`

Generate CMake files and optionally run cmake configure + build.

### Usage

```bash
veld build
veld build --run
veld build --release --run
veld build --run -j 8
veld build --build-dir out --run
```

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `--release` | `false` | Build in release mode (overrides profile build type) |
| `--run` | `false` | Actually execute `cmake -B` and `cmake --build` |
| `--build-dir <name>` | `build` | Build directory name |
| `-j, --jobs <N>` | CMake default | Number of parallel build jobs |
| `--profile` | `default` | Profile name from `veld.toml` |

### Behavior

- Always generates CMake files (`veld.cmake`, `veld_deps.cmake`, `veld-build.cmake`, `CMakeLists.txt`).
- With `--run`:
  - Executes `cmake -B <build-dir> -DCMAKE_BUILD_TYPE=<Debug|Release>`.
  - Executes `cmake --build <build-dir> [-j <N>]`.
- Without `--run`, you must run cmake manually.

### Examples

```bash
# Just generate CMake files
veld build

# Debug build
veld build --run

# Release build
veld build --release --run

# Custom build directory with 8 parallel jobs
veld build --build-dir out -j 8 --run
```

---

## `veld tree`

Display the resolved dependency tree.

### Usage

```bash
veld tree
```

### Requirements

- `veld.lock` must exist. Run `veld install` first.

### Output Format

```
myapp v0.1.0
├── fmt v10.2.1 [abc12345]
├── spdlog v1.12.0 [def67890]
│   └── fmt v10.2.1 [abc12345]
└── mylib v0.1.0 [path: ../libs/mylib]
```

- Direct dependencies are listed first.
- Transitive dependencies are shown indented under their parent.
- `[abc12345]` shows the first 8 characters of the commit hash (git deps).
- `[path: ...]` shows the path (path deps).

---

## `veld cache clean`

Remove the local veld cache directory.

### Usage

```bash
veld cache clean
```

### Behavior

- Deletes the entire cache directory (default: `~/.local/shared/veld/deps`).
- All cached git clones will be re-fetched on the next `veld install`.
- The cache directory can be overridden with the `VELD_DEPS_DIR` environment variable.
