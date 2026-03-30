use semver::VersionReq;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manifest {
    pub package: PackageMetadata,
    pub profiles: BTreeMap<String, ProfileConfig>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageMetadata {
    pub name: String,
    pub version: semver::Version,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProfileConfig {
    pub cxx_std: CxxStandard,
    pub build: BuildType,
    pub optimization: OptimizationLevel,
    pub pic: bool,
    pub lto: bool,
    #[serde(default)]
    pub sanitizers: BTreeSet<Sanitizer>,
    pub target: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum CxxStandard {
    C99,
    C11,
    Cxx11,
    Cxx14,
    Cxx17,
    Cxx20,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BuildType {
    Debug,
    Release,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum CompilerFamily {
    Clang,
    Gcc,
    Msvc,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
    Oz,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Sanitizer {
    Address,
    Undefined,
    Thread,
    Memory,
}

// ---------------- Dependency Source Support ----------------
//
// Extend dependencies to support registry-based, git-based, and path-based sources.
//

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitSource {
    pub repo: String, // Git repository URL
    pub branch: Option<String>,
    pub tag: Option<String>,
    pub commit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subdir: Option<String>, // Optional path inside the repo
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_depth: Option<u32>, // Optional shallow fetch depth
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathSource {
    pub path: String, // Local filesystem path to the dependency
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DependencySource {
    #[serde(rename = "version")]
    Registry { version: VersionReq },
    #[serde(rename = "git")]
    Git(GitSource),
    #[serde(rename = "path")]
    Path(PathSource),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySpec {
    pub source: DependencySource,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub features: Vec<String>,
}

// Note:
// - The Manifest dependencies map now stores DependencySpec values.
// - Existing volk: crate users can specify registry dependencies as `name = ">=1.2, <2.0"`
//   or explicitly as `name = { version = ">=1.2, <2.0" }`, depending on your TOML/JSON parsing conventions.
// - Path dependencies can be specified as: `mylib = { path = "../mylib" }`
