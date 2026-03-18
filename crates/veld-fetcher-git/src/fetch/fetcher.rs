use std::path::{Path, PathBuf};

use veld_error::VeldResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRef {
    Tag(String),
    Branch(String),
    Commit(String),
}

impl std::fmt::Display for SourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tag(s) => write!(f, "tag:{s}"),
            Self::Branch(s) => write!(f, "branch:{s}"),
            Self::Commit(s) => write!(f, "commit:{s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FetchedSource {
    pub path: PathBuf,
    /// Resoved 40-char SHA-1 - the normative value for the lockfile
    pub commit: String,
    /// Original symbolic ref (tag/branch name)
    pub symbolic_ref: Option<String>,
}

pub trait GitFetcher {
    fn fetch<S: AsRef<str>, P: AsRef<Path>>(
        &self,
        url: S,
        source_ref: &SourceRef,
        dest: P,
    ) -> VeldResult<FetchedSource>;
}
