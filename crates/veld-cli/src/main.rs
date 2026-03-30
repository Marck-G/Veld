//! Veld CLI — C/C++ package manager
//!
//! Commands:
//! - `veld init` — Create a new veld.toml manifest
//! - `veld install` — Resolve dependencies, fetch sources, generate lockfile + CMake
//! - `veld add <dep> --git <url> --tag|--branch|--commit <ref>` — Add a git dependency
//! - `veld add <dep> --path <path>` — Add a local path dependency
//! - `veld build` — Generate CMake files and optionally run cmake

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use owo_colors::OwoColorize;
use semver::Version;

use veld_config::{
    constants::FILE_BUILD_NAME, constants::FILE_LOCK_NAME, hash_manifest, hash_profile,
    load_manifest, BuildType, CxxStandard, DependencySource, DependencySpec, GitSource, Manifest,
    OptimizationLevel, PackageMetadata, PathSource, ProfileConfig,
};
use veld_error::VeldError;
use veld_generator::{
    generate_build_helper, generate_cmake, generate_cmakelists, CMakeConfig, CMakeDependency,
    CMakeProjectConfig,
};
use veld_lock::{LockedPackage, Lockfile};
use veld_resolver::{DependencyGraph, DependencyResolver};

#[derive(Parser)]
#[command(
    name = "veld",
    version,
    about = "A C/C++ package manager with git and path dependencies"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new veld project (creates veld.toml)
    Init {
        /// Package name (defaults to directory name)
        #[arg(long)]
        name: Option<String>,

        /// Package version
        #[arg(long, default_value = "0.1.0")]
        version: String,
    },

    /// Resolve dependencies, fetch sources, and generate lockfile + CMake
    Install {
        /// Force re-resolution even if lockfile is valid
        #[arg(long)]
        force: bool,
    },

    /// Add a dependency to veld.toml (git or local path)
    Add {
        /// Dependency name
        name: String,

        /// Git repository URL
        #[arg(long, conflicts_with = "path")]
        git: Option<String>,

        /// Local filesystem path to the dependency
        #[arg(long, conflicts_with = "git")]
        path: Option<String>,

        /// Git branch to use
        #[arg(long, conflicts_with_all = ["tag", "commit"])]
        branch: Option<String>,

        /// Git tag to use
        #[arg(long, conflicts_with_all = ["branch", "commit"])]
        tag: Option<String>,

        /// Git commit hash to use
        #[arg(long, conflicts_with_all = ["branch", "tag"])]
        commit: Option<String>,

        /// Add as dev dependency
        #[arg(long)]
        dev: bool,
    },

    /// Generate CMake files for the project
    Build {
        /// C++ standard (e.g., 11, 14, 17, 20)
        #[arg(long, default_value = "17")]
        std: String,

        /// Build type (debug, release)
        #[arg(long, default_value = "debug")]
        build_type: String,
    },

    /// Show the dependency tree
    Tree,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { name, version } => cmd_init(name, version),
        Commands::Install { force } => cmd_install(force),
        Commands::Add {
            name,
            git,
            path,
            branch,
            tag,
            commit,
            dev,
        } => cmd_add(name, git, path, branch, tag, commit, dev),
        Commands::Build { std, build_type } => cmd_build(std, build_type),
        Commands::Tree => cmd_tree(),
    };

    if let Err(e) = result {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

// ─────────────────────────────────────────────
// INIT
// ─────────────────────────────────────────────

fn cmd_init(name: Option<String>, version_str: String) -> Result<(), VeldError> {
    let cwd = std::env::current_dir().map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                ".".into(),
                format!("Cannot determine current directory: {}", e),
            ),
            veld_error::ErrorContext::new(),
        )
    })?;

    let manifest_path = cwd.join(FILE_BUILD_NAME);
    if manifest_path.exists() {
        eprintln!(
            "{} {} already exists",
            "!".yellow().bold(),
            FILE_BUILD_NAME.yellow()
        );
        return Ok(());
    }

    let pkg_name = name.unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("myproject")
            .to_string()
    });

    let version = Version::parse(&version_str).map_err(|_e| {
        VeldError::new(
            veld_error::ErrorCode::InvalidVersion(version_str.clone()),
            veld_error::ErrorContext::new(),
        )
    })?;

    let manifest = Manifest {
        package: PackageMetadata {
            name: pkg_name.clone(),
            version: version.clone(),
            authors: None,
            license: Some("MIT".to_string()),
            description: None,
        },
        profiles: {
            let mut m = BTreeMap::new();
            m.insert(
                "default".to_string(),
                ProfileConfig {
                    cxx_std: CxxStandard::Cxx17,
                    build: BuildType::Debug,
                    optimization: OptimizationLevel::O0,
                    pic: true,
                    lto: false,
                    sanitizers: BTreeSet::new(),
                    target: None,
                },
            );
            m
        },
        dependencies: BTreeMap::new(),
        dev_dependencies: BTreeMap::new(),
    };

    let content = toml::to_string_pretty(&manifest).map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::ManifestParseError(format!(
                "Failed to serialize manifest: {}",
                e
            )),
            veld_error::ErrorContext::new(),
        )
    })?;

    std::fs::write(&manifest_path, content).map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                manifest_path.to_string_lossy().into_owned(),
                format!("Failed to write manifest: {}", e),
            ),
            veld_error::ErrorContext::new().with_file(manifest_path.clone()),
        )
    })?;

    println!(
        "{} Created {} for {} v{}",
        "✓".green().bold(),
        FILE_BUILD_NAME.green(),
        pkg_name,
        version
    );
    Ok(())
}

// ─────────────────────────────────────────────
// INSTALL
// ─────────────────────────────────────────────

fn cmd_install(force: bool) -> Result<(), VeldError> {
    let cwd = std::env::current_dir().map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                ".".into(),
                format!("Cannot determine current directory: {}", e),
            ),
            veld_error::ErrorContext::new(),
        )
    })?;

    // Load manifest
    let manifest = load_manifest(&cwd)?;
    println!(
        "{} Resolving dependencies for {} v{}",
        "→".cyan().bold(),
        manifest.package.name,
        manifest.package.version
    );

    // Check if lockfile is still valid (unless --force)
    let lock_path = cwd.join(FILE_LOCK_NAME);
    if !force && lock_path.exists() {
        let current_manifest_hash = hash_manifest(&manifest)?;
        match Lockfile::read(&lock_path) {
            Ok(lock) if lock.metadata.manifest_hash == current_manifest_hash => {
                println!(
                    "{} {} is up to date (use --force to re-resolve)",
                    "✓".green().bold(),
                    FILE_LOCK_NAME.green()
                );
                // Still regenerate CMake in case it was deleted
                generate_cmake_from_graph(&cwd, &manifest, &lock)?;
                return Ok(());
            }
            _ => {
                println!(
                    "{} {} is outdated, re-resolving...",
                    "!".yellow().bold(),
                    FILE_LOCK_NAME.yellow()
                );
            }
        }
    }

    // Resolve dependencies
    let mut resolver = DependencyResolver::new();
    let resolution = resolver.resolve(&manifest)?;

    if !resolution.conflicts.is_empty() {
        for conflict in &resolution.conflicts {
            eprintln!(
                "{} Conflict in '{}': {} vs {} (using first)",
                "!".yellow().bold(),
                conflict.package_name,
                conflict.existing_ref,
                conflict.conflicting_ref,
            );
        }
    }

    let graph = &resolution.graph;
    println!(
        "{} Resolved {} dependencies",
        "✓".green().bold(),
        graph.len()
    );

    // Check for cycles
    if graph.has_cycles() {
        return Err(VeldError::new(
            veld_error::ErrorCode::LockfileCorrupted(
                "Circular dependency detected in resolved graph".to_string(),
            ),
            veld_error::ErrorContext::new(),
        ));
    }

    // Convert graph to lockfile packages
    let manifest_hash = hash_manifest(&manifest)?;
    let profile_hash = hash_profile(
        &manifest,
        manifest
            .profiles
            .keys()
            .next()
            .unwrap_or(&"default".to_string()),
    )?;

    let lock = build_lockfile(graph, profile_hash, manifest_hash)?;

    // Write lockfile
    lock.write(&lock_path)?;
    println!(
        "{} Generated {} with {} packages",
        "✓".green().bold(),
        FILE_LOCK_NAME.green(),
        lock.packages.len()
    );

    // Generate CMake files
    generate_cmake_from_graph(&cwd, &manifest, &lock)?;

    Ok(())
}

/// Build a Lockfile from a DependencyGraph.
fn build_lockfile(
    graph: &DependencyGraph,
    profile_hash: String,
    manifest_hash: String,
) -> Result<Lockfile, VeldError> {
    use petgraph::visit::{EdgeRef, IntoNodeReferences};

    let inner = graph.inner_graph();

    let mut packages: Vec<LockedPackage> = inner
        .node_references()
        .map(|(idx, pkg)| {
            let required_by: Vec<String> = inner
                .edges_directed(idx, petgraph::Direction::Outgoing)
                .map(|edge: petgraph::graph::EdgeReference<'_, ()>| {
                    inner[edge.target()].name.clone()
                })
                .collect();

            // Determine if this is a path dependency (no repository/commit)
            let path_dep = if pkg.repository.is_none() {
                Some(pkg.src_path.to_string_lossy().into_owned())
            } else {
                None
            };

            LockedPackage {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                repository: pkg.repository.clone(),
                commit: pkg.commit.clone(),
                artifact_id: pkg.artifact_id.clone(),
                required_by: if required_by.is_empty() {
                    vec!["root".to_string()]
                } else {
                    required_by
                },
                content_hash: None,
                path: path_dep,
            }
        })
        .collect();

    packages.sort();

    Ok(Lockfile::new(packages, profile_hash, manifest_hash))
}

/// Generate CMake files from the resolved dependency graph.
fn generate_cmake_from_graph(
    project_dir: &Path,
    manifest: &Manifest,
    lock: &Lockfile,
) -> Result<(), VeldError> {
    let cmake_deps: Vec<CMakeDependency> = lock
        .packages
        .iter()
        .map(|pkg| {
            // Determine the source path:
            // - For path dependencies, use the path directly from the lockfile
            // - For git dependencies, compute from the cache
            let src_path = if let Some(ref dep_path) = pkg.path {
                PathBuf::from(dep_path)
            } else {
                let cache_manager = veld_resolver::cache::CacheManager::new();
                cache_manager
                    .src_dir(&pkg.name, pkg.commit.as_deref().unwrap_or_default())
                    .unwrap_or_else(|_| PathBuf::from(format!("/veld/cache/{}/src", pkg.name)))
            };

            CMakeDependency {
                name: pkg.name.clone(),
                src_path,
                version: pkg.version.to_string(),
                required_by: pkg.required_by.clone(),
            }
        })
        .collect();

    // Read C++ standard from the manifest profile instead of hardcoding
    let cxx_standard = manifest
        .profiles
        .values()
        .next()
        .map(|p| cxx_standard_to_string(&p.cxx_std))
        .unwrap_or_else(|| "17".to_string());

    let config = CMakeConfig {
        cxx_standard: cxx_standard.clone(),
        build_type: "Debug".to_string(),
    };

    let cmake_path = generate_cmake(project_dir, &cmake_deps, &config)?;
    let helper_path = generate_build_helper(project_dir, &cmake_deps)?;

    // Generate CMakeLists.txt (skips if already exists)
    let project_config = CMakeProjectConfig {
        project_name: manifest.package.name.clone(),
        project_version: manifest.package.version.to_string(),
        description: manifest.package.description.clone(),
        cxx_standard,
        languages: "C CXX".to_string(),
    };
    let cmakelists_path = generate_cmakelists(project_dir, &project_config, &cmake_deps)?;

    println!(
        "{} Generated {}, {}, and {}",
        "✓".green().bold(),
        cmake_path.file_name().unwrap().to_string_lossy().green(),
        helper_path.file_name().unwrap().to_string_lossy().green(),
        cmakelists_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .green(),
    );

    Ok(())
}

/// Map a `CxxStandard` enum variant to its CMake-compatible string.
fn cxx_standard_to_string(std: &veld_config::CxxStandard) -> String {
    match std {
        veld_config::CxxStandard::C99 => "99".to_string(),
        veld_config::CxxStandard::C11 => "11".to_string(),
        veld_config::CxxStandard::Cxx11 => "11".to_string(),
        veld_config::CxxStandard::Cxx14 => "14".to_string(),
        veld_config::CxxStandard::Cxx17 => "17".to_string(),
        veld_config::CxxStandard::Cxx20 => "20".to_string(),
    }
}

// ─────────────────────────────────────────────
// ADD
// ─────────────────────────────────────────────

fn cmd_add(
    name: String,
    git: Option<String>,
    path: Option<String>,
    branch: Option<String>,
    tag: Option<String>,
    commit: Option<String>,
    dev: bool,
) -> Result<(), VeldError> {
    let cwd = std::env::current_dir().map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                ".".into(),
                format!("Cannot determine current directory: {}", e),
            ),
            veld_error::ErrorContext::new(),
        )
    })?;

    let mut manifest = load_manifest(&cwd)?;

    let (dep_spec, source_desc) = if let Some(git_url) = git {
        let git_source = GitSource {
            repo: git_url.clone(),
            branch,
            tag,
            commit,
            subdir: None,
            fetch_depth: None,
        };
        (
            DependencySpec {
                source: DependencySource::Git(git_source),
                optional: false,
                features: vec![],
            },
            git_url,
        )
    } else if let Some(dep_path) = path {
        (
            DependencySpec {
                source: DependencySource::Path(PathSource {
                    path: dep_path.clone(),
                }),
                optional: false,
                features: vec![],
            },
            dep_path,
        )
    } else {
        return Err(VeldError::new(
            veld_error::ErrorCode::MissingField("git or path".into()),
            veld_error::ErrorContext::new(),
        ));
    };

    if dev {
        manifest.dev_dependencies.insert(name.clone(), dep_spec);
    } else {
        manifest.dependencies.insert(name.clone(), dep_spec);
    }

    // Write back the manifest
    let content = toml::to_string_pretty(&manifest).map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::ManifestParseError(format!(
                "Failed to serialize manifest: {}",
                e
            )),
            veld_error::ErrorContext::new(),
        )
    })?;

    let manifest_path = cwd.join(FILE_BUILD_NAME);
    std::fs::write(&manifest_path, content).map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                manifest_path.to_string_lossy().into_owned(),
                format!("Failed to write manifest: {}", e),
            ),
            veld_error::ErrorContext::new().with_file(manifest_path),
        )
    })?;

    println!(
        "{} Added {} dependency '{}' from {}",
        "✓".green().bold(),
        if dev { "dev" } else { "runtime" },
        name.green(),
        source_desc.cyan()
    );

    println!("  Run {} to fetch and resolve", "veld install".bold());

    Ok(())
}

// ─────────────────────────────────────────────
// BUILD
// ─────────────────────────────────────────────

fn cmd_build(_cxx_std: String, build_type: String) -> Result<(), VeldError> {
    let cwd = std::env::current_dir().map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                ".".into(),
                format!("Cannot determine current directory: {}", e),
            ),
            veld_error::ErrorContext::new(),
        )
    })?;

    let manifest = load_manifest(&cwd)?;

    let lock_path = cwd.join(FILE_LOCK_NAME);
    if !lock_path.exists() {
        eprintln!(
            "{} {} not found — run {} first",
            "!".yellow().bold(),
            FILE_LOCK_NAME.yellow(),
            "veld install".bold()
        );
        return Ok(());
    }

    let lock = Lockfile::read(&lock_path)?;

    generate_cmake_from_graph(&cwd, &manifest, &lock)?;

    println!(
        "{} CMake files generated. To build your project:",
        "✓".green().bold()
    );
    println!("  cmake -B build -DCMAKE_BUILD_TYPE={}", build_type);
    println!("  cmake --build build");

    Ok(())
}

// ─────────────────────────────────────────────
// TREE
// ─────────────────────────────────────────────

fn cmd_tree() -> Result<(), VeldError> {
    let cwd = std::env::current_dir().map_err(|e| {
        VeldError::new(
            veld_error::ErrorCode::FileError(
                ".".into(),
                format!("Cannot determine current directory: {}", e),
            ),
            veld_error::ErrorContext::new(),
        )
    })?;

    let manifest = load_manifest(&cwd)?;

    let lock_path = cwd.join(FILE_LOCK_NAME);
    if !lock_path.exists() {
        eprintln!(
            "{} {} not found — run {} first",
            "!".yellow().bold(),
            FILE_LOCK_NAME.yellow(),
            "veld install".bold()
        );
        return Ok(());
    }

    let lock = Lockfile::read(&lock_path)?;

    println!(
        "{} v{}",
        manifest.package.name.bold(),
        manifest.package.version
    );

    // Build a map for quick lookup
    let dep_map: std::collections::HashMap<String, &LockedPackage> =
        lock.packages.iter().map(|p| (p.name.clone(), p)).collect();

    // Show direct dependencies first
    let direct = lock.direct_dependencies();
    for (i, dep) in direct.iter().enumerate() {
        let is_last = i == direct.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        let ref_label = dep
            .commit
            .as_ref()
            .map(|c| c[..8.min(c.len())].to_string())
            .or_else(|| dep.path.as_ref().map(|p| format!("path: {}", p)))
            .unwrap_or_else(|| "local".to_string());

        println!(
            "{}{} v{} [{}]",
            prefix,
            dep.name.green(),
            dep.version,
            ref_label.dimmed()
        );

        // Show transitive dependencies
        print_transitive_deps(&dep_map, &dep.name, &child_prefix);
    }

    if direct.is_empty() {
        println!("  (no dependencies)");
    }

    Ok(())
}

fn print_transitive_deps(
    dep_map: &std::collections::HashMap<String, &LockedPackage>,
    parent_name: &str,
    prefix: &str,
) {
    // Find packages that have this parent in their required_by
    let children: Vec<_> = dep_map
        .values()
        .filter(|p| p.required_by.contains(&parent_name.to_string()))
        .collect();

    for (i, child) in children.iter().enumerate() {
        let is_last = i == children.len() - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = format!("{}{}", prefix, if is_last { "    " } else { "│   " });

        let ref_label = child
            .commit
            .as_ref()
            .map(|c| c[..8.min(c.len())].to_string())
            .or_else(|| child.path.as_ref().map(|p| format!("path: {}", p)))
            .unwrap_or_else(|| "local".to_string());

        println!(
            "{}{}{} v{} [{}]",
            prefix,
            connector,
            child.name.green(),
            child.version,
            ref_label.dimmed()
        );

        print_transitive_deps(dep_map, &child.name, &child_prefix);
    }
}
