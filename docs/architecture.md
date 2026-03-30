# Architecture

Veld is organized as a Rust workspace with 7 crates, each responsible for a specific domain. This document describes each crate's purpose, key types, and how they interact.

## Workspace

**Root**: `Cargo.toml`

```toml
[workspace]
resolver = "3"
members = ["crates/*"]
```

Shared dependencies are declared at the workspace level and referenced by individual crates: `serde`, `toml`, `semver`, `owo-colors`, `reqwest`, `git2`, `blake3`, `chrono`, `petgraph`, `clap`, `tempfile`, `rstest`.

## Dependency Graph Between Crates

```
veld-cli
  ├── veld-config
  │     ├── veld-error
  │     └── (serde, toml, semver, blake3)
  ├── veld-resolver
  │     ├── veld-config
  │     ├── veld-error
  │     ├── veld-fetcher-git
  │     └── (petgraph, semver, dirs)
  ├── veld-lock
  │     ├── veld-error
  │     └── (serde, toml, semver, blake3, chrono, petgraph)
  ├── veld-generator
  │     ├── veld-error
  │     └── (semver)
  ├── veld-error
  │     └── (owo-colors)
  └── (clap, toml, semver, owo-colors, petgraph)

veld-fetcher-git
  ├── veld-error
  └── (git2, reqwest)
```

---

## `veld-error` (Foundation)

**Path**: `crates/veld-error/`  
**Lines**: ~586  
**Role**: Structured error types with diagnostic codes, context, and colorized display.

### Key Types

| Type | Description |
|------|-------------|
| `VeldError` | Error struct with `code: ErrorCode` and `context: ErrorContext` |
| `VeldResult<T>` | `Result<T, VeldError>` type alias |
| `ErrorCode` | Enum with 30+ variants (E001-E050), each implementing `Diagnostic` |
| `ErrorContext` | Builder for file path, line/column, source line, and value |
| `Diagnostic` trait | `code()`, `message()`, `hint()` for each error variant |

### Macros

- `veld_error!(code)` / `veld_error!(code, context)` -- Create a `VeldError`.
- `map_veld_error!(expr, code)` -- Map `Result` errors to `VeldError`.
- `map_veld_errin!(expr, fn)` -- Map with custom function.

### Design

Every error has a unique code (E001-E050) for programmatic reference and a human-readable message with optional hints. The `Display` impl produces colorized output resembling Rust's compiler diagnostics, including source location and caret indicators.

---

## `veld-config` (Manifest)

**Path**: `crates/veld-config/`  
**Lines**: ~570  
**Role**: Defines the `veld.toml` manifest format, data structures, parsing, validation, and config generation.

### Key Types

| Type | Description |
|------|-------------|
| `Manifest` | Top-level: `package`, `profiles`, `dependencies`, `dev_dependencies` |
| `PackageMetadata` | Name, version, authors, license, description |
| `ProfileConfig` | C++ standard, build type, optimization, PIC, LTO, sanitizers, target |
| `DependencySpec` | Source (git/path/registry), optional flag, features |
| `DependencySource` | Untagged enum: `Git(GitSource)`, `Path(PathSource)`, `Registry` |
| `GitSource` | Repo URL, branch/tag/commit, subdir, fetch_depth |
| `PathSource` | Filesystem path |
| `CxxStandard` | Enum: C99, C11, Cxx11, Cxx14, Cxx17, Cxx20 |
| `BuildType` | Enum: Debug, Release |
| `OptimizationLevel` | Enum: O0-O3, Os, Oz |
| `Sanitizer` | Enum: Address, Undefined, Thread, Memory |

### Key Functions

| Function | Description |
|----------|-------------|
| `load_manifest(dir)` | Read and parse `veld.toml` from a directory |
| `hash_manifest(manifest)` | Deterministic blake3 hash of the manifest |
| `hash_profile(manifest, name)` | Hash a specific profile |
| `config::generate(path, metadata)` | Write a new `veld.toml` |

### Validation

Each type implements `validate()` with specific checks:
- **GitSource**: URL scheme, exactly one revision, commit format, fetch_depth >= 1
- **PathSource**: Non-empty, path exists
- **ProfileConfig**: Target triple format, sanitizer rules (debug only, no LTO, thread conflicts)
- **Manifest**: Runs all sub-validations, collects errors

### Modules

| Module | Contents |
|--------|----------|
| `structs` | All data structures with serde derives |
| `manifest` | Load, hash functions |
| `validations` | Validation logic for each type |
| `constants` | File names, version |
| `tools/generate_config` | Config file generation |

---

## `veld-fetcher-git` (Git Operations)

**Path**: `crates/veld-fetcher-git/`  
**Lines**: ~204  
**Role**: Clone, fetch, and checkout git repositories via libgit2.

### Key Types

| Type | Description |
|------|-------------|
| `GitFetcher` trait | `fetch(url, source_ref, dest) -> FetchedSource` |
| `LibGit2Fetcher` | Implementation using `git2` crate |
| `SourceRef` | Enum: `Tag(String)`, `Branch(String)`, `Commit(String)` |
| `FetchedSource` | Result: `path`, `commit`, `symbolic_ref` |

### Key Functions

| Function | Description |
|----------|-------------|
| `repo_exists(url)` | HTTP HEAD check for repository existence |

### Pipeline

1. `open_or_clone()` -- Open existing repo (fetch remotes) or clone fresh.
2. `resolve_ref()` -- Find the commit OID for the requested tag/branch/commit.
3. `checkout()` -- Force checkout tree, detach HEAD.

---

## `veld-resolver` (Dependency Resolution)

**Path**: `crates/veld-resolver/`  
**Lines**: ~1099  
**Role**: Build the dependency graph, manage the cache, resolve transitive dependencies.

### Submodules

| Module | Lines | Purpose |
|--------|-------|---------|
| `graph` | 366 | `DependencyGraph` (petgraph DAG) |
| `provider` | 448 | `DependencyResolver` with first-wins strategy |
| `cache` | 411 | `CacheManager` for git clone storage |
| `filesystem` | 159 | Standalone cache path functions |

### Key Types

| Type | Description |
|------|-------------|
| `DependencyGraph` | Wraps `DiGraph<ResolvedPackage, ()>` with name index |
| `DependencyResolver` | Orchestrates resolution with cache, fetcher, visited set |
| `ResolutionResult` | Graph + list of conflicts |
| `DependencyConflict` | Package name, existing ref, conflicting ref |
| `ResolvedPackage` | Name, version, repo, commit, artifact_id, src_path |
| `CacheManager` | Manages `{base}/{prefix}/{Name}/{commit}/src/` paths |

### Resolution Algorithm

1. Iterate dependencies + dev_dependencies.
2. For each: dispatch to git/path/registry resolver.
3. Git: fetch to cache, load manifest, recurse.
4. Path: canonicalize, load manifest, recurse.
5. Build graph with edges from dependency to dependent.
6. Detect cycles via topological sort.
7. Return graph + conflicts.

### First-Wins Strategy

Duplicate package names: the first encountered version wins. Subsequent conflicts are logged as warnings.

---

## `veld-lock` (Lockfile)

**Path**: `crates/veld-lock/`  
**Lines**: ~556  
**Role**: Lockfile data structures, serialization, integrity checksums, staleness detection.

### Key Types

| Type | Description |
|------|-------------|
| `Lockfile` | Top-level: metadata, profile_hash, packages, checksum |
| `LockMetadata` | generated_by, generated_at, manifest_hash |
| `LockedPackage` | name, version, repository, commit, artifact_id, required_by, content_hash, path |
| `ResolvedPackage` | Subset used by the resolver graph |

### Key Operations

| Method | Description |
|--------|-------------|
| `Lockfile::new()` | Create from sorted packages with metadata |
| `Lockfile::from_resolved_graph()` | Convert petgraph DAG to lockfile |
| `Lockfile::write()` | Serialize with blake3 checksum |
| `Lockfile::read()` | Parse and verify checksum |
| `Lockfile::validate_profile()` | Compare profile hash |
| `Lockfile::direct_dependencies()` | Filter by `required_by == ["root"]` |

### Integrity

Write: serialize without checksum -> blake3 hash -> re-serialize with checksum.  
Read: parse -> extract checksum -> re-serialize -> verify hash.

---

## `veld-generator` (CMake Generation)

**Path**: `crates/veld-generator/`  
**Lines**: ~730  
**Role**: Generate CMake build files from resolved dependencies and profile config.

### Key Types

| Type | Description |
|------|-------------|
| `CMakeDependency` | name, src_path, version, required_by |
| `CMakeConfig` | cxx_standard, build_type, optimization_flag, pic, lto, sanitizers |
| `CMakeProjectConfig` | project_name, project_version, description, cxx_standard, languages |

### Generation Functions

| Function | Output File | Description |
|----------|-------------|-------------|
| `generate_cmake()` | `veld_deps.cmake` | Compiler flags, per-dep variables, import targets |
| `generate_build_helper()` | `veld-build.cmake` | Helper functions for linking |
| `render_cmakelists()` | `CMakeLists.txt` | Project-level CMake (skip if exists) |

### Generated Targets

| Target | Type | Description |
|--------|------|-------------|
| `veld::<name>` | INTERFACE IMPORTED | Per-dependency import target |
| `veld_deps` | INTERFACE | Aggregate target linking all deps |

---

## `veld-cli` (Binary)

**Path**: `crates/veld-cli/`  
**Lines**: ~1028  
**Role**: CLI argument parsing (clap derive), command dispatch, orchestration.

### Commands

| Command | Description |
|---------|-------------|
| `init` | Create `veld.toml` |
| `add` | Add dependency to manifest |
| `remove` | Remove dependency from manifest |
| `install` | Resolve + lock + generate CMake |
| `build` | Generate CMake + optionally run cmake |
| `tree` | Display dependency tree |
| `cache clean` | Remove cache directory |

### Orchestration

The `install` command:
1. Check lockfile staleness (manifest hash).
2. Create `DependencyResolver`, call `resolve(manifest)`.
3. Check for cycles.
4. Build `Lockfile` from graph.
5. Write lockfile.
6. Generate CMake files.

The `build` command:
1. Load manifest + profile.
2. Generate CMake files.
3. Optionally run `cmake -B` and `cmake --build`.

### Key Helper Functions

| Function | Description |
|----------|-------------|
| `build_lockfile()` | Convert `DependencyGraph` to `Lockfile` |
| `generate_cmake_from_graph()` | Read lockfile, determine source paths, call generator |
| `cxx_standard_to_string()` | Map `CxxStandard` to CMake string |
| `cmd_tree()` + `print_transitive_deps()` | Recursive tree display |

---

## Testing

| Location | Framework | Description |
|----------|-----------|-------------|
| `crates/veld-config/src/test.rs` | rstest | Manifest parsing tests |
| `crates/veld-lock/src/lib.rs` | rstest | Lockfile roundtrip, checksum, sorting tests |
| `crates/veld-resolver/src/graph.rs` | rstest | Graph operations, cycle detection |
| `crates/veld-resolver/src/provider.rs` | rstest | Source ref conversion |
| `crates/veld-resolver/src/cache.rs` | rstest | Path construction, cache operations |
| `crates/veld-resolver/src/filesystem.rs` | rstest | Path building functions |
| `crates/veld-generator/src/lib.rs` | rstest | CMake rendering, file generation |
| `crates/veld-error/src/lib.rs` | rstest | Error formatting and display |
| `tests/cli_integration.rs` | Command | End-to-end CLI tests |
