use serde::Serialize;
use serde_json::{Value, to_value};

use super::error::MutationError;

/// A change in JSON format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// `Set` is the default change for `DerefMut` operations.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// foo.a.b = 1;        // Set "a/b"
    /// foo.num *= 2;       // Set "num"
    /// foo.vec.clear();    // Set "vec"
    /// ```
    ///
    /// If an operation triggers `Append`, no `Set` change is emitted.
    Set { p: String, v: Value },

    /// `Append` represents a `String` or `Vec` append operation.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// foo.a.b += "text";          // Append "a/b"
    /// foo.a.b.push_str("text");   // Append "a/b"
    /// foo.vec.push(1);            // Append "vec"
    /// foo.vec.extend(iter);       // Append "vec"
    /// ```
    #[cfg(feature = "append")]
    Append { p: String, v: Value },

    /// `Batch` represents a sequence of changes.
    Batch { p: String, v: Vec<Self> },
}

impl Change {
    /// Construct a `Set` change.
    pub fn set<P: Into<String>, V: Serialize>(p: P, v: V) -> Result<Self, serde_json::Error> {
        Ok(Self::Set {
            p: p.into(),
            v: to_value(v)?,
        })
    }

    /// Construct an `Append` change.
    #[cfg(feature = "append")]
    pub fn append<P: Into<String>, V: Serialize>(p: P, v: V) -> Result<Self, serde_json::Error> {
        Ok(Self::Append {
            p: p.into(),
            v: to_value(v)?,
        })
    }

    /// Construct a `Batch` change.
    pub fn batch<P: Into<String>>(p: P, v: Vec<Change>) -> Self {
        Self::Batch { p: p.into(), v }
    }

    /// Get the path of the change.
    pub fn path(&self) -> &String {
        match self {
            Self::Batch { p, .. } => p,
            Self::Set { p, .. } => p,
            #[cfg(feature = "append")]
            Self::Append { p, .. } => p,
        }
    }

    /// Get the mutable path of the change.
    pub fn path_mut(&mut self) -> &mut String {
        match self {
            Self::Batch { p, .. } => p,
            Self::Set { p, .. } => p,
            #[cfg(feature = "append")]
            Self::Append { p, .. } => p,
        }
    }

    /// Apply the change to a JSON value.
    pub fn apply(self, mut value: &mut Value, prefix: &str) -> Result<(), MutationError> {
        let mut parts = split_path(self.path());
        let mut prefix = prefix.to_string();
        while let Some(key) = parts.pop() {
            prefix += key;
            prefix += "/";
            match json_index(value, key, parts.is_empty() && matches!(self, Self::Set { .. })) {
                Some(v) => value = v,
                None => {
                    prefix.pop();
                    return Err(MutationError::IndexError { path: prefix });
                }
            }
        }
        match self {
            Self::Set { v, .. } => {
                *value = v;
            }
            #[cfg(feature = "append")]
            Self::Append { v, .. } => {
                if !append(value, v) {
                    prefix.pop();
                    return Err(MutationError::OperationError { path: prefix });
                }
            }
            Self::Batch { v, .. } => {
                for delta in v {
                    delta.apply(value, &prefix)?;
                }
            }
        }
        Ok(())
    }
}

fn json_index<'v>(value: &'v mut Value, key: &str, insert: bool) -> Option<&'v mut Value> {
    match value {
        Value::Array(vec) => key.parse::<usize>().ok().and_then(|index| vec.get_mut(index)),
        Value::Object(map) => match insert {
            true => Some(map.entry(key.to_string()).or_insert(Value::Null)),
            false => map.get_mut(key),
        },
        _ => None,
    }
}

#[cfg(feature = "append")]
pub(crate) fn append(lhs: &mut Value, rhs: Value) -> bool {
    match (lhs, rhs) {
        (Value::String(lhs), Value::String(rhs)) => {
            *lhs += &rhs;
        }
        (Value::Array(lhs), Value::Array(rhs)) => {
            lhs.extend(rhs);
        }
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
    if path.is_empty() { key } else { format!("{key}/{path}") }
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
        assert_eq!(error, MutationError::IndexError { path: "a".to_string() });

        let error = Change::set("a/b", 3)
            .unwrap()
            .apply(&mut json!({"a": 1}), "")
            .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: "a/b".to_string()
            }
        );

        let error = Change::set("a/b", 3)
            .unwrap()
            .apply(&mut json!({"a": []}), "")
            .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: "a/b".to_string()
            }
        );

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
        assert_eq!(error, MutationError::OperationError { path: "".to_string() });

        let error = Change::append("", "3").unwrap().apply(&mut json!({}), "").unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: "".to_string() });

        let error = Change::append("", "3").unwrap().apply(&mut json!([]), "").unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: "".to_string() });

        let error = Change::append("", [3]).unwrap().apply(&mut json!(""), "").unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: "".to_string() });
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Change::batch("", vec![]).apply(&mut value, "").unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": {}}}}));

        let mut value = json!({"a": {"b": {"c": "1"}}});
        let error = Change::batch("a/d", vec![]).apply(&mut value, "").unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: "a/d".to_string()
            }
        );

        let mut value = json!({"a": {"b": {"c": "1"}}});
        Change::batch(
            "a",
            vec![Change::append("b/c", "2").unwrap(), Change::set("d", 3).unwrap()],
        )
        .apply(&mut value, "")
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": "12"}, "d": 3}}));
    }
}
