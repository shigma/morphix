use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};

use crate::batch::Batch;
use crate::change::Change;
use crate::error::Error;

/// A structured change with optional `p` and `o` fields.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Delta {
    p: Option<String>,
    o: Option<DeltaKind>,
    v: Value,
}

/// The kind of a delta operation.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeltaKind {
    #[default]
    SET,
    #[cfg(feature = "append")]
    APPEND,
    BATCH,
    HISTORY,
}

/// A history of delta operations, used for caching `p` and `o` fields.
#[derive(Debug, Clone, Default)]
pub struct DeltaHistory {
    p: String,
    o: DeltaKind,
}

impl DeltaHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode a `Delta` into a `Change`.
    pub fn decode(&mut self, delta: Delta) -> Result<Change, Error> {
        if let Some(p) = delta.p {
            self.p = p;
        }
        if let Some(o) = delta.o {
            self.o = o;
        }
        Ok(match self.o {
            DeltaKind::SET => Change::SET { p: self.p.clone(), v: delta.v },
            #[cfg(feature = "append")]
            DeltaKind::APPEND => Change::APPEND { p: self.p.clone(), v: delta.v },
            DeltaKind::BATCH => {
                let mut history = Self::new();
                let Value::Array(deltas) = delta.v else {
                    panic!("invalid batch operation");
                };
                let changes = deltas
                    .into_iter()
                    .map(|delta| -> Result<Change, Error> {
                        history.decode(from_value(delta).map_err(Error::JsonError)?)
                    })
                    .collect::<Result<_, _>>()?;
                Change::batch(self.p.clone(), changes)
            },
            DeltaKind::HISTORY => {
                self.o = from_value(delta.v).map_err(Error::JsonError)?;
                Change::batch(self.p.clone(), vec![])
            },
        })
    }

    /// Encode a `Change` into a `Delta`.
    pub fn encode(&mut self, change: Change) -> Result<Delta, Error> {
        let (p, o, v) = match change {
            Change::SET { p, v } => (p, DeltaKind::SET, v),
            #[cfg(feature = "append")]
            Change::APPEND { p, v } => (p, DeltaKind::APPEND, v),
            Change::BATCH { p, v } => {
                let mut history = Self::new();
                let deltas = v
                    .into_iter()
                    .map(|change| -> Result<Delta, Error> {
                        history.encode(change)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                (p, DeltaKind::BATCH, to_value(deltas).map_err(Error::JsonError)?)
            },
        };
        let p = if self.p == p {
            None
        } else {
            self.p = p;
            Some(self.p.clone())
        };
        let o = if self.o == o {
            None
        } else {
            self.o = o;
            Some(self.o.clone())
        };
        Ok(Delta { p, o, v })
    }

    /// Batch encode a list of `Change`s into a `Delta`.
    pub fn batch_encode<I: IntoIterator<Item = Change>>(&mut self, changes: I) -> Result<Delta, Error> {
        let mut batch = Batch::new();
        for change in changes {
            batch.load(change, "")?;
        }
        self.encode(batch.dump())
    }
}
