use serde::{Deserialize, Serialize};
use serde_json::{from_value, json, to_value, Value};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "o")]
pub enum Change {
    SET { p: String, v: Value },
    #[cfg(feature = "append")]
    APPEND { p: String, v: Value },
    BATCH { p: String, v: Vec<Self> },
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

    pub fn path(&self) -> &str {
        match self {
            Self::BATCH { p, .. } => p,
            Self::SET { p, .. } => p,
            Self::APPEND { p, .. } => p,
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
            #[cfg(feature = "append")]
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
pub struct Delta {
    p: Option<String>,
    o: Option<DeltaKind>,
    v: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Copy, Default)]
pub enum DeltaKind {
    #[default]
    SET,
    #[cfg(feature = "append")]
    APPEND,
    BATCH,
    HISTORY,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct DeltaHistory {
    p: String,
    // TODO: rename to v
    o: DeltaKind,
}

impl DeltaHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, delta: Delta) -> Change {
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
                    history.load(from_value(delta).unwrap())
                }).collect();
                Change::batch(self.p.clone(), changes)
            },
            DeltaKind::HISTORY => {
                self.o = from_value(delta.v).unwrap();
                Change::batch(self.p.clone(), vec![])
            },
        }
    }

    pub fn dump(&mut self, change: Change) -> Delta {
        let (p, o, v) = match change {
            Change::SET { p, v } => (p, DeltaKind::SET, v),
            #[cfg(feature = "append")]
            Change::APPEND { p, v } => (p, DeltaKind::APPEND, v),
            Change::BATCH { p, v } => {
                let mut history = Self::new();
                let deltas = v.into_iter().map(|change| {
                    history.dump(change)
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
