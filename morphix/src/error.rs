use std::borrow::Cow;
use std::error::Error;
use std::fmt::Display;

/// Error types for mutation operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationError {
    /// The specified path does not exist.
    IndexError { path: Vec<Cow<'static, str>> },
    /// Mutation could not be performed at the specified path.
    OperationError { path: Vec<Cow<'static, str>> },
}

impl Display for MutationError {
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

impl Error for MutationError {}
