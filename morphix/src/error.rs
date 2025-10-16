use std::error::Error;
use std::fmt::Display;

use crate::Path;

/// Error types for mutation operations.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MutationError {
    /// The specified path does not exist.
    IndexError { path: Path<false> },
    /// Mutation could not be performed at the specified path.
    OperationError { path: Path<false> },
}

impl Display for MutationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexError { path } => {
                write!(f, "path {path} does not exist or is malformed")
            }
            Self::OperationError { path } => {
                write!(f, "operation could not be performed at {path}")
            }
        }
    }
}

impl Error for MutationError {}
