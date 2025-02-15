use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "o")]
pub enum Delta {
    SET { p: String, v: Value },
    #[cfg(feature = "append")]
    APPEND { p: String, v: Value },
    BATCH { p: String, v: Vec<Delta> },
    HISTORY { p: String, v: DeltaKind },
}

impl Delta {
    pub fn set<P: Into<String>, V: Into<Value>>(p: P, v: V) -> Self {
        Delta::SET { p: p.into(), v: v.into() }
    }

    #[cfg(feature = "append")]
    pub fn append<P: Into<String>, V: Into<Value>>(p: P, v: V) -> Self {
        Delta::APPEND { p: p.into(), v: v.into() }
    }

    pub fn batch<P: Into<String>>(p: P, v: Vec<Delta>) -> Self {
        Delta::BATCH { p: p.into(), v }
    }

    pub fn history<P: Into<String>>(p: P, v: DeltaKind) -> Self {
        Delta::HISTORY { p: p.into(), v }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeltaKind {
    SET,
    #[cfg(feature = "append")]
    APPEND,
    BATCH,
    HISTORY,
}
