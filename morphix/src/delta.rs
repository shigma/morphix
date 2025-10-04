use serde::{Deserialize, Serialize};
use serde_json::{Value, from_value, to_value};

use crate::batch::Batch;
use crate::change::{Change, Operation};
use crate::error::UmiliError;

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
    Replace,
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

    /// Load a `Delta` into a `Change`.
    pub fn load(&mut self, delta: Delta) -> Result<Change, UmiliError> {
        if let Some(o) = delta.o {
            self.o = o;
        }
        if let Some(p) = delta.p {
            self.p = p;
        }
        let path_rev = self
            .p
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .rev()
            .collect();
        let operation = match self.o {
            DeltaKind::Replace => Operation::Replace(delta.v),
            DeltaKind::Append => Operation::Append(delta.v),
            DeltaKind::Batch => {
                let mut state = Self::new();
                let Value::Array(deltas) = delta.v else {
                    panic!("invalid batch operation"); // TODO: replace with error
                };
                let mut changes = Vec::with_capacity(deltas.len());
                for delta in deltas {
                    changes.push(state.load(from_value(delta).map_err(UmiliError::JsonError)?)?);
                }
                Operation::Batch(changes)
            }
            DeltaKind::State => {
                self.o = from_value(delta.v).map_err(UmiliError::JsonError)?;
                Operation::Batch(vec![])
            }
        };
        Ok(Change { operation, path_rev })
    }

    /// Dump a `Change` into a `Delta`.
    pub fn dump(&mut self, change: Change) -> Result<Delta, UmiliError> {
        let mut p = String::new();
        for key in change.path_rev.iter().rev() {
            if !p.is_empty() {
                p.push('/');
            }
            p.push_str(key);
        }
        let (o, v) = match change.operation {
            Operation::Replace(value) => (DeltaKind::Replace, value),
            Operation::Append(value) => (DeltaKind::Append, value),
            Operation::Batch(changes) => {
                let mut state = Self::new();
                let mut deltas = Vec::with_capacity(changes.len());
                for change in changes {
                    deltas.push(state.dump(change)?);
                }
                (DeltaKind::Batch, to_value(deltas).map_err(UmiliError::JsonError)?)
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
        Ok(Delta { p, o, v })
    }

    /// Batch dump a list of `Change`s into a `Delta`.
    pub fn batch_dump<I: IntoIterator<Item = Change>>(&mut self, changes: I) -> Result<Option<Delta>, UmiliError> {
        let mut batch = Batch::new();
        for change in changes {
            batch.load(change)?;
        }
        Ok(match batch.dump() {
            Some(change) => Some(self.dump(change)?),
            None => None,
        })
    }
}

/// A composer for `Delta` operations, maintaining input and output states.
#[derive(Debug, Default)]
pub struct DeltaComposer {
    input_state: DeltaState,
    output_state: DeltaState,
    batched_changes: Vec<Change>,
}

impl DeltaComposer {
    /// Create a new `DeltaComposer`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a `Delta` into the composer.
    pub fn load_delta(&mut self, delta: Delta) -> Result<(), UmiliError> {
        self.batched_changes.push(self.input_state.load(delta)?);
        Ok(())
    }

    /// Load a `DeltaState` into the composer.
    pub fn load_delta_state(&mut self, state: DeltaState) {
        self.input_state = state;
    }

    /// Dump the composed `Delta`.
    pub fn dump_delta(&mut self) -> Result<Option<Delta>, UmiliError> {
        self.output_state.batch_dump(self.batched_changes.drain(..))
    }

    /// Dump the current `DeltaState`.
    pub fn dump_delta_state(self) -> DeltaState {
        self.output_state
    }
}
