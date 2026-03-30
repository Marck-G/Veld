# Veld Documentation

Veld is a C/C++ package manager written in Rust that manages dependencies via git and path sources and generates CMake build files.

## Table of Contents

### User Guides

- [Getting Started](getting-started.md) -- Installation, quickstart, and typical workflow
- [CLI Reference](cli-reference.md) -- Complete command reference for all `veld` commands
- [Manifest Format](manifest-format.md) -- `veld.toml` file structure and all fields
- [Build Profiles](build-profiles.md) -- Compiler flags, optimization, sanitizers, and C++ standards

### Internals

- [Overview](overview.md) -- Project overview, core concepts, and design decisions
- [Architecture](architecture.md) -- Crate-level breakdown, types, and dependency graph
- [Dependency Resolution](dependency-resolution.md) -- Resolution algorithm, graph construction, first-wins strategy
- [CMake Generation](cmake-generation.md) -- Generated files, variables, targets, and usage
- [Lockfile](lockfile.md) -- Lockfile format, integrity checksums, and staleness detection
- [Git Fetching](git-fetching.md) -- libgit2-based clone/fetch/checkout pipeline
- [Caching](caching.md) -- Local cache structure and management
- [Error Handling](error-handling.md) -- Error codes, diagnostics, and context system

## Quick Links

| Topic | Document |
|-------|----------|
| Install veld | [Getting Started](getting-started.md#installation) |
| Create a project | [CLI Reference: init](cli-reference.md#veld-init) |
| Add a dependency | [CLI Reference: add](cli-reference.md#veld-add) |
| Build a project | [CLI Reference: build](cli-reference.md#veld-build) |
| Configure profiles | [Build Profiles](build-profiles.md) |
| Understand the lockfile | [Lockfile](lockfile.md) |
| How resolution works | [Dependency Resolution](dependency-resolution.md) |
