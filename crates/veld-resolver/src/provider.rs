//! Dependency resolution: recursive tree walking with git fetching.
//!
//! This module provides `DependencyResolver`, which:
//! 1. Takes a root manifest
//! 2. Recursively fetches and resolves all transitive git dependencies
//! 3. Builds a `DependencyGraph` suitable for lockfile generation and CMake output
//!
//! The resolution algorithm is Go-like: the first encountered version of a package
//! wins. If the same package is encountered with a different git ref, a warning
//! is emitted and the first version is kept.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use semver::Version;
use veld_config::{
    load_manifest, DependencySource, DependencySpec, GitSource, Manifest, PathSource,
};
use veld_error::{ErrorCode, ErrorContext, VeldError, VeldResult};
use veld_fetcher_git::{FetchedSource, GitFetcher, LibGit2Fetcher, SourceRef};

use crate::cache::CacheManager;
use crate::graph::{DependencyGraph, ResolvedPackage};

/// A dependency conflict: the same package was requested with different refs.
#[derive(Debug, Clone)]
pub struct DependencyConflict {
    pub package_name: String,
    pub existing_ref: String,
    pub conflicting_ref: String,
}

/// Result of dependency resolution.
pub struct ResolutionResult {
    /// The resolved dependency graph.
    pub graph: DependencyGraph,
    /// Warnings about dependency conflicts (same package, different ref).
    pub conflicts: Vec<DependencyConflict>,
}

/// Resolves dependencies recursively by fetching git repos and walking the tree.
pub struct DependencyResolver {
    cache: CacheManager,
    fetcher: LibGit2Fetcher,
    /// Track visited packages to avoid infinite recursion.
    visited: HashSet<String>,
    /// Track resolved refs per package for conflict detection.
    resolved_refs: BTreeMap<String, String>,
    /// Accumulated conflicts.
    conflicts: Vec<DependencyConflict>,
}

impl DependencyResolver {
    /// Create a new resolver with the default cache and fetcher.
    pub fn new() -> Self {
        Self {
            cache: CacheManager::new(),
            fetcher: LibGit2Fetcher,
            visited: HashSet::new(),
            resolved_refs: BTreeMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Create a new resolver with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            cache: CacheManager::with_base_dir(cache_dir),
            fetcher: LibGit2Fetcher,
            visited: HashSet::new(),
            resolved_refs: BTreeMap::new(),
            conflicts: Vec::new(),
        }
    }

    /// Resolve all dependencies for a given manifest.
    ///
    /// This is the main entry point. It:
    /// 1. Ensures the cache directory exists
    /// 2. Iterates over all dependencies in the manifest
    /// 3. For each git dependency, fetches it (or uses cache), then recurses
    /// 4. Returns the complete dependency graph and any conflicts
    pub fn resolve(&mut self, manifest: &Manifest) -> VeldResult<ResolutionResult> {
        self.cache.ensure_base()?;

        let mut graph = DependencyGraph::new();

        for (dep_name, dep_spec) in &manifest.dependencies {
            self.resolve_dependency(dep_name, dep_spec, &mut graph, "root")?;
        }

        // Also resolve dev dependencies
        for (dep_name, dep_spec) in &manifest.dev_dependencies {
            self.resolve_dependency(dep_name, dep_spec, &mut graph, "root")?;
        }

        Ok(ResolutionResult {
            graph,
            conflicts: std::mem::take(&mut self.conflicts),
        })
    }

    /// Resolve a single dependency and all its transitive dependencies.
    fn resolve_dependency(
        &mut self,
        dep_name: &str,
        dep_spec: &DependencySpec,
        graph: &mut DependencyGraph,
        _required_by: &str,
    ) -> VeldResult<Option<NodeHandle>> {
        match &dep_spec.source {
            DependencySource::Git(git) => self.resolve_git_dependency(dep_name, git, graph),
            DependencySource::Path(path) => self.resolve_path_dependency(dep_name, path, graph),
            DependencySource::Registry { .. } => {
                eprintln!(
                    "warning: registry dependency '{}' is not yet supported, skipping",
                    dep_name
                );
                Ok(None)
            }
        }
    }

    /// Resolve a git-based dependency.
    fn resolve_git_dependency(
        &mut self,
        dep_name: &str,
        git_source: &GitSource,
        graph: &mut DependencyGraph,
    ) -> VeldResult<Option<NodeHandle>> {
        // Determine the source ref
        let source_ref = self.git_source_to_ref(git_source)?;

        // Check for conflicts with previously resolved refs
        let ref_str = format!("{}", source_ref);
        if let Some(existing_ref) = self.resolved_refs.get(dep_name) {
            if existing_ref != &ref_str {
                self.conflicts.push(DependencyConflict {
                    package_name: dep_name.to_string(),
                    existing_ref: existing_ref.clone(),
                    conflicting_ref: ref_str,
                });
                // First-encountered wins (Go-like)
                return Ok(graph.find_index(dep_name).map(NodeHandle));
            }
        }
        self.resolved_refs.insert(dep_name.to_string(), ref_str);

        // Check if already visited (avoid infinite recursion)
        if self.visited.contains(dep_name) {
            return Ok(graph.find_index(dep_name).map(NodeHandle));
        }
        self.visited.insert(dep_name.to_string());

        // Fetch the dependency
        let fetched = self.fetch_dependency(dep_name, git_source, &source_ref)?;

        // Get the source directory where we look for veld.toml
        let src_dir = self.get_source_dir(dep_name, &fetched, git_source)?;

        // Try to load the dependency's manifest
        let dep_manifest = match load_manifest(&src_dir) {
            Ok(m) => m,
            Err(_) => {
                // No veld.toml found — treat as leaf dependency (no transitive deps)
                let package =
                    self.make_resolved_package(dep_name, &fetched, git_source, &src_dir, None);
                let idx = graph.add_package(package);
                return Ok(Some(NodeHandle(idx)));
            }
        };

        // Create the resolved package
        let package = self.make_resolved_package(
            dep_name,
            &fetched,
            git_source,
            &src_dir,
            Some(&dep_manifest),
        );
        let parent_idx = graph.add_package(package);

        // Copy the dependency's deps into a local vec to avoid borrow issues
        let child_deps: Vec<(String, DependencySpec)> = dep_manifest
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Recursively resolve transitive dependencies
        for (child_name, child_spec) in &child_deps {
            if let Some(child_handle) =
                self.resolve_dependency(child_name, child_spec, graph, dep_name)?
            {
                graph.add_dependency(parent_idx, child_handle.0);
            }
        }

        Ok(Some(NodeHandle(parent_idx)))
    }

    /// Resolve a path-based (local) dependency.
    fn resolve_path_dependency(
        &mut self,
        dep_name: &str,
        path_source: &PathSource,
        graph: &mut DependencyGraph,
    ) -> VeldResult<Option<NodeHandle>> {
        // Check if already visited
        if self.visited.contains(dep_name) {
            return Ok(graph.find_index(dep_name).map(NodeHandle));
        }
        self.visited.insert(dep_name.to_string());

        // Resolve the path to an absolute path
        let dep_path = std::path::Path::new(&path_source.path);
        let abs_path = if dep_path.is_absolute() {
            dep_path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| {
                    VeldError::new(
                        ErrorCode::FileError(
                            ".".into(),
                            format!("Cannot determine current directory: {}", e),
                        ),
                        ErrorContext::new(),
                    )
                })?
                .join(dep_path)
        };

        // Canonicalize the path
        let canonical_path = abs_path.canonicalize().map_err(|e| {
            VeldError::new(
                ErrorCode::FileError(
                    abs_path.to_string_lossy().into_owned(),
                    format!("Failed to resolve path dependency '{}': {}", dep_name, e),
                ),
                ErrorContext::new().with_file(abs_path.clone()),
            )
        })?;

        // Try to load the dependency's manifest from the path
        let dep_manifest = match load_manifest(&canonical_path) {
            Ok(m) => m,
            Err(_) => {
                // No veld.toml found -- treat as leaf dependency
                let package = ResolvedPackage {
                    name: dep_name.to_string(),
                    version: semver::Version::new(0, 0, 0),
                    repository: None,
                    commit: None,
                    artifact_id: format!("{:0<34}", dep_name),
                    src_path: canonical_path.clone(),
                };
                let idx = graph.add_package(package);
                return Ok(Some(NodeHandle(idx)));
            }
        };

        let version = dep_manifest.package.version.clone();

        // Create the resolved package
        let package = ResolvedPackage {
            name: dep_name.to_string(),
            version,
            repository: None,
            commit: None,
            artifact_id: format!("{:0<34}", dep_name),
            src_path: canonical_path.clone(),
        };
        let parent_idx = graph.add_package(package);

        // Copy the dependency's deps into a local vec to avoid borrow issues
        let child_deps: Vec<(String, DependencySpec)> = dep_manifest
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Recursively resolve transitive dependencies
        for (child_name, child_spec) in &child_deps {
            if let Some(child_handle) =
                self.resolve_dependency(child_name, child_spec, graph, dep_name)?
            {
                graph.add_dependency(parent_idx, child_handle.0);
            }
        }

        Ok(Some(NodeHandle(parent_idx)))
    }

    /// Convert a GitSource to a SourceRef for the fetcher.
    fn git_source_to_ref(&self, git: &GitSource) -> VeldResult<SourceRef> {
        if let Some(ref branch) = git.branch {
            Ok(SourceRef::Branch(branch.clone()))
        } else if let Some(ref tag) = git.tag {
            Ok(SourceRef::Tag(tag.clone()))
        } else if let Some(ref commit) = git.commit {
            Ok(SourceRef::Commit(commit.clone()))
        } else {
            Err(VeldError::new(
                ErrorCode::GitMissingRev("unknown".to_string()),
                ErrorContext::new(),
            ))
        }
    }

    /// Fetch a dependency into the cache, or use existing cached version.
    fn fetch_dependency(
        &self,
        dep_name: &str,
        git_source: &GitSource,
        source_ref: &SourceRef,
    ) -> VeldResult<FetchedSource> {
        // Build destination path in cache
        let dest = self.cache.dependency_path(dep_name)?;

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                VeldError::new(
                    ErrorCode::FileError(
                        parent.to_string_lossy().into_owned(),
                        format!("Failed to create cache directory: {}", e),
                    ),
                    ErrorContext::new().with_file(parent.to_path_buf()),
                )
            })?;
        }

        self.fetcher.fetch(&git_source.repo, source_ref, &dest)
    }

    /// Get the source directory for a dependency.
    ///
    /// If `subdir` is specified in the git source, returns `{dest}/{subdir}`.
    /// Otherwise returns `dest` directly.
    fn get_source_dir(
        &self,
        _dep_name: &str,
        fetched: &FetchedSource,
        git_source: &GitSource,
    ) -> VeldResult<PathBuf> {
        if let Some(ref subdir) = git_source.subdir {
            Ok(fetched.path.join(subdir))
        } else {
            Ok(fetched.path.clone())
        }
    }

    /// Create a ResolvedPackage from fetched source information.
    fn make_resolved_package(
        &self,
        dep_name: &str,
        fetched: &FetchedSource,
        git_source: &GitSource,
        src_dir: &Path,
        manifest: Option<&Manifest>,
    ) -> ResolvedPackage {
        let version = manifest
            .map(|m| m.package.version.clone())
            .unwrap_or_else(|| Version::new(0, 0, 0));

        ResolvedPackage {
            name: dep_name.to_string(),
            version,
            repository: Some(git_source.repo.clone()),
            commit: Some(fetched.commit.clone()),
            artifact_id: format!("{:0<34}", dep_name),
            src_path: src_dir.to_path_buf(),
        }
    }
}

/// Wrapper around NodeIndex for internal use.
struct NodeHandle(petgraph::graph::NodeIndex);

impl Default for DependencyResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_source_to_ref_branch() {
        let resolver = DependencyResolver::new();
        let git = GitSource {
            repo: "https://github.com/test/lib".to_string(),
            branch: Some("main".to_string()),
            tag: None,
            commit: None,
            subdir: None,
            fetch_depth: None,
        };
        let source_ref = resolver.git_source_to_ref(&git).unwrap();
        assert_eq!(source_ref, SourceRef::Branch("main".to_string()));
    }

    #[test]
    fn test_git_source_to_ref_tag() {
        let resolver = DependencyResolver::new();
        let git = GitSource {
            repo: "https://github.com/test/lib".to_string(),
            branch: None,
            tag: Some("v1.0.0".to_string()),
            commit: None,
            subdir: None,
            fetch_depth: None,
        };
        let source_ref = resolver.git_source_to_ref(&git).unwrap();
        assert_eq!(source_ref, SourceRef::Tag("v1.0.0".to_string()));
    }

    #[test]
    fn test_git_source_to_ref_commit() {
        let resolver = DependencyResolver::new();
        let git = GitSource {
            repo: "https://github.com/test/lib".to_string(),
            branch: None,
            tag: None,
            commit: Some("a".repeat(40)),
            subdir: None,
            fetch_depth: None,
        };
        let source_ref = resolver.git_source_to_ref(&git).unwrap();
        assert_eq!(source_ref, SourceRef::Commit("a".repeat(40)));
    }

    #[test]
    fn test_git_source_to_ref_missing() {
        let resolver = DependencyResolver::new();
        let git = GitSource {
            repo: "https://github.com/test/lib".to_string(),
            branch: None,
            tag: None,
            commit: None,
            subdir: None,
            fetch_depth: None,
        };
        assert!(resolver.git_source_to_ref(&git).is_err());
    }
}
