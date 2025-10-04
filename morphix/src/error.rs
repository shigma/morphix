use std::error::Error;
use std::fmt::Display;

/// Error types for mutation operations.
#[derive(Debug)]
pub enum ChangeError {
    /// The specified path does not exist.
    IndexError { path: Vec<String> },
    /// Operation could not be performed at the specified path.
    OperationError { path: Vec<String> },
}

impl Display for ChangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexError { path } => {
                // use `Debug` for quotes around path
                write!(f, "path {:?} does not exist", path.join("."))
            }
            Self::OperationError { path } => {
                // use `Debug` for quotes around path
                write!(f, "operation could not be performed at {:?}", path.join("."))
            }
        }
    }
}

impl Error for ChangeError {}
