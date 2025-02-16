use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{json, to_value, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    SET { p: String, v: Value },
    #[cfg(feature = "append")]
    APPEND { p: String, v: Value },
    BATCH { p: String, v: Vec<Self> },
}

impl Change {
    pub fn set<P: Into<String>, V: Serialize>(p: P, v: V) -> Result<Self, serde_json::Error> {
        Ok(Self::SET { p: p.into(), v: to_value(v)? })
    }

    #[cfg(feature = "append")]
    pub fn append<P: Into<String>, V: Serialize>(p: P, v: V) -> Result<Self, serde_json::Error> {
        Ok(Self::APPEND { p: p.into(), v: to_value(v)? })
    }

    pub fn batch<P: Into<String>>(p: P, v: Vec<Change>) -> Self {
        Self::BATCH { p: p.into(), v }
    }

    pub fn path(&self) -> &String {
        match self {
            Self::BATCH { p, .. } => p,
            Self::SET { p, .. } => p,
            #[cfg(feature = "append")]
            Self::APPEND { p, .. } => p,
        }
    }

    pub fn path_mut(&mut self) -> &mut String {
        match self {
            Self::BATCH { p, .. } => p,
            Self::SET { p, .. } => p,
            #[cfg(feature = "append")]
            Self::APPEND { p, .. } => p,
        }
    }

    pub fn apply(self, value: Value) -> Result<Value, Error> {
        let mut root = json!({ "__ROOT__": value });
        let mut parts = vec!["__ROOT__".to_string()];
        parts.extend(split_path(Some(self.path())));
        let mut node = &mut root;
        while parts.len() > 1 {
            let key = parts.remove(0);
            node = json_index(node, &key, false)?;
        }
        let key = parts.remove(0);
        // node[key] = value;
        let mut value = match self {
            Self::SET { .. } => Value::Null,
            _ => json_index(node, &key, false)?.clone(),
        };
        match self {
            Self::SET { v, .. } => {
                *json_index(node, &key, true)? = v;
            },
            #[cfg(feature = "append")]
            Self::APPEND { v, .. } => {
                append(&mut value, v)
            },
            Self::BATCH { v, .. } => {
                for delta in v {
                    value = delta.apply(value)?;
                }
            },
        }
        Ok(root["__ROOT__"].take())
    }
}

#[derive(Debug, Default)]
pub struct BatchTree {
    /// can only be SET or APPEND
    change: Option<Change>,
    children: BTreeMap<String, BatchTree>,
}

impl BatchTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, mut change: Change) -> Result<(), Error> {
        let mut node = self;
        let mut parts = split_path(Some(change.path()));
        if let Some(Change::SET { v, .. }) = &mut node.change {
            *v = change.apply(v.clone())?; // FIXME: no clone
            return Ok(())
        }
        while parts.len() > 0 {
            let part = parts.remove(0);
            node = node.children.entry(part).or_default();
            *change.path_mut() = parts.join("/");
            if let Some(Change::SET { v, .. }) = &mut node.change {
                *v = change.apply(v.clone())?; // FIXME: no clone
                return Ok(())
            }
        }
        match change {
            Change::SET { .. } => {
                node.change = Some(change);
                node.children.clear();
            },
            #[cfg(feature = "append")]
            Change::APPEND { p, v: rhs } => {
                match &mut node.change {
                    Some(Change::APPEND { v: lhs, .. }) => {
                        append(lhs, rhs)
                    },
                    Some(_) => panic!("invalid append operation"),
                    None => node.change = Some(Change::APPEND { p, v: rhs }),
                }
            },
            Change::BATCH { .. } => unreachable!(),
        }
        Ok(())
    }

    pub fn dump(self) -> Change {
        let mut changes = vec![];
        if let Some(mut change) = self.change {
            *change.path_mut() = String::new();
            changes.push(change);
        }
        for (key, tree) in self.children {
            let mut change = tree.dump();
            *change.path_mut() = concat_path(key, change.path());
            changes.push(change);
        }
        match changes.len() {
            1 => changes.swap_remove(0),
            _ => Change::batch("", changes),
        }
    }
}

pub enum Error {
    JsonError(serde_json::Error),
    IndexError(String),
}

fn json_index<'v>(node: &'v mut Value, key: &str, insert: bool) -> Result<&'v mut Value, Error> {
    match node {
        Value::Array(vec) => {
            key.parse::<usize>().ok().and_then(|index| vec.get_mut(index))
        },
        Value::Object(map) => {
            match insert {
                true => Some(map.entry(key.to_string()).or_insert(Value::Null)),
                false => map.get_mut(key),
            }
        },
        _ => panic!("invalid index"),
    }.ok_or_else(|| Error::IndexError(key.to_string())) // FIXME: full path
}

#[cfg(feature = "append")]
fn append(lhs: &mut Value, rhs: Value) {
    match (lhs, rhs) {
        (Value::String(lhs), Value::String(rhs)) => {
            *lhs += &rhs;
        },
        (Value::Array(lhs), Value::Array(rhs)) => {
            lhs.extend(rhs);
        },
        _ => panic!("invalid append operation"),
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

fn concat_path(key: String, path: &str) -> String {
    if path.is_empty() {
        key
    } else {
        format!("{}/{}", key, path)
    }
}
