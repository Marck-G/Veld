# Dependency Resolution

Veld resolves dependencies by building a **directed acyclic graph (DAG)** of all packages (direct and transitive) and detecting conflicts and cycles.

## Resolution Algorithm

### Overview

```
veld.toml
    |
    v
[Iterate dependencies + dev_dependencies]
    |
    v
[For each dependency: resolve_dependency()]
    |
    v
[Git: clone/fetch -> load manifest -> recurse]
[Path: canonicalize -> load manifest -> recurse]
    |
    v
[DependencyGraph] --> [Lockfile]
```

### Step-by-Step

1. **Entry Point** -- The resolver iterates over both `[dependencies]` and `[dev_dependencies]` from the manifest.

2. **Dispatch** -- Each dependency is dispatched based on its source type:
   - **Git**: `resolve_git_dependency()`
   - **Path**: `resolve_path_dependency()`
   - **Registry**: Skipped with warning (not yet implemented)

3. **Git Resolution** (`crates/veld-resolver/src/provider.rs`):
   - Converts the `GitSource` into a `SourceRef` (Tag, Branch, or Commit).
   - Checks for **conflicts**: if the same package name was already resolved with a different reference, a warning is emitted. The **first-wins** strategy keeps the original.
   - Checks the **visited set**: if the package was already resolved, skips it.
   - **Fetches** the dependency via `LibGit2Fetcher` to the local cache.
   - **Loads the manifest** from the fetched source (if a `veld.toml` exists there).
   - Creates a `ResolvedPackage` with name, version, repository URL, commit hash, artifact ID, and source path.
   - **Recursively resolves** the dependency's own dependencies.

4. **Path Resolution** (`crates/veld-resolver/src/provider.rs`):
   - Resolves relative paths to absolute using `std::fs::canonicalize()`.
   - Loads the manifest from the path (if `veld.toml` exists).
   - Creates a `ResolvedPackage` with `repository = None` and `commit = None`.
   - **Recursively resolves** the dependency's own dependencies.

5. **Graph Construction** -- Each resolved package is added to the `DependencyGraph`. Edges are created from dependency to dependent (so topological sort produces dependencies first).

6. **Cycle Detection** -- After graph construction, `topological_order()` is called. If the graph has cycles, petgraph returns an error.

7. **Conflict Reporting** -- All accumulated conflicts are returned in the `ResolutionResult`.

## The Dependency Graph

### Structure (`crates/veld-resolver/src/graph.rs`)

```rust
pub struct DependencyGraph {
    graph: DiGraph<ResolvedPackage, ()>,
    name_index: HashMap<String, NodeIndex>,
}
```

The graph uses `petgraph::DiGraph` (directed graph) with packages as nodes and dependency relationships as edges.

### Edge Direction

Edges point **from dependency to dependent**:

```
fmt -> myapp    (myapp depends on fmt)
spdlog -> myapp (myapp depends on spdlog)
fmt -> spdlog   (spdlog depends on fmt)
```

This means topological sort produces `fmt, spdlog, myapp` -- dependencies before dependents.

### Key Operations

| Method | Description |
|--------|-------------|
| `add_package()` | Add a node. First-encountered wins for duplicate names. |
| `add_dependency()` | Add an edge from dependency to dependent. |
| `find_by_name()` | Look up a package by name. |
| `topological_order()` | Return packages in dependency-first order. |
| `has_cycles()` | Check for circular dependencies. |
| `dependencies_of()` | What does this package depend on? |
| `dependents_of()` | What depends on this package? |

## First-Wins Strategy

When the same package name appears with different versions:

```
myapp -> fmt@10.2.1 (direct)
myapp -> spdlog -> fmt@9.0.0 (transitive)
```

Veld keeps **fmt@10.2.1** (the first encountered) and emits a warning:

```
Warning: conflict for fmt: already resolved as tag:10.2.1, now requested as tag:9.0.0. Keeping first.
```

This is a deliberate design choice (similar to Go's module system). It avoids complex version negotiation at the cost of potential incompatibilities.

## Conflict Detection

A `DependencyConflict` is recorded when:

- The same package name is resolved more than once.
- The two resolutions have **different source references** (different tags, branches, commits, or source types).

Conflicts are returned in `ResolutionResult.conflicts` but do **not** halt resolution. The first-wins strategy always produces a valid (if potentially incompatible) graph.

## Cycle Detection

Circular dependencies are detected via petgraph's topological sort:

```
A -> B -> C -> A  (cycle!)
```

If a cycle is found, resolution returns an error:

```
Error: circular dependency detected
```

## Transitive Dependencies

If a dependency has its own `veld.toml`, those dependencies are recursively resolved:

```
myapp/veld.toml:
  [dependencies.spdlog]
  source.repo = "https://github.com/gabime/spdlog.git"
  source.tag = "v1.12.0"

spdlog/veld.toml (in the fetched repo):
  [dependencies.fmt]
  source.repo = "https://github.com/fmtlib/fmt.git"
  source.tag = "10.2.1"
```

Resolution produces: `myapp -> spdlog -> fmt`

If a dependency does **not** have a `veld.toml`, it is treated as a leaf node with no transitive dependencies.

## Resolution Result

```rust
pub struct ResolutionResult {
    pub graph: DependencyGraph,
    pub conflicts: Vec<DependencyConflict>,
}
```

The `graph` is always valid (no cycles). The `conflicts` list contains all version conflicts that were detected during resolution. Consumers can inspect conflicts to decide whether to proceed or abort.

## Source Path Resolution

After resolution, each package's source path is determined:

| Source Type | Source Path |
|-------------|-------------|
| Git | `{cache_dir}/{prefix}/{Name}/{commit}/src` (or `{subdir}` within that) |
| Path | The canonicalized absolute path |

This source path is written to the lockfile and used by the CMake generator to set `VELD_<NAME>_SOURCE_DIR`.
