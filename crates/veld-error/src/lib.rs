mod error_context;
mod errorcodes;

pub use error_context::*;
pub use errorcodes::*;
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
