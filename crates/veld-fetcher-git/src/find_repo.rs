use veld_error::{VeldError, VeldResult};

/// Check if url exists
pub fn repo_exists(url: String) -> VeldResult<bool> {
    let result = reqwest::blocking::get(url);
    let response = match result {
        Ok(r) => r,
        Err(_) => return Ok(false),
    };
    Ok(response.status().as_u16() >= 400u16)
}
