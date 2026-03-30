# Getting Started

## Prerequisites

- **Rust** (1.75+ recommended) -- for building veld from source
- **CMake** (3.15+) -- for building C/C++ projects managed by veld
- **A C/C++ compiler** -- GCC, Clang, or MSVC
- **Git** -- for fetching git-based dependencies

## Installation

### From Source

```bash
git clone <repo-url>
cd veld
cargo install --path crates/veld-cli
```

Or build without installing:

```bash
cargo build --release
# Binary at target/release/veld
```

### Verify Installation

```bash
veld --version
```

## Quickstart Tutorial

### 1. Create a New Project

```bash
mkdir myproject && cd myproject
veld init
```

This creates a `veld.toml` manifest:

```toml
[package]
name = "myproject"
version = "0.1.0"

[profiles.default]
cxx_std = "cxx17"
build = "debug"
optimization = "o0"
pic = true
lto = false
sanitizers = []
```

### 2. Add Dependencies

**Add a git dependency:**

```bash
veld add fmt --git https://github.com/fmtlib/fmt.git --tag 10.2.1
```

**Add a local path dependency:**

```bash
veld add mylib --path ../libs/mylib
```

**Add a dev dependency:**

```bash
veld add gtest --git https://github.com/google/googletest.git --tag v1.14.0 --dev
```

### 3. Resolve and Generate

```bash
veld install
```

This fetches all dependencies, resolves the dependency graph, writes `veld.lock`, and generates CMake files.

### 4. Build

```bash
veld build --run
```

This generates CMake files (if not already present), then runs `cmake -B build` and `cmake --build build`.

Or build manually:

```bash
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build
```

### 5. Inspect the Dependency Tree

```bash
veld tree
```

Output:

```
myproject v0.1.0
├── fmt v10.2.1 [abc12345]
└── mylib v0.1.0 [path: ../libs/mylib]
```

## Typical Workflow

```
veld init                    # Create manifest
veld add <dep> --git ...     # Add dependencies
veld install                 # Resolve + lock + generate CMake
veld build --run             # Build the project
# ... develop ...
veld add <new-dep> --path .. # Add more deps
veld install                 # Re-resolve
veld build --run             # Rebuild
```

## Project Layout After Setup

```
myproject/
  veld.toml           # Manifest (you edit this)
  veld.lock           # Lockfile (auto-generated)
  veld.cmake          # Compiler flags (auto-generated)
  veld_deps.cmake     # Dependency targets (auto-generated)
  veld-build.cmake    # Build helpers (auto-generated)
  CMakeLists.txt      # Project CMake (auto-generated, not overwritten)
  src/                # Your source code
```
