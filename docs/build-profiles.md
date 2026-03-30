# Build Profiles

Build profiles define how your C/C++ code is compiled. They map directly to CMake build types and compiler flags.

## Profile Structure

Each profile in `veld.toml` under `[profiles.<name>]` contains:

```toml
[profiles.default]
cxx_std = "cxx17"        # C/C++ standard
build = "debug"           # debug or release
optimization = "o0"       # optimization level
pic = true                # position-independent code
lto = false               # link-time optimization
sanitizers = []           # sanitizer list
target = "x86_64-linux-gnu"  # optional target triple
```

## C++ Standards

| Value | CMake Equivalent | Description |
|-------|------------------|-------------|
| `c99` | `99` | C99 standard |
| `c11` | `11` | C11 standard |
| `cxx11` | `11` | C++11 standard |
| `cxx14` | `14` | C++14 standard |
| `cxx17` | `17` | C++17 standard |
| `cxx20` | `20` | C++20 standard |

These are mapped to `CMAKE_CXX_STANDARD` (or `CMAKE_C_STANDARD` for C variants) in the generated `veld.cmake`.

## Build Types

| Value | CMake Type | Description |
|-------|------------|-------------|
| `debug` | `Debug` | Debug symbols, no optimization, `DEBUG` preprocessor define |
| `release` | `Release` | Full optimization, `NDEBUG` preprocessor define |

### Debug Mode Flags

```
-g -O0 -DDEBUG
```

### Release Mode Flags

```
<optimization_flag> -DNDEBUG
```

## Optimization Levels

| Value | Flag | Description |
|-------|------|-------------|
| `o0` | `-O0` | No optimization |
| `o1` | `-O1` | Basic optimization |
| `o2` | `-O2` | Standard optimization |
| `o3` | `-O3` | Aggressive optimization |
| `os` | `-Os` | Optimize for size |
| `oz` | `-Oz` | Aggressively optimize for size |

The optimization flag is applied in release mode. In debug mode, `-O0` is always used regardless of this setting.

## Position-Independent Code (PIC)

When `pic = true`:

```cmake
set(CMAKE_POSITION_INDEPENDENT_CODE ON)
```

This adds `-fPIC` to compiler flags. Required for shared libraries on most platforms.

## Link-Time Optimization (LTO)

When `lto = true`:

```cmake
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION ON)
```

Enables LTO/whole-program optimization. Requires compiler support.

## Sanitizers

Available sanitizers:

| Value | Flag | Description |
|-------|------|-------------|
| `address` | `-fsanitize=address` | Memory error detection (buffer overflow, use-after-free) |
| `undefined` | `-fsanitize=undefined` | Undefined behavior detection |
| `thread` | `-fsanitize=thread` | Data race detection |
| `memory` | `-fsanitize=memory` | Uninitialized memory reads (Clang only) |

When sanitizers are enabled, the following flags are added:

```
-fsanitize=<sanitizer1,sanitizer2> -fno-omit-frame-pointer
```

### Sanitizer Rules

1. **Debug only** -- Sanitizers can only be used when `build = "release"` is false. Using sanitizers in release mode is a validation error.
2. **No LTO** -- LTO and sanitizers are mutually exclusive. Enabling both is a validation error.
3. **Thread conflicts** -- The `thread` sanitizer conflicts with `address` and `memory`. Combining them is a validation error.

## Target Triple

The optional `target` field specifies a cross-compilation target:

```toml
[profiles.cross]
cxx_std = "cxx17"
build = "release"
optimization = "o2"
pic = true
lto = false
sanitizers = []
target = "aarch64-linux-gnu"
```

Format: `arch-os-abi` (3 dash-separated parts). This is validated but not yet fully used for cross-compilation toolchain selection.

## Using Profiles

The `default` profile is used unless otherwise specified:

```bash
veld install                    # uses "default" profile
veld install --profile release  # uses "release" profile
veld build --profile release --run
```

The `--release` flag on `veld build` overrides the build type to `release` regardless of the selected profile.

## Multiple Profiles Example

```toml
[profiles.default]
cxx_std = "cxx17"
build = "debug"
optimization = "o0"
pic = true
lto = false
sanitizers = ["address", "undefined"]

[profiles.release]
cxx_std = "cxx17"
build = "release"
optimization = "o3"
pic = true
lto = true
sanitizers = []

[profiles.minsize]
cxx_std = "cxx17"
build = "release"
optimization = "oz"
pic = true
lto = true
sanitizers = []
```
