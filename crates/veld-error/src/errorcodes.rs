use std::path::PathBuf;

pub trait Diagnostic {
    fn code(&self) -> &'static str;
    fn message(&self) -> String;
    fn hint(&self) -> Option<String>;
}

#[derive(Debug, Clone)]
pub enum ErrorCode {
    // Manifest
    InvalidVersion(String),
    InvalidPackageName(String),
    InvalidDependencyVersion(String, String), // (pkg_name, bad_version)
    MissingField(String),
    InvalidCxxStandard(String),
    InvalidBuildType(String),
    InvalidOptLevel(String),
    InvalidSanitizer(String),

    // Profile
    ProfileNotFound(String),
    InvalidTargetTriple(String),

    // IO
    ManifestNotFound(PathBuf),
    ManifestReadError(String),
    ManifestParseError(String),

    // Git sources
    GitMissingRev(String),            // ningún branch/tag/commit especificado
    GitConflictingRevs(String),       // más de uno especificado simultáneamente
    GitEmptyUrl(String),              // repo = ""
    GitInvalidUrl(String, String),    // esquema no reconocido
    GitInvalidCommit(String, String), // hash no es 40 hex chars
    GitInvalidFetchDepth(String),     // fetch_depth = 0
}

impl Diagnostic for ErrorCode {
    fn code(&self) -> &'static str {
        match self {
            // cuando llegue i18n, este match no cambia
            Self::InvalidVersion(_) => "E001",
            Self::InvalidPackageName(_) => "E002",
            Self::InvalidDependencyVersion(..) => "E003",
            Self::MissingField(_) => "E004",
            Self::InvalidCxxStandard(_) => "E005",
            Self::InvalidBuildType(_) => "E006",
            Self::InvalidOptLevel(_) => "E007",
            Self::InvalidSanitizer(_) => "E008",
            Self::ProfileNotFound(_) => "E009",
            Self::InvalidTargetTriple(_) => "E010",
            Self::ManifestNotFound(_) => "E011",
            Self::ManifestReadError(_) => "E012",
            Self::ManifestParseError(_) => "E013",
            // Git sources
            Self::GitMissingRev(_) => "E014",
            Self::GitConflictingRevs(_) => "E015",
            Self::GitEmptyUrl(_) => "E016",
            Self::GitInvalidUrl(..) => "E017",
            Self::GitInvalidCommit(..) => "E018",
            Self::GitInvalidFetchDepth(_) => "E019",
        }
    }

    fn message(&self) -> String {
        // ┌──────────────────────────────────────────────────────┐
        // │ I18N HOOK                                            │
        // │ Reemplazar los strings literales por:                │
        // │   i18n::get(self.code(), lang).format(args)         │
        // └──────────────────────────────────────────────────────┘
        match self {
            Self::InvalidVersion(v) => format!("'{}' is not a valid semver version", v),
            Self::InvalidPackageName(n) => format!("'{}' is not a valid package name", n),
            Self::InvalidDependencyVersion(pkg, v) => {
                format!("'{}' is not a valid version requirement for '{}'", v, pkg)
            }
            Self::MissingField(f) => format!("missing required field '{}'", f),
            Self::InvalidCxxStandard(s) => format!("'{}' is not a valid C/C++ standard", s),
            Self::InvalidBuildType(b) => format!("'{}' is not a valid build type", b),
            Self::InvalidOptLevel(o) => format!("'{}' is not a valid optimization level", o),
            Self::InvalidSanitizer(s) => format!("'{}' is not a valid sanitizer", s),
            Self::ProfileNotFound(p) => format!("profile '{}' not found in veld.toml", p),
            Self::InvalidTargetTriple(t) => format!("'{}' is not a recognized target triple", t),
            Self::ManifestNotFound(p) => format!("no veld.toml found at '{}'", p.display()),
            Self::ManifestReadError(e) => format!("could not read veld.toml: {}", e),
            Self::ManifestParseError(e) => format!("could not parse veld.toml: {}", e),
            Self::GitMissingRev(pkg) => format!(
                "git dependency '{}' must specify 'branch', 'tag', or 'commit'",
                pkg
            ),

            Self::GitConflictingRevs(pkg) => format!(
                "git dependency '{}' specifies more than one of branch/tag/commit",
                pkg
            ),

            Self::GitEmptyUrl(pkg) => format!("git dependency '{}' has an empty 'git' URL", pkg),

            Self::GitInvalidUrl(pkg, url) => {
                format!("'{}' is not a valid git URL for dependency '{}'", url, pkg)
            }

            Self::GitInvalidCommit(pkg, hash) => format!(
                "'{}' is not a valid commit hash for dependency '{}' — expected 40 hex characters",
                hash, pkg
            ),

            Self::GitInvalidFetchDepth(pkg) => format!("'fetch_depth' for '{}' must be >= 1", pkg),
        }
    }

    fn hint(&self) -> Option<String> {
        // ┌──────────────────────────────────────────────────────┐
        // │ I18N HOOK — mismo patrón que message()               │
        // └──────────────────────────────────────────────────────┘
        match self {
            Self::InvalidVersion(_) => {
                Some("expected format: MAJOR.MINOR.PATCH (e.g. 1.0.0)".into())
            }
            Self::InvalidPackageName(_) => {
                Some("package names must be lowercase alphanumeric with hyphens".into())
            }
            Self::InvalidCxxStandard(_) => {
                Some("valid values: c99, c11, cxx11, cxx14, cxx17, cxx20".into())
            }
            Self::InvalidBuildType(_) => Some("valid values: debug, release".into()),
            Self::InvalidOptLevel(_) => Some("valid values: O0, O1, O2, O3, Os, Oz".into()),
            Self::InvalidTargetTriple(_) => {
                Some("example: aarch64-linux-gnu, x86_64-linux-musl".into())
            }
            Self::ManifestNotFound(_) => Some("run 'veld init' to create a new project".into()),
            Self::GitMissingRev(_) => {
                Some("example: { git = \"https://github.com/org/lib\", tag = \"v1.0.0\" }".into())
            }
            Self::GitConflictingRevs(_) => {
                Some("use exactly one of: branch, tag, or commit".into())
            }
            Self::GitInvalidUrl(..) => {
                Some("supported schemes: https://, http://, ssh://, git@".into())
            }
            Self::GitInvalidCommit(..) => {
                Some("use the full 40-character SHA-1 hash (e.g. a1b2c3d4...ef)".into())
            }
            _ => None,
        }
    }
}
