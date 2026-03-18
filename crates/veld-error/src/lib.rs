mod error_context;
mod errorcodes;
mod macros;

pub use error_context::*;
pub use errorcodes::*;
pub use macros::*;
use owo_colors::OwoColorize;
use std::fmt;

// ─────────────────────────────────────────────
// VELD ERROR
// ─────────────────────────────────────────────
#[derive(Debug)]
pub struct VeldError {
    pub code: ErrorCode,
    pub context: ErrorContext,
}

pub type VeldResult<T> = std::result::Result<T, VeldError>;

impl VeldError {
    pub fn new(code: ErrorCode, context: ErrorContext) -> Self {
        Self { code, context }
    }
}

impl fmt::Display for VeldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ── línea 1: error[E001]: mensaje ──────────────────────
        writeln!(
            f,
            "{}: {}",
            format!("error[{}]", self.code.code()).red().bold(),
            self.code.message().bold(),
        )?;

        // ── línea 2: --> archivo:línea:columna ─────────────────
        if let Some(file) = &self.context.file {
            let location = match (self.context.line, self.context.column) {
                (Some(l), Some(c)) => format!("{}:{}:{}", file.display(), l, c),
                (Some(l), None) => format!("{}:{}", file.display(), l),
                _ => format!("{}", file.display()),
            };
            writeln!(f, "  {} {}", "-->".cyan(), location)?;
        }

        // ── línea 3+: source line + carets ─────────────────────
        if let (Some(src), Some(line)) = (&self.context.source_line, self.context.line) {
            let line_str = line.to_string();
            let pad = line_str.len();

            writeln!(f, "{}", format!("{:>pad$} |", "", pad = pad).dimmed())?;
            writeln!(
                f,
                "{} {}",
                format!("{:>pad$} |", line_str, pad = pad).cyan(),
                src
            )?;

            if let (Some(raw), Some(col)) = (&self.context.raw_value, self.context.column) {
                let carets = "^".repeat(raw.len());
                writeln!(
                    f,
                    "{}{}",
                    format!("{:>pad$} |  {:col$}", "", "", pad = pad, col = col - 1).dimmed(),
                    carets.yellow(),
                )?;
            }
        }

        // ── hint ───────────────────────────────────────────────
        if let Some(hint) = self.code.hint() {
            writeln!(f, "\n  {} {}", "hint:".yellow().bold(), hint)?;
        }

        Ok(())
    }
}

// ─────────────────────────────────────────────
// DASHBOARD
// ─────────────────────────────────────────────
pub fn display_errors(errors: &[VeldError]) {
    if errors.is_empty() {
        return;
    }

    println!(
        "\n{}",
        "╭───────────────────────────────────────────╮".red()
    );
    println!(
        "{}",
        "│           VELD ERROR DASHBOARD            │".red().bold()
    );

    let error_text = format!("Total errors found: {}", errors.len());
    let padding = " ".repeat(41_usize.saturating_sub(error_text.len()));

    println!(
        "{} {}{} {}",
        "│".red(),
        error_text.cyan().bold(),
        padding,
        "│".red()
    );
    println!(
        "{}\n",
        "╰───────────────────────────────────────────╯".red()
    );

    for (i, error) in errors.iter().enumerate() {
        println!("{}", error);
        if i < errors.len() - 1 {
            println!("{}", "·".repeat(45).dimmed());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_error_formatting_basic() {
        let err = VeldError::new(
            ErrorCode::InvalidBuildType("super-mega-fast".to_string()),
            ErrorContext::new(),
        );

        let formatted = format!("{}", err);
        assert!(formatted.contains("error[E006]"));
        assert!(formatted.contains("super-mega-fast"));
        assert!(formatted.contains("is not a valid build type"));
        assert!(formatted.contains("hint:"));
    }

    #[test]
    fn test_error_formatting_with_context() {
        let ctx = ErrorContext::new()
            .with_file(PathBuf::from("veld.toml"))
            .with_location(10, 5)
            .with_source_line("    build = \"super-mega-fast\"")
            .with_value("super-mega-fast");

        let err = VeldError::new(
            ErrorCode::InvalidBuildType("super-mega-fast".to_string()),
            ctx,
        );

        let formatted = format!("{}", err);
        // Check for error code and message
        assert!(formatted.contains("error[E006]"));
        // Check for location string
        assert!(formatted.contains("-->"));
        assert!(formatted.contains("veld.toml:10:5"));
        // Check for source line and carets
        assert!(formatted.contains("build = \"super-mega-fast\""));
        assert!(formatted.contains("^^^^^^^^^^^^^^^"));
    }

    #[test]
    fn test_display_errors() {
        // Just verify it doesn't panic on empty
        display_errors(&[]);

        // Verify it doesn't panic on a populated list
        let err1 = VeldError::new(
            ErrorCode::InvalidBuildType("magic".to_string()),
            ErrorContext::new(),
        );
        let err2 = VeldError::new(
            ErrorCode::MissingField("name".to_string()),
            ErrorContext::new().with_file(PathBuf::from("veld.toml")),
        );

        display_errors(&[err1, err2]);
    }
}
