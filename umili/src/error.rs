use std::fmt::Display;

/// Error type for umili.
#[derive(Debug)]
pub enum Error {
    JsonError(serde_json::Error),
    IndexError {
        path: String,
    },
    OperationError {
        path: String,
    },
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::JsonError(_), Self::JsonError(_)) => false,
            (Self::IndexError { path: a }, Self::IndexError { path: b }) => a == b,
            (Self::OperationError { path: a }, Self::OperationError { path: b }) => a == b,
            _ => false,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsonError(e) => write!(f, "{}", e),
            Self::IndexError { path } => write!(f, "index error at {}", path),
            Self::OperationError { path } => write!(f, "operation error at {}", path),
        }
    }
}
