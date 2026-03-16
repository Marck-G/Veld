use veld_error::{ErrorCode, ErrorContext, VeldError};

use crate::structs::GitSource;
impl GitSource {
    pub fn validate(&self, pkg_name: &str, ctx: &ErrorContext) -> Result<(), VeldError> {
        // 1. URL no vacía
        if self.repo.trim().is_empty() {
            return Err(VeldError::new(
                ErrorCode::GitEmptyUrl(pkg_name.to_string()),
                ctx.clone(),
            ));
        }

        // 2. Esquema válido
        if !self.repo.starts_with("https://")
            && !self.repo.starts_with("http://")
            && !self.repo.starts_with("ssh://")
            && !self.repo.starts_with("git@")
        {
            return Err(VeldError::new(
                ErrorCode::GitInvalidUrl(pkg_name.to_string(), self.repo.clone()),
                ctx.clone(),
            ));
        }

        // 3. Exactamente un rev
        match (&self.branch, &self.tag, &self.commit) {
            (Some(_), None, None) => {}
            (None, Some(_), None) => {}
            (None, None, Some(_)) => {}
            (None, None, None) => {
                return Err(VeldError::new(
                    ErrorCode::GitMissingRev(pkg_name.to_string()),
                    ctx.clone(),
                ));
            }
            _ => {
                return Err(VeldError::new(
                    ErrorCode::GitConflictingRevs(pkg_name.to_string()),
                    ctx.clone(),
                ));
            }
        }

        // 4. Commit: exactamente 40 hex chars
        if let Some(ref hash) = self.commit {
            if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(VeldError::new(
                    ErrorCode::GitInvalidCommit(pkg_name.to_string(), hash.clone()),
                    ctx.clone(),
                ));
            }
        }

        // 5. fetch_depth >= 1
        if let Some(depth) = self.fetch_depth {
            if depth == 0 {
                return Err(VeldError::new(
                    ErrorCode::GitInvalidFetchDepth(pkg_name.to_string()),
                    ctx.clone(),
                ));
            }
        }

        Ok(())
    }
}
