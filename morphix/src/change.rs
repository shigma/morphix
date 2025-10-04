use std::fmt::Debug;

use crate::operation::{Operator, ValueOperator};

/// A change in JSON format.
pub struct Change<O: Operator = ValueOperator> {
    pub path_rev: Vec<String>,
    pub operation: Operation<O>,
}

impl<O: Operator> Change<O> {
    /// Apply the change to a JSON value.
    pub fn apply(self, value: &mut O::Replace) -> Result<(), O::Error> {
        O::apply(self, value, &mut vec![])
    }
}

impl<O: Operator> Debug for Change<O>
where
    O::Replace: Debug,
    O::Append: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Change")
            .field("path_rev", &self.path_rev)
            .field("operation", &self.operation)
            .finish()
    }
}

impl<O: Operator> Clone for Change<O>
where
    O::Replace: Clone,
    O::Append: Clone,
{
    fn clone(&self) -> Self {
        Self {
            path_rev: self.path_rev.clone(),
            operation: self.operation.clone(),
        }
    }
}

impl<O: Operator> PartialEq for Change<O>
where
    O::Replace: PartialEq,
    O::Append: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.path_rev == other.path_rev && self.operation == other.operation
    }
}

impl<O: Operator> Eq for Change<O>
where
    O::Replace: Eq,
    O::Append: Eq,
{
}

/// A change in JSON format.
pub enum Operation<O: Operator = ValueOperator> {
    /// `Replace` is the default change for `DerefMut` operations.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// foo.a.b = 1;        // Replace "a/b"
    /// foo.num *= 2;       // Replace "num"
    /// foo.vec.clear();    // Replace "vec"
    /// ```
    ///
    /// If an operation triggers `Append`, no `Replace` change is emitted.
    Replace(O::Replace),

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
    Append(O::Append),

    /// `Batch` represents a sequence of changes.
    Batch(Vec<Change<O>>),
}

impl<O: Operator> Debug for Operation<O>
where
    O::Replace: Debug,
    O::Append: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Replace(value) => f.debug_tuple("Replace").field(value).finish(),
            Operation::Append(value) => f.debug_tuple("Append").field(value).finish(),
            Operation::Batch(changes) => f.debug_tuple("Batch").field(changes).finish(),
        }
    }
}

impl<O: Operator> Clone for Operation<O>
where
    O::Replace: Clone,
    O::Append: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Operation::Replace(value) => Operation::Replace(value.clone()),
            Operation::Append(value) => Operation::Append(value.clone()),
            Operation::Batch(changes) => Operation::Batch(changes.clone()),
        }
    }
}

impl<O: Operator> PartialEq for Operation<O>
where
    O::Replace: PartialEq,
    O::Append: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Operation::Replace(a), Operation::Replace(b)) => a == b,
            (Operation::Append(a), Operation::Append(b)) => a == b,
            (Operation::Batch(a), Operation::Batch(b)) => a == b,
            _ => false,
        }
    }
}

impl<O: Operator> Eq for Operation<O>
where
    O::Replace: Eq,
    O::Append: Eq,
{
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::UmiliError;

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        let change = Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Replace(json!({})),
        };
        change.apply(&mut value).unwrap();
        assert_eq!(value, json!({}));

        let mut value = json!({});
        Change::<ValueOperator> {
            path_rev: vec!["a".to_string()],
            operation: Operation::Replace(json!(1)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": 1}));

        let mut value = json!({"a": 1});
        Change::<ValueOperator> {
            path_rev: vec!["a".to_string()],
            operation: Operation::Replace(json!(2)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": 2}));

        let error = Change::<ValueOperator> {
            path_rev: vec!["a".to_string(), "b".to_string()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut json!({}))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::IndexError {
                path: vec!["a".to_string()]
            }
            .to_string()
        );

        let error = Change::<ValueOperator> {
            path_rev: vec!["a".to_string(), "b".to_string()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut json!({"a": 1}))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::IndexError {
                path: vec!["a".to_string(), "b".to_string()],
            }
            .to_string()
        );

        let error = Change::<ValueOperator> {
            path_rev: vec!["a".to_string(), "b".to_string()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut json!({"a": []}))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::IndexError {
                path: vec!["a".to_string(), "b".to_string()],
            }
            .to_string()
        );

        let mut value = json!({"a": {}});
        Change::<ValueOperator> {
            path_rev: vec!["a".to_string(), "b".to_string()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": 3}}));
    }

    #[test]
    fn apply_append() {
        let mut value = json!("2");
        Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Append(json!("34")),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!("234"));

        let mut value = json!([2]);
        Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Append(json!(["3", "4"])),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        let error = Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Append(json!(3)),
        }
        .apply(&mut json!(""))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::OperationError { path: vec![] }.to_string()
        );

        let error = Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Append(json!("3")),
        }
        .apply(&mut json!({}))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::OperationError { path: vec![] }.to_string()
        );

        let error = Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Append(json!("3")),
        }
        .apply(&mut json!([]))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::OperationError { path: vec![] }.to_string()
        );

        let error = Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Append(json!([3])),
        }
        .apply(&mut json!(""))
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::OperationError { path: vec![] }.to_string()
        );
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Change::<ValueOperator> {
            path_rev: vec![],
            operation: Operation::Batch(vec![]),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": {}}}}));

        let mut value = json!({"a": {"b": {"c": "1"}}});
        let error = Change::<ValueOperator> {
            path_rev: vec!["a".to_string(), "d".to_string()],
            operation: Operation::Batch(vec![]),
        }
        .apply(&mut value)
        .unwrap_err();
        assert_eq!(
            error.to_string(),
            UmiliError::IndexError {
                path: vec!["a".to_string(), "d".to_string()],
            }
            .to_string()
        );

        let mut value = json!({"a": {"b": {"c": "1"}}});
        Change::<ValueOperator> {
            path_rev: vec!["a".to_string()],
            operation: Operation::Batch(vec![
                Change::<ValueOperator> {
                    path_rev: vec!["b".to_string(), "c".to_string()],
                    operation: Operation::Append(json!("2")),
                },
                Change::<ValueOperator> {
                    path_rev: vec!["d".to_string()],
                    operation: Operation::Replace(json!(3)),
                },
            ]),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": "12"}, "d": 3}}));
    }
}
