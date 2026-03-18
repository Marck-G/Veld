/// Macro to create a VeldError with an error code.
/// 
/// The macro accepts either one or two arguments:
/// - With one argument: `veld_error!(ErrorCode::InvalidVersion("1.0".to_string()))`
/// - With two arguments: `veld_error!(ErrorCode::InvalidVersion("1.0".to_string()), ErrorContext::new().with_file(...))`
/// 
/// # Example
/// ```rust
/// use veld_error::{ErrorCode, ErrorContext, VeldError};
/// 
/// // Without context
/// let err = veld_error!(ErrorCode::InvalidVersion("1.0".to_string()));
/// 
/// // With context
/// let err = veld_error!(ErrorCode::InvalidVersion("1.0".to_string()),
///                       ErrorContext::new()
///                           .with_file(PathBuf::from("veld.toml"))
///                           .with_location(10, 5)
///                           .with_source_line("    version = \"1.0\""));
/// ```
#[macro_export]
macro_rules! veld_error {
    ($code:expr) => {
        $crate::VeldError::new($code, $crate::ErrorContext::new())
    };
    ($code:expr, $context:expr) => {
        $crate::VeldError::new($code, $context)
    };
}

/// Macro to convert errors to VeldError using map_err.
/// 
/// This macro takes an expression that returns a Result and transforms any error
/// into a VeldError using the provided error code.
/// 
/// The macro accepts either two or three arguments:
/// - With two arguments: `map_veld_error!(expr, ErrorCode::...)`
/// - With three arguments: `map_veld_error!(expr, ErrorCode::..., ErrorContext::new().with_file(...))`
/// 
/// # Arguments
/// - `$exp`: The expression that returns a Result
/// - `$code`: The ErrorCode to use when creating the VeldError
/// - `$context`: (Optional) The ErrorContext to attach to the VeldError
/// 
/// # Example
/// ```rust
/// use veld_error::{ErrorCode, ErrorContext, VeldError};
/// 
/// // Without context
/// let result = map_veld_error!(std::fs::read_to_string("file.txt"), 
///     ErrorCode::ManifestReadError("file not found".to_string()));
/// 
/// // With context
/// let ctx = ErrorContext::new()
///     .with_file(PathBuf::from("veld.toml"))
///     .with_location(10, 5);
///     
/// let result = map_veld_error!(std::fs::read_to_string("file.txt"), 
///     ErrorCode::ManifestReadError("file not found".to_string()), 
///     ctx);
/// ```
#[macro_export]
macro_rules! map_veld_error {
    ($exp:expr, $code:expr) => {
        $exp.map_err(|_err| $crate::VeldError::new($code, $crate::ErrorContext::new()))
    };
    ($exp:expr, $code:expr, $context:expr) => {
        $exp.map_err(|_err| $crate::VeldError::new($code, $context))
    };
}

/// Macro to convert errors to VeldError with a custom error mapping function.
/// 
/// This macro takes an expression that returns a Result and transforms any error
/// into a VeldError using the provided function.
/// 
/// # Arguments
/// - `$exp`: The expression that returns a Result
/// - `$fnt`: A function that takes the original error and returns a VeldError
/// 
/// # Example
/// ```rust
/// use veld_error::{ErrorCode, ErrorContext, VeldError};
/// 
/// let result = map_veld_errin!(std::fs::read_to_string("file.txt"), |err| {
///     VeldError::new(
///         ErrorCode::ManifestReadError(err.to_string()),
///         ErrorContext::new().with_file(PathBuf::from("veld.toml"))
///     )
/// });
/// ```
#[macro_export]
macro_rules! map_veld_errin {
    ($exp:expr, $fnt:expr) => {
        $exp.map_err(|err| $fnt(err))
    };
}