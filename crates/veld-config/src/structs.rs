use semver::VersionReq;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub package: PackageMetadata,
    pub profiles: BTreeMap<String, ProfileConfig>,
    #[serde(default)]
    pub dependencies: BTreeMap<String, VersionReq>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, VersionReq>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageMetadata {
    pub name: String,
    pub version: semver::Version,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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
    Addres,
    Undefined,
    Thread,
    Memory,
}
