use serde::{Deserialize, Serialize};
use serde_json::{json, to_value, Value};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "o")]
pub enum Change {
    SET { p: String, v: Value },
    #[cfg(feature = "append")]
    APPEND { p: String, v: Value },
    BATCH { p: String, v: Vec<Self> },
    HISTORY(DeltaHistory),
}

impl Change {
    pub fn set<P: Into<String>, V: Serialize>(p: P, v: V) -> Self {
        Self::SET { p: p.into(), v: to_value(v).unwrap() }
    }

    #[cfg(feature = "append")]
    pub fn append<P: Into<String>, V: Serialize>(p: P, v: V) -> Self {
        Self::APPEND { p: p.into(), v: to_value(v).unwrap() }
    }

    pub fn batch<P: Into<String>>(p: P, v: Vec<Change>) -> Self {
        Self::BATCH { p: p.into(), v }
    }

    pub fn history<P: Into<String>>(p: P, v: DeltaKind) -> Self {
        Self::HISTORY(DeltaHistory { p: p.into(), v })
    }

    pub fn path(&self) -> &str {
        match self {
            Self::BATCH { p, .. } => p,
            Self::SET { p, .. } => p,
            Self::APPEND { p, .. } => p,
            Self::HISTORY(history) => &history.p,
        }
    }

    pub fn apply(self, value: Value) -> Value {
        let mut root = json!({ "__ROOT__": value });
        let mut parts = vec!["__ROOT__".to_string()];
        parts.extend(split_path(Some(self.path())));
        let mut node = &mut root;
        while parts.len() > 1 {
            let key = parts.remove(0);
            node = json_index(node, &key, false);
        }
        let key = parts.remove(0);
        // node[key] = value;
        let mut value = match self {
            Self::SET { .. } => Value::Null,
            _ => json_index(node, &key, false).clone(),
        };
        match self {
            Self::SET { v, .. } => {
                *json_index(node, &key, true) = v;
            },
            Self::APPEND { v, .. } => {
                match (&mut value, v) {
                    (Value::String(lhs), Value::String(rhs)) => {
                        *lhs += &rhs;
                    },
                    (Value::Array(lhs), Value::Array(rhs)) => {
                        lhs.extend(rhs);
                    },
                    _ => panic!("invalid append operation"),
                }
            },
            Self::BATCH { v, .. } => {
                for delta in v {
                    value = delta.apply(value);
                }
            },
            Self::HISTORY(..) => {},
        }
        root["__ROOT__"].take()
    }
}

fn json_index<'v>(node: &'v mut Value, key: &str, insert: bool) -> &'v mut Value {
    match node {
        Value::Array(vec) => {
            let index = key.parse::<usize>().unwrap(); // TODO: handle error
            vec.get_mut(index).unwrap() // TODO: handle error
        },
        Value::Object(map) => {
            match insert {
                true => map.entry(key.to_string()).or_insert(Value::Null),
                false => map.get_mut(key).unwrap(), // TODO: handle error
            }
        },
        _ => panic!("invalid index"),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DeltaHistory {
    p: String,
    v: DeltaKind,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum DeltaKind {
    SET,
    #[cfg(feature = "append")]
    APPEND,
    BATCH,
}

fn concat_path(key: String, path: String) -> String {
    if path.is_empty() {
        key
    } else {
        format!("{}/{}", key, path)
    }
}

fn split_path(path: Option<&str>) -> Vec<String> {
    let Some(path) = path else {
        return vec![]
    };
    if path.is_empty() {
        vec![]
    } else {
        path.split('/').map(|s| s.to_string()).collect()
    }
}
