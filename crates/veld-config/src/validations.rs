use veld_error::{ErrorCode, ErrorContext, VeldError};

use crate::{
    constants::FILE_BUILD_NAME,
    structs::{
        BuildType, DependencySource, DependencySpec, GitSource, Manifest, PackageMetadata,
        ProfileConfig, Sanitizer,
    },
};
impl GitSource {
    pub fn validate(&self, pkg_name: &str, ctx: &ErrorContext) -> Result<(), Vec<VeldError>> {
        // 1. URL no vacía
        if self.repo.trim().is_empty() {
            return Err(vec![VeldError::new(
                ErrorCode::GitEmptyUrl(pkg_name.to_string()),
                ctx.clone(),
            )]);
        }

        // 2. Esquema válido
        if !self.repo.starts_with("https://")
            && !self.repo.starts_with("http://")
            && !self.repo.starts_with("ssh://")
            && !self.repo.starts_with("git@")
        {
            return Err(vec![VeldError::new(
                ErrorCode::GitInvalidUrl(pkg_name.to_string(), self.repo.clone()),
                ctx.clone(),
            )]);
        }

        // 3. Exactamente un rev
        match (&self.branch, &self.tag, &self.commit) {
            (Some(_), None, None) => {}
            (None, Some(_), None) => {}
            (None, None, Some(_)) => {}
            (None, None, None) => {
                return Err(vec![VeldError::new(
                    ErrorCode::GitMissingRev(pkg_name.to_string()),
                    ctx.clone(),
                )]);
            }
            _ => {
                return Err(vec![VeldError::new(
                    ErrorCode::GitConflictingRevs(pkg_name.to_string()),
                    ctx.clone(),
                )]);
            }
        }

        // 4. Commit: exactamente 40 hex chars
        if let Some(ref hash) = self.commit {
            if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(vec![VeldError::new(
                    ErrorCode::GitInvalidCommit(pkg_name.to_string(), hash.clone()),
                    ctx.clone(),
                )]);
            }
        }

        // 5. fetch_depth >= 1
        if let Some(depth) = self.fetch_depth {
            if depth == 0 {
                return Err(vec![VeldError::new(
                    ErrorCode::GitInvalidFetchDepth(pkg_name.to_string()),
                    ctx.clone(),
                )]);
            }
        }

        Ok(())
    }
}

impl PackageMetadata {
    pub fn validate(&self, ctx: &ErrorContext) -> Result<(), Vec<VeldError>> {
        if self.name.is_empty() {
            return Err(vec![VeldError::new(
                ErrorCode::MissingField("name".into()),
                ctx.clone(),
            )]);
        }
        Ok(())
    }
}

impl ProfileConfig {
    pub fn validate(&self, profile_name: &str, ctx: &ErrorContext) -> Result<(), Vec<VeldError>> {
        // 1. target triple tiene formato válido: arch-os-abi
        if let Some(ref triple) = self.target {
            let parts: Vec<&str> = triple.split('-').collect();
            if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
                return Err(vec![VeldError::new(
                    ErrorCode::InvalidTargetTriple(profile_name.to_string(), triple.clone()),
                    ctx.clone(),
                )]);
            }
        }

        // 2. Sanitizers solo tienen sentido en debug
        if !self.sanitizers.is_empty() && matches!(self.build, BuildType::Release) {
            return Err(vec![VeldError::new(
                ErrorCode::SanitizersInRelease(profile_name.to_string()),
                ctx.clone(),
            )]);
        }

        // 3. LTO y sanitizers son incompatibles
        if self.lto && !self.sanitizers.is_empty() {
            return Err(vec![VeldError::new(
                ErrorCode::LtoWithSanitizers(profile_name.to_string()),
                ctx.clone(),
            )]);
        }

        // 4. Thread sanitizer es incompatible con address/memory
        if self.sanitizers.contains(&Sanitizer::Thread)
            && (self.sanitizers.contains(&Sanitizer::Address)
                || self.sanitizers.contains(&Sanitizer::Memory))
        {
            return Err(vec![VeldError::new(
                ErrorCode::ConflictingSanitizers(profile_name.to_string()),
                ctx.clone(),
            )]);
        }

        Ok(())
    }
}

impl DependencySource {
    pub fn validate(&self, pkg: String, ctx: &ErrorContext) -> Result<(), Vec<VeldError>> {
        if let Self::Git(e) = self {
            return e.validate(pkg.as_str(), ctx);
        }
        Ok(())
    }
}

impl DependencySpec {
    pub fn validate(&self, name: String, ctx: &ErrorContext) -> Result<(), Vec<VeldError>> {
        self.source.validate(name, ctx)
    }
}

impl Manifest {
    pub fn validate(&self) -> Result<(), Vec<VeldError>> {
        let mut errors: Vec<VeldError> = Vec::new();
        let ctx: ErrorContext = ErrorContext {
            file: Some(FILE_BUILD_NAME.into()),
            line: None,
            column: None,
            raw_value: None,
            source_line: None,
        };

        if let Err(errs) = self.package.validate(&ctx) {
            errors.extend(errs);
        }

        if self.profiles.is_empty() {
            errors.push(VeldError::new(
                ErrorCode::MissingField("profiles".into()),
                ctx.clone(),
            ));
        }

        for (name, profile) in &self.profiles {
            if let Err(errs) = profile.validate(name, &ctx) {
                errors.extend(errs);
            }
        }

        for (pkg_name, dep) in &self.dependencies {
            if let Err(errs) = dep.validate(pkg_name.clone(), &ctx) {
                errors.extend(errs);
            }
        }

        for (name, dep) in &self.dev_dependencies {
            if let Err(errs) = dep.validate(name.clone(), &ctx) {
                errors.extend(errs);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
