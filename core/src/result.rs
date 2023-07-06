use std::fmt::Display;

#[derive(Debug)]
pub enum SourceLocation {
    TextFile { filename: String, line: u32 },
    JobFile { filename: String, path: String },
}

impl SourceLocation {
    #[track_caller]
    pub fn here() -> Self {
        let caller = std::panic::Location::caller();
        SourceLocation::TextFile { filename: caller.file().to_string(), line: caller.line() }
    }
}

impl Display for SourceLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceLocation::TextFile { filename, line } => write!(f, "{filename}: {line}"),
            SourceLocation::JobFile { filename, path } => write!(f, "{filename}: {path}"),
        }
    }
}

#[derive(Debug)]
pub struct Error {
    message: String,
    source: SourceLocation,
}

impl Error {
    pub fn new<M : Into<String>>(message: M, source: SourceLocation) -> Self {
        return Self { message: message.into(), source };
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source(&self) -> &SourceLocation {
        &self.source
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error occured: {} (at {})", self.message, self.source)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
