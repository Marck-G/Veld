//! Dependency graph: DAG representation of resolved packages.
//!
//! Provides:
//! - `ResolvedPackage` — a fully resolved dependency node
//! - `DependencyGraph` — a DAG with topological sort and cycle detection

use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{EdgeRef, IntoNodeReferences};
use std::collections::HashMap;

/// A fully resolved dependency node in the graph.
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Package name (as declared in its manifest)
    pub name: String,
    /// Semantic version from the package manifest
    pub version: semver::Version,
    /// Git repository URL
    pub repository: String,
    /// Resolved 40-char SHA-1 commit hash
    pub commit: String,
    /// Artifact ID used in lockfile and CMake targets
    pub artifact_id: String,
    /// Path to the source directory in the local cache
    pub src_path: std::path::PathBuf,
}

/// Directed acyclic graph of resolved dependencies.
///
/// Nodes are `ResolvedPackage` instances; edges represent "depends on" relationships.
/// The root project is NOT a node — it is the implicit source of all outgoing edges
/// to direct dependencies.
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    graph: DiGraph<ResolvedPackage, ()>,
    /// Map from package name to node index for fast lookup
    name_index: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            name_index: HashMap::new(),
        }
    }

    /// Add a resolved package to the graph.
    ///
    /// If a package with the same name already exists, returns its existing NodeIndex
    /// without adding a duplicate. The first occurrence wins (Go-like behavior).
    pub fn add_package(&mut self, package: ResolvedPackage) -> NodeIndex {
        if let Some(&idx) = self.name_index.get(&package.name) {
            return idx;
        }

        let idx = self.graph.add_node(package.clone());
        self.name_index.insert(package.name.clone(), idx);
        idx
    }

    /// Add a dependency edge: `dependent` depends on `dependency`.
    ///
    /// Creates an edge from dependency to dependent so that topological sort
    /// produces the correct build order (dependencies before dependents).
    ///
    /// Both packages must already be in the graph (call `add_package` first).
    pub fn add_dependency(&mut self, dependent: NodeIndex, dependency: NodeIndex) {
        // Edge direction: dependency → dependent (so toposort puts deps first)
        if !self.graph.contains_edge(dependency, dependent) {
            self.graph.add_edge(dependency, dependent, ());
        }
    }

    /// Find a package by name.
    pub fn find_by_name(&self, name: &str) -> Option<&ResolvedPackage> {
        self.name_index.get(name).map(|&idx| &self.graph[idx])
    }

    /// Find the node index of a package by name.
    pub fn find_index(&self, name: &str) -> Option<NodeIndex> {
        self.name_index.get(name).copied()
    }

    /// Get all packages in topological order (dependencies before dependents).
    ///
    /// This is the correct build order: each package's dependencies appear
    /// before the package itself.
    ///
    /// Returns an error if the graph contains cycles.
    pub fn topological_order(&self) -> Result<Vec<&ResolvedPackage>, String> {
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let node = &self.graph[cycle.node_id()];
            format!(
                "Circular dependency detected involving package '{}'",
                node.name
            )
        })?;

        Ok(sorted.into_iter().map(|idx| &self.graph[idx]).collect())
    }

    /// Check if the graph has any cycles.
    pub fn has_cycles(&self) -> bool {
        toposort(&self.graph, None).is_err()
    }

    /// Get the number of packages in the graph.
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Check if the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// Iterate over all packages (unordered).
    pub fn packages(&self) -> impl Iterator<Item = &ResolvedPackage> {
        self.graph.node_weights()
    }

    /// Iterate over all packages with their node indices.
    pub fn packages_with_indices(&self) -> impl Iterator<Item = (NodeIndex, &ResolvedPackage)> {
        self.graph.node_references()
    }

    /// Get direct dependencies of a package (by node index).
    ///
    /// Edge direction: dependency → dependent.
    /// Incoming edges to a node come from its dependencies.
    pub fn dependencies_of(&self, idx: NodeIndex) -> Vec<&ResolvedPackage> {
        self.graph
            .edges_directed(idx, petgraph::Direction::Incoming)
            .map(|edge| &self.graph[edge.source()])
            .collect()
    }

    /// Get packages that depend on a given package (by node index).
    ///
    /// Edge direction: dependency → dependent.
    /// Outgoing edges from a node point to its dependents.
    pub fn dependents_of(&self, idx: NodeIndex) -> Vec<&ResolvedPackage> {
        self.graph
            .edges_directed(idx, petgraph::Direction::Outgoing)
            .map(|edge| &self.graph[edge.target()])
            .collect()
    }

    /// Get the internal petgraph for advanced use (e.g., lockfile generation).
    pub fn inner_graph(&self) -> &DiGraph<ResolvedPackage, ()> {
        &self.graph
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_package(name: &str, version: &str, repo: &str, commit: &str) -> ResolvedPackage {
        ResolvedPackage {
            name: name.to_string(),
            version: semver::Version::parse(version).unwrap(),
            repository: repo.to_string(),
            commit: commit.to_string(),
            artifact_id: format!("{:0<34}", name),
            src_path: PathBuf::from(format!("/cache/{}/src", name)),
        }
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.len(), 0);
    }

    #[test]
    fn test_add_single_package() {
        let mut graph = DependencyGraph::new();
        let pkg = make_package(
            "zlib",
            "1.3.1",
            "https://github.com/madler/zlib",
            &"0".repeat(40),
        );
        let _idx = graph.add_package(pkg);

        assert_eq!(graph.len(), 1);
        assert!(graph.find_by_name("zlib").is_some());
        assert_eq!(
            graph.find_by_name("zlib").unwrap().version,
            semver::Version::new(1, 3, 1)
        );
    }

    #[test]
    fn test_duplicate_package_wins_first() {
        let mut graph = DependencyGraph::new();
        let pkg1 = make_package(
            "zlib",
            "1.3.1",
            "https://github.com/madler/zlib",
            &"a".repeat(40),
        );
        let pkg2 = make_package(
            "zlib",
            "1.3.0",
            "https://github.com/madler/zlib",
            &"b".repeat(40),
        );

        let idx1 = graph.add_package(pkg1);
        let idx2 = graph.add_package(pkg2);

        assert_eq!(idx1, idx2); // Same node index returned
        assert_eq!(graph.len(), 1);
        assert_eq!(
            graph.find_by_name("zlib").unwrap().version,
            semver::Version::new(1, 3, 1)
        );
    }

    #[test]
    fn test_dependency_edges() {
        let mut graph = DependencyGraph::new();
        let curl_idx = graph.add_package(make_package(
            "curl",
            "7.88.0",
            "https://github.com/curl/curl",
            &"a".repeat(40),
        ));
        let ssl_idx = graph.add_package(make_package(
            "openssl",
            "3.1.0",
            "https://github.com/openssl/openssl",
            &"b".repeat(40),
        ));

        graph.add_dependency(curl_idx, ssl_idx); // curl depends on openssl

        let deps = graph.dependencies_of(curl_idx);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].name, "openssl");

        let dependents = graph.dependents_of(ssl_idx);
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].name, "curl");
    }

    #[test]
    fn test_topological_order() {
        let mut graph = DependencyGraph::new();
        let zlib_idx = graph.add_package(make_package(
            "zlib",
            "1.3.1",
            "https://github.com/madler/zlib",
            &"a".repeat(40),
        ));
        let ssl_idx = graph.add_package(make_package(
            "openssl",
            "3.1.0",
            "https://github.com/openssl/openssl",
            &"b".repeat(40),
        ));
        let curl_idx = graph.add_package(make_package(
            "curl",
            "7.88.0",
            "https://github.com/curl/curl",
            &"c".repeat(40),
        ));

        // curl -> openssl, curl -> zlib
        graph.add_dependency(curl_idx, ssl_idx);
        graph.add_dependency(curl_idx, zlib_idx);

        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 3);

        // curl must come after openssl and zlib
        let curl_pos = order.iter().position(|p| p.name == "curl").unwrap();
        let ssl_pos = order.iter().position(|p| p.name == "openssl").unwrap();
        let zlib_pos = order.iter().position(|p| p.name == "zlib").unwrap();

        assert!(curl_pos > ssl_pos);
        assert!(curl_pos > zlib_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        let a_idx = graph.add_package(make_package(
            "a",
            "1.0.0",
            "https://github.com/test/a",
            &"a".repeat(40),
        ));
        let b_idx = graph.add_package(make_package(
            "b",
            "1.0.0",
            "https://github.com/test/b",
            &"b".repeat(40),
        ));
        let c_idx = graph.add_package(make_package(
            "c",
            "1.0.0",
            "https://github.com/test/c",
            &"c".repeat(40),
        ));

        // a -> b -> c -> a (cycle)
        graph.add_dependency(a_idx, b_idx);
        graph.add_dependency(b_idx, c_idx);
        graph.add_dependency(c_idx, a_idx);

        assert!(graph.has_cycles());
        assert!(graph.topological_order().is_err());
    }

    #[test]
    fn test_no_cycle_in_diamond() {
        let mut graph = DependencyGraph::new();
        let a_idx = graph.add_package(make_package(
            "a",
            "1.0.0",
            "https://github.com/test/a",
            &"a".repeat(40),
        ));
        let b_idx = graph.add_package(make_package(
            "b",
            "1.0.0",
            "https://github.com/test/b",
            &"b".repeat(40),
        ));
        let c_idx = graph.add_package(make_package(
            "c",
            "1.0.0",
            "https://github.com/test/c",
            &"c".repeat(40),
        ));
        let d_idx = graph.add_package(make_package(
            "d",
            "1.0.0",
            "https://github.com/test/d",
            &"d".repeat(40),
        ));

        // Diamond: a -> b -> d, a -> c -> d
        graph.add_dependency(a_idx, b_idx);
        graph.add_dependency(a_idx, c_idx);
        graph.add_dependency(b_idx, d_idx);
        graph.add_dependency(c_idx, d_idx);

        assert!(!graph.has_cycles());
        assert!(graph.topological_order().is_ok());
    }
}
