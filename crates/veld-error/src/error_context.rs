use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub raw_value: Option<String>,
    pub source_line: Option<String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_file(mut self, file: PathBuf) -> Self {
        self.file = Some(file);
        self
    }
    pub fn with_location(mut self, line: usize, col: usize) -> Self {
        self.line = Some(line);
        self.column = Some(col);
        self
    }
    pub fn with_value(mut self, val: impl Into<String>) -> Self {
        self.raw_value = Some(val.into());
        self
    }
    pub fn with_source_line(mut self, line: impl Into<String>) -> Self {
        self.source_line = Some(line.into());
        self
    }
}
