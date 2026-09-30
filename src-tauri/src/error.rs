use std::{fmt, io};

pub type Result<T> = std::result::Result<T, CorzaError>;

#[derive(Debug)]
pub enum CorzaError {
    Io(io::Error),
    Parse(String),
    Schema(String),
    NotFound(String),
    Corrupt(String),
    Limit(String),
}

impl fmt::Display for CorzaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, message) = match self {
            Self::Io(error) => return write!(f, "I/O error: {error}"),
            Self::Parse(message) => ("parse error", message),
            Self::Schema(message) => ("schema error", message),
            Self::NotFound(message) => ("not found", message),
            Self::Corrupt(message) => ("corrupt database", message),
            Self::Limit(message) => ("limit exceeded", message),
        };
        write!(f, "{kind}: {message}")
    }
}

impl std::error::Error for CorzaError {}

impl From<io::Error> for CorzaError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
