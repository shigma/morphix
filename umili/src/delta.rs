use serde::{Deserialize, Serialize};
use serde_json::{from_value, to_value, Value};

use crate::change::{BatchTree, Change};

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

    pub fn decode(&mut self, delta: Delta) -> Change {
        if let Some(p) = delta.p {
            self.p = p;
        }
        if let Some(o) = delta.o {
            self.o = o;
        }
        match self.o {
            DeltaKind::SET => Change::set(self.p.clone(), delta.v),
            #[cfg(feature = "append")]
            DeltaKind::APPEND => Change::append(self.p.clone(), delta.v),
            DeltaKind::BATCH => {
                let mut history = Self::new();
                let Value::Array(deltas) = delta.v else {
                    panic!("invalid batch operation");
                };
                let changes = deltas.into_iter().map(|delta| {
                    history.decode(from_value(delta).unwrap())
                }).collect();
                Change::batch(self.p.clone(), changes)
            },
            DeltaKind::HISTORY => {
                self.o = from_value(delta.v).unwrap();
                Change::batch(self.p.clone(), vec![])
            },
        }
    }

    pub fn encode(&mut self, change: Change) -> Delta {
        let (p, o, v) = match change {
            Change::SET { p, v } => (p, DeltaKind::SET, v),
            #[cfg(feature = "append")]
            Change::APPEND { p, v } => (p, DeltaKind::APPEND, v),
            Change::BATCH { p, v } => {
                let mut history = Self::new();
                let deltas = v.into_iter().map(|change| {
                    history.encode(change)
                }).collect::<Vec<_>>();
                (p, DeltaKind::BATCH, to_value(deltas).unwrap())
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
        Delta { p, o, v }
    }

    pub fn batch_encode(&mut self, changes: Vec<Change>) -> Delta {
        let mut tree = BatchTree::new();
        for change in changes {
            tree.load(change);
        }
        self.encode(tree.dump())
    }
}
