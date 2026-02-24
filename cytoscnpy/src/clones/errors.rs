/// Clone detection error
#[derive(Debug)]
pub enum CloneError {
    /// Error during parsing
    ParseError(String),
    /// IO error
    IoError(std::io::Error),
}

impl std::fmt::Display for CloneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
        }
    }
}

impl std::error::Error for CloneError {}
