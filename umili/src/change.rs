use serde::Serialize;
use serde_json::{to_value, Value};

use crate::error::Error;

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

    pub fn apply(self, mut value: &mut Value, prefix: &str) -> Result<(), Error> {
        let mut parts = split_path(self.path());
        let mut prefix = prefix.to_string();
        while let Some(key) = parts.pop() {
            prefix += key;
            prefix += "/";
            match json_index(value, &key, parts.len() == 0 && matches!(self, Self::SET { .. })) {
                Some(v) => value = v,
                None => {
                    prefix.pop();
                    return Err(Error::IndexError { path: prefix })
                },
            }
        }
        match self {
            Self::SET { v, .. } => {
                *value = v;
            },
            #[cfg(feature = "append")]
            Self::APPEND { v, .. } => {
                if !append(value, v) {
                    prefix.pop();
                    return Err(Error::OperationError { path: prefix })
                }
            },
            Self::BATCH { v, .. } => {
                for delta in v {
                    delta.apply(value, &prefix)?;
                }
            },
        }
        Ok(())
    }
}

fn json_index<'v>(value: &'v mut Value, key: &str, insert: bool) -> Option<&'v mut Value> {
    match value {
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
    }
}

#[cfg(feature = "append")]
pub(crate) fn append(lhs: &mut Value, rhs: Value) -> bool {
    match (lhs, rhs) {
        (Value::String(lhs), Value::String(rhs)) => {
            *lhs += &rhs;
        },
        (Value::Array(lhs), Value::Array(rhs)) => {
            lhs.extend(rhs);
        },
        _ => return false,
    }
    true
}

pub(crate) fn split_path(path: &str) -> Vec<&str> {
    if path.is_empty() {
        vec![]
    } else {
        path.split('/').rev().collect()
    }
}

pub(crate) fn concat_path(key: String, path: &str) -> String {
    if path.is_empty() {
        key
    } else {
        format!("{}/{}", key, path)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        Change::set("", json!({})).unwrap().apply(&mut value, "").unwrap();
        assert_eq!(value, json!({}));

        let mut value = json!({});
        Change::set("a", 1).unwrap().apply(&mut value, "").unwrap();
        assert_eq!(value, json!({"a": 1}));

        let mut value = json!({"a": 1});
        Change::set("a", 2).unwrap().apply(&mut value, "").unwrap();
        assert_eq!(value, json!({"a": 2}));

        let error = Change::set("a/b", 3).unwrap().apply(&mut json!({}), "").unwrap_err();
        assert_eq!(error, Error::IndexError { path: "a".to_string() });

        let error = Change::set("a/b", 3).unwrap().apply(&mut json!({"a": 1}), "").unwrap_err();
        assert_eq!(error, Error::IndexError { path: "a/b".to_string() });

        let error = Change::set("a/b", 3).unwrap().apply(&mut json!({"a": []}), "").unwrap_err();
        assert_eq!(error, Error::IndexError { path: "a/b".to_string() });

        let mut value = json!({"a": {}});
        Change::set("a/b", 3).unwrap().apply(&mut value, "").unwrap();
        assert_eq!(value, json!({"a": {"b": 3}}));
    }

    #[test]
    fn apply_append() {
        let mut value = json!("2");
        Change::append("", "34").unwrap().apply(&mut value, "").unwrap();
        assert_eq!(value, json!("234"));

        let mut value = json!([2]);
        Change::append("", ["3", "4"]).unwrap().apply(&mut value, "").unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        let error = Change::append("", 3).unwrap().apply(&mut json!(""), "").unwrap_err();
        assert_eq!(error, Error::OperationError { path: "".to_string() });

        let error = Change::append("", "3").unwrap().apply(&mut json!({}), "").unwrap_err();
        assert_eq!(error, Error::OperationError { path: "".to_string() });

        let error = Change::append("", "3").unwrap().apply(&mut json!([]), "").unwrap_err();
        assert_eq!(error, Error::OperationError { path: "".to_string() });

        let error = Change::append("", [3]).unwrap().apply(&mut json!(""), "").unwrap_err();
        assert_eq!(error, Error::OperationError { path: "".to_string() });
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Change::batch("", vec![]).apply(&mut value, "").unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": {}}}}));

        let mut value = json!({"a": {"b": {"c": "1"}}});
        let error = Change::batch("a/d", vec![]).apply(&mut value, "").unwrap_err();
        assert_eq!(error, Error::IndexError { path: "a/d".to_string() });

        let mut value = json!({"a": {"b": {"c": "1"}}});
        Change::batch("a", vec![
            Change::append("b/c", "2").unwrap(),
            Change::set("d", 3).unwrap(),
        ]).apply(&mut value, "").unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": "12"}, "d": 3}}));
    }
}
