use std::error::Error;
use std::fmt::Display;

/// Error types for mutation operations.
#[derive(Debug)]
pub enum UmiliError {
    /// Error during JSON serialization or deserialization.
    JsonError(serde_json::Error),
    /// The specified path does not exist.
    IndexError { path: Vec<String> },
    /// Operation could not be performed at the specified path.
    OperationError { path: Vec<String> },
}

impl Display for UmiliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsonError(inner) => inner.fmt(f),
            Self::IndexError { path } => {
                // use `Debug` for quotes around path
                write!(f, "path {:?} does not exist", path.join("/"))
            }
            Self::OperationError { path } => {
                // use `Debug` for quotes around path
                write!(f, "operation could not be performed at {:?}", path.join("/"))
            }
        }
    }
}

impl Error for UmiliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::JsonError(inner) => Some(inner),
            _ => None,
        }
    }
}
