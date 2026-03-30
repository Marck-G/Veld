# CMake Generation

Veld generates four CMake files that encode your project's build configuration and dependency information.

## Generated Files

### 1. `veld.cmake` -- Compiler Settings

Sets global CMake variables based on the active build profile.

**Contents:**

```cmake
# C++ Standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Build Type
set(CMAKE_BUILD_TYPE Debug)

# Compiler Flags
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0 -DDEBUG")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")

# PIC
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# LTO (if enabled)
set(CMAKE_INTERPROCEDURAL_OPTIMIZATION ON)

# Sanitizers (if enabled)
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fsanitize=address,undefined -fno-omit-frame-pointer")
set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -fsanitize=address,undefined -fno-omit-frame-pointer")
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fsanitize=address,undefined")
```

**Generation logic** (`crates/veld-generator/src/lib.rs`):

The `CMakeConfig` struct holds the profile settings:

| Field | Source | CMake Variable |
|-------|--------|----------------|
| `cxx_standard` | `profile.cxx_std` | `CMAKE_CXX_STANDARD` / `CMAKE_C_STANDARD` |
| `build_type` | `profile.build` | `CMAKE_BUILD_TYPE` |
| `optimization_flag` | `profile.optimization` | Used in `CMAKE_CXX_FLAGS_RELEASE` |
| `pic` | `profile.pic` | `CMAKE_POSITION_INDEPENDENT_CODE` |
| `lto` | `profile.lto` | `CMAKE_INTERPROCEDURAL_OPTIMIZATION` |
| `sanitizers` | `profile.sanitizers` | `-fsanitize=` flags |

The `generate_cmake()` function writes this file.

### 2. `veld_deps.cmake` -- Dependency Targets

Defines per-dependency variables and import targets.

**Contents:**

```cmake
if(NOT _VELD_DEPS_INCLUDED)
set(_VELD_DEPS_INCLUDED TRUE)

# (compiler settings from veld.cmake are inlined here)

# Per-dependency variables
set(VELD_FMT_SOURCE_DIR "/home/user/.local/shared/veld/deps/fm/Fmt/abc12345/src")
set(VELD_FMT_INCLUDE_DIR "/home/user/.local/shared/veld/deps/fm/Fmt/abc12345/src")
set(VELD_FMT_LIB_DIR "/home/user/.local/shared/veld/deps/fm/Fmt/abc12345/src")

# Validate source directories exist
if(NOT EXISTS "${VELD_FMT_SOURCE_DIR}")
  message(FATAL_ERROR "veld: dependency 'fmt' source not found at ${VELD_FMT_SOURCE_DIR}. Run 'veld install' first.")
endif()

# Per-dependency import targets
add_library(veld::fmt INTERFACE IMPORTED)
target_include_directories(veld::fmt INTERFACE "${VELD_FMT_INCLUDE_DIR}")

# Aggregate target
add_library(veld_deps INTERFACE)
target_link_libraries(veld_deps INTERFACE veld::fmt)
```

**Variables per dependency:**

| Variable | Description |
|----------|-------------|
| `VELD_<NAME>_SOURCE_DIR` | Absolute path to the dependency's source directory |
| `VELD_<NAME>_INCLUDE_DIR` | Include directory (same as source by default) |
| `VELD_<NAME>_LIB_DIR` | Library directory (same as source by default) |

**Targets per dependency:**

| Target | Type | Description |
|--------|------|-------------|
| `veld::<name>` | `INTERFACE IMPORTED` | Import target with include directories |
| `veld_deps` | `INTERFACE` | Aggregate target linking all dependency targets |

The dependency name is uppercased and non-alphanumeric characters are replaced with `_` for variable names.

**Generation logic:**

The `CMakeDependency` struct holds:

```rust
struct CMakeDependency {
    name: String,
    src_path: PathBuf,
    version: String,
    required_by: Vec<String>,
}
```

The `generate_cmake()` function iterates over all dependencies and emits the variables, existence checks, import targets, and the aggregate target.

### 3. `veld-build.cmake` -- Build Helpers

Provides convenience functions for linking dependencies to your targets.

**Contents:**

```cmake
include(${CMAKE_CURRENT_LIST_DIR}/veld_deps.cmake)

# Link all veld dependencies to a target
function(veld_add_dependencies TARGET)
  target_link_libraries(${TARGET} PRIVATE veld_deps)
endfunction()

# Add include directories for all veld dependencies
function(veld_include_dependencies TARGET)
  target_include_directories(${TARGET} PRIVATE
    ${VELD_FMT_INCLUDE_DIR}
    # ... other deps
  )
endfunction()
```

**Usage in your CMakeLists.txt:**

```cmake
include(veld-build.cmake)

add_executable(myapp src/main.cpp)
veld_add_dependencies(myapp)
# or
veld_include_dependencies(myapp)
```

**Generation logic:**

The `generate_build_helper()` function produces this file. It includes `veld_deps.cmake` and defines two functions:
- `veld_add_dependencies(TARGET)` -- links `veld_deps` to the target.
- `veld_include_dependencies(TARGET)` -- adds all dependency include directories.

### 4. `CMakeLists.txt` -- Project File

A project-level CMakeLists.txt that builds each dependency as a static library.

**Contents:**

```cmake
cmake_minimum_required(VERSION 3.15)
project(myapp VERSION 0.1.0 LANGUAGES C CXX DESCRIPTION "My application")

include(veld_deps.cmake)

# Dependency: fmt
file(GLOB FMT_SOURCES
  "${VELD_FMT_SOURCE_DIR}/*.c"
  "${VELD_FMT_SOURCE_DIR}/*.cpp"
  "${VELD_FMT_SOURCE_DIR}/src/*.c"
  "${VELD_FMT_SOURCE_DIR}/src/*.cpp"
)
add_library(fmt STATIC ${FMT_SOURCES})
target_include_directories(fmt PUBLIC ${VELD_FMT_INCLUDE_DIR})

# Executable
add_executable(myapp)
target_link_libraries(myapp PRIVATE fmt veld_deps)
```

**Important:** This file is **never overwritten** if it already exists. If you already have a `CMakeLists.txt`, veld skips generation and prints a warning. This allows you to maintain custom project structure while still using veld for dependency management.

**Generation logic:**

The `render_cmakelists()` function produces this file. It:
- Sets `cmake_minimum_required(VERSION 3.15)`.
- Calls `project()` with name, version, languages, and optional description.
- Includes `veld_deps.cmake`.
- For each dependency, globs for `.c` and `.cpp` files and creates a static library.
- Creates a placeholder executable and links all dependencies.

## Generation Flow

```
veld install / veld build
    |
    +--> generate_cmake()          --> veld.cmake + veld_deps.cmake
    |
    +--> generate_build_helper()   --> veld-build.cmake
    |
    +--> generate_cmakelists()     --> CMakeLists.txt (skip if exists)
```

All generation functions are in `crates/veld-generator/src/lib.rs`.

## C++ Standard Mapping

| `CxxStandard` | CMake Value |
|---------------|-------------|
| `C99` | `"99"` (sets `CMAKE_C_STANDARD`) |
| `C11` | `"11"` (sets `CMAKE_C_STANDARD`) |
| `Cxx11` | `"11"` (sets `CMAKE_CXX_STANDARD`) |
| `Cxx14` | `"14"` (sets `CMAKE_CXX_STANDARD`) |
| `Cxx17` | `"17"` (sets `CMAKE_CXX_STANDARD`) |
| `Cxx20` | `"20"` (sets `CMAKE_CXX_STANDARD`) |

The CMake variable is set to `CMAKE_C_STANDARD` for C variants (`c99`, `c11`) and `CMAKE_CXX_STANDARD` for C++ variants.

## Using Generated Files Without `veld build --run`

You can use the generated CMake files directly:

```bash
cmake -B build -DCMAKE_BUILD_TYPE=Debug
cmake --build build
```

Or in your own CMakeLists.txt:

```cmake
cmake_minimum_required(VERSION 3.15)
project(myapp)

include(veld-build.cmake)

add_executable(myapp src/main.cpp)
veld_add_dependencies(myapp)
```
