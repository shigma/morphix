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
