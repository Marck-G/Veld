# Veld - Overview

## What is Veld?

Veld is a **C/C++ package manager** written in Rust. It manages project dependencies via **git repositories** and **local filesystem paths**, then generates **CMake build files** so your project can be built with standard CMake toolchains.

It draws inspiration from Rust's Cargo in its workflow (`init`, `add`, `install`, `build`) but targets the C/C++ ecosystem and CMake as its build backend.

## Core Concepts

| Concept | Description |
|---------|-------------|
| **Manifest** (`veld.toml`) | Declares your package metadata, build profiles, and dependencies |
| **Lockfile** (`veld.lock`) | Pins resolved dependency versions with integrity checksums |
| **Dependency** | A C/C++ library sourced from a git repo or local path |
| **Profile** | A set of compiler flags, optimization levels, and sanitizer config |
| **Cache** | Local clone storage for git dependencies at `~/.local/shared/veld/deps` |
| **CMake Generation** | Produces `veld.cmake`, `veld_deps.cmake`, `veld-build.cmake`, and `CMakeLists.txt` |

## How It Works (Pipeline)

```
veld.toml  -->  [Resolver]  -->  [Fetcher]  -->  [Lockfile]  -->  [Generator]  -->  CMake files
```

1. **Manifest Parsing** -- Reads `veld.toml`, deserializes TOML into typed structs, validates field constraints (URL formats, path existence, profile rules).
2. **Dependency Resolution** -- Builds a directed acyclic graph (DAG) of all dependencies (direct + transitive). Uses a **first-wins** strategy: if the same package name appears multiple times, the first encountered version is kept and subsequent conflicts are warned about.
3. **Source Fetching** -- Git dependencies are cloned/fetched into a local cache. Path dependencies are used in-place. Each fetched source yields a resolved package with a pinned commit hash.
4. **Lockfile Generation** -- Serializes the resolved graph into `veld.lock` with a blake3 integrity checksum and a manifest hash for staleness detection.
5. **CMake Generation** -- Produces three (or four) CMake files encoding compiler flags, dependency import targets, and a project-level `CMakeLists.txt`.

## Workspace Structure

Veld is organized as a Rust workspace with 7 crates:

```
veld/
  Cargo.toml              # Workspace root
  crates/
    veld-cli/             # Binary crate: CLI argument parsing and command dispatch
    veld-config/          # Manifest data structures, parsing, and validation
    veld-error/           # Structured error types with diagnostics
    veld-fetcher-git/     # Git clone/fetch operations via libgit2
    veld-generator/       # CMake file generation
    veld-lock/            # Lockfile data structures, serialization, integrity
    veld-resolver/        # Dependency graph building and resolution
  tests/
    cli_integration.rs    # End-to-end CLI tests
  examples/
    path-deps/            # Example project with path dependencies
```

## Generated Files

When you run `veld install` or `veld build`, the following files are produced:

| File | Purpose |
|------|---------|
| `veld.lock` | Pinned dependency versions with integrity checksum |
| `veld.cmake` | Compiler flags, C++ standard, build type, sanitizers, LTO, PIC settings |
| `veld_deps.cmake` | Per-dependency variables (`VELD_<NAME>_SOURCE_DIR`) and import targets (`veld::<name>`) |
| `veld-build.cmake` | Helper functions `veld_add_dependencies()` and `veld_include_dependencies()` |
| `CMakeLists.txt` | Project-level CMake file (only created if absent) |

## Design Decisions

### First-Wins Resolution

When the same dependency appears with different versions (e.g., two transitive deps require different versions of `fmt`), veld keeps the **first encountered** version. This is a deliberate trade-off:

- **Pro**: Simple, predictable, no complex version negotiation.
- **Con**: May silently use an incompatible version. A warning is emitted when a conflict is detected.

### Git-Centric Source Model

Dependencies are identified by git repository URL + a revision specifier (branch, tag, or commit). There is no centralized registry. This mirrors how most C/C++ libraries are distributed today (via GitHub, GitLab, etc.).

### CMake as Backend

Rather than implementing its own build system, veld generates standard CMake files. This means:

- Projects integrate naturally with existing CMake-based toolchains (IDEs, CI, cross-compilation).
- Users can inspect and customize the generated CMake files.
- The generated `CMakeLists.txt` is never overwritten if it already exists.

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `VELD_DEPS_DIR` | Override the dependency cache directory | `~/.local/shared/veld/deps` |
