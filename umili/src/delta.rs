use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};

use crate::change::{BatchTree, Change, Error};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Delta {
    p: Option<String>,
    o: Option<DeltaKind>,
    v: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeltaKind {
    #[default]
    SET,
    #[cfg(feature = "append")]
    APPEND,
    BATCH,
    HISTORY,
}

#[derive(Debug, Clone, Default)]
pub struct DeltaHistory {
    p: String,
    o: DeltaKind,
}

impl DeltaHistory {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn batch_encode(&mut self, changes: Vec<Change>) -> Result<Delta, Error> {
        let mut tree = BatchTree::new();
        for change in changes {
            tree.load(change)?;
        }
        self.encode(tree.dump())
    }
}
