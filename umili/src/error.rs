use std::error::Error;
use std::fmt::Display;

/// Error types for mutation operations.
#[derive(Debug)]
pub enum MutationError {
    /// The specified path does not exist.
    IndexError { path: String },
    /// Operation could not be performed at the specified path.
    OperationError { path: String },
}

impl PartialEq for MutationError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::IndexError { path: a }, Self::IndexError { path: b }) => a == b,
            (Self::OperationError { path: a }, Self::OperationError { path: b }) => a == b,
            _ => false,
        }
    }
}

impl Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexError { path } => write!(f, "index error at {path}"),
            Self::OperationError { path } => write!(f, "operation error at {path}"),
        }
    }
}

impl Error for MutationError {}
