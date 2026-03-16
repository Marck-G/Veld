pub mod config {
    use std::{collections::BTreeMap, fs::File, io::Write, path::Path};

    use veld_error::{ErrorCode, ErrorContext, VeldError, VeldResult};

    use crate::{
        constants::FILE_BUILD_NAME,
        structs::{Manifest, PackageMetadata},
    };

    pub fn generate<T: AsRef<Path>>(path: T, metadata: PackageMetadata) -> VeldResult<()> {
        let config = Manifest {
            package: metadata,
            profiles: BTreeMap::default(),
            dependencies: BTreeMap::default(),
            dev_dependencies: BTreeMap::default(),
        };
        let file = path.as_ref().join(FILE_BUILD_NAME);
        // file exists
        if file.exists() {
            let context: ErrorContext = ErrorContext {
                file: Some("veld.toml".into()),
                line: None,
                column: None,
                raw_value: None,
                source_line: None,
            };
            let error: VeldError = VeldError::new(
                ErrorCode::FileError(FILE_BUILD_NAME.into(), "File Exists".into()),
                context,
            );
            return Err(error);
        }
        let content = toml::to_string_pretty(&config);
        let mut file: File = match File::create(file) {
            Ok(f) => f,
            Err(e) => {
                let context = ErrorContext {
                    file: Some(FILE_BUILD_NAME.into()),
                    line: None,
                    column: None,
                    raw_value: None,
                    source_line: None,
                };
                let error: VeldError = VeldError::new(
                    ErrorCode::FileError(
                        FILE_BUILD_NAME.into(),
                        format!("Error opening the file: {}", e.to_string()),
                    ),
                    context,
                );
                return Err(error);
            }
        };
        match file.write_all(content.unwrap().as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => {
                let context = ErrorContext {
                    file: Some(FILE_BUILD_NAME.into()),
                    line: None,
                    column: None,
                    raw_value: None,
                    source_line: None,
                };
                let error: VeldError = VeldError::new(
                    ErrorCode::FileError(
                        FILE_BUILD_NAME.into(),
                        format!("Error opening the file: {}", e.to_string()),
                    ),
                    context,
                );
                return Err(error);
            }
        }
    }
}
