use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{to_value, Value};

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

    pub fn apply(self, value: &mut Value) -> Result<(), Error> {
        let mut parts = split_path(self.path());
        let mut node = value;
        while let Some(part) = parts.pop() {
            node = json_index(node, &part, parts.len() == 0 && matches!(self, Self::SET { .. }))?;
        }
        match self {
            Self::SET { v, .. } => {
                *node = v;
            },
            #[cfg(feature = "append")]
            Self::APPEND { v, .. } => {
                append(node, v)?
            },
            Self::BATCH { v, .. } => {
                for delta in v {
                    delta.apply(node)?;
                }
            },
        }
        Ok(())
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
        let mut parts = split_path(change.path());
        if let Some(Change::SET { v, .. }) = &mut node.change {
            change.apply(v)?;
            return Ok(())
        }
        while let Some(part) = parts.pop() {
            node = node.children.entry(part).or_default();
            *change.path_mut() = parts.join("/");
            if let Some(Change::SET { v, .. }) = &mut node.change {
                change.apply(v)?;
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
                        append(lhs, rhs)?
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

#[derive(Debug)]
pub enum Error {
    JsonError(serde_json::Error),
    IndexError(String),
    OperationError(),
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
        _ => None,
    }.ok_or_else(|| Error::IndexError(key.to_string())) // FIXME: full path
}

#[cfg(feature = "append")]
fn append(lhs: &mut Value, rhs: Value) -> Result<(), Error> {
    Ok(match (lhs, rhs) {
        (Value::String(lhs), Value::String(rhs)) => {
            *lhs += &rhs;
        },
        (Value::Array(lhs), Value::Array(rhs)) => {
            lhs.extend(rhs);
        },
        _ => return Err(Error::OperationError()),
    })
}

fn split_path(path: &str) -> Vec<String> {
    if path.is_empty() {
        vec![]
    } else {
        path.split('/').map(|s| s.to_string()).rev().collect()
    }
}

fn concat_path(key: String, path: &str) -> String {
    if path.is_empty() {
        key
    } else {
        format!("{}/{}", key, path)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        Change::set("", json!({})).unwrap().apply(&mut value).unwrap();
        assert_eq!(value, json!({}));

        let mut value = json!({});
        Change::set("a", 1).unwrap().apply(&mut value).unwrap();
        assert_eq!(value, json!({"a": 1}));

        let mut value = json!({"a": 1});
        Change::set("a", 2).unwrap().apply(&mut value).unwrap();
        assert_eq!(value, json!({"a": 2}));

        Change::set("a/b", 3).unwrap().apply(&mut json!({})).unwrap_err();
        Change::set("a/b", 3).unwrap().apply(&mut json!({"a": 1})).unwrap_err();
        Change::set("a/b", 3).unwrap().apply(&mut json!({"a": []})).unwrap_err();

        let mut value = json!({"a": {}});
        Change::set("a/b", 3).unwrap().apply(&mut value).unwrap();
        assert_eq!(value, json!({"a": {"b": 3}}));
    }

    #[test]
    fn apply_append() {
        let mut value = json!("2");
        Change::append("", "34").unwrap().apply(&mut value).unwrap();
        assert_eq!(value, json!("234"));

        let mut value = json!([2]);
        Change::append("", ["3", "4"]).unwrap().apply(&mut value).unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        Change::append("", 3).unwrap().apply(&mut json!("")).unwrap_err();
        Change::append("", "3").unwrap().apply(&mut json!({})).unwrap_err();
        Change::append("", "3").unwrap().apply(&mut json!([])).unwrap_err();
        Change::append("", [3]).unwrap().apply(&mut json!("")).unwrap_err();
    }
}
