use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};

use super::batch::Batch;
use super::change::Change;

/// A structured change with optional `p` and `o` fields.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Delta {
    p: Option<String>,
    o: Option<DeltaKind>,
    v: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeltaKind {
    #[default]
    Set,
    #[cfg(feature = "append")]
    Append,
    Batch,
    State,
}

/// State of `Delta` operations, used for caching `p` and `o` fields.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DeltaState {
    p: String,
    o: DeltaKind,
}

impl DeltaState {
    /// Create a new `DeltaState`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode a `Delta` into a `Change`.
    pub fn decode(&mut self, delta: Delta) -> Change {
        if let Some(p) = delta.p {
            self.p = p;
        }
        if let Some(o) = delta.o {
            self.o = o;
        }
        match self.o {
            DeltaKind::Set => Change::Set {
                p: self.p.clone(),
                v: delta.v,
            },
            #[cfg(feature = "append")]
            DeltaKind::Append => Change::Append {
                p: self.p.clone(),
                v: delta.v,
            },
            DeltaKind::Batch => {
                let mut state = Self::new();
                let Value::Array(deltas) = delta.v else {
                    panic!("invalid batch operation");
                };
                let changes = deltas
                    .into_iter()
                    .map(|delta| state.decode(from_value(delta).unwrap()))
                    .collect::<Vec<_>>();
                Change::batch(self.p.clone(), changes)
            }
            DeltaKind::State => {
                self.o = from_value(delta.v).unwrap();
                Change::batch(self.p.clone(), vec![])
            }
        }
    }

    /// Encode a `Change` into a `Delta`.
    pub fn encode(&mut self, change: Change) -> Delta {
        let (p, o, v) = match change {
            Change::Set { p, v } => (p, DeltaKind::Set, v),
            #[cfg(feature = "append")]
            Change::Append { p, v } => (p, DeltaKind::Append, v),
            Change::Batch { p, v } => {
                let mut state = Self::new();
                let deltas = v.into_iter().map(|change| state.encode(change)).collect::<Vec<_>>();
                (p, DeltaKind::Batch, to_value(deltas).unwrap())
            }
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
            Some(self.o)
        };
        Delta { p, o, v }
    }

    /// Batch encode a list of `Change`s into a `Delta`.
    pub fn batch_encode<I: IntoIterator<Item = Change>>(&mut self, changes: I) -> Option<Delta> {
        let mut batch = Batch::new();
        for change in changes {
            batch.load(change, "").unwrap(); // TODO: remove unwrap
        }
        batch.dump().map(|change| self.encode(change))
    }
}
