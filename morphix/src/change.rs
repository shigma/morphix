use std::borrow::Cow;
use std::fmt::{Debug, Display};

use crate::adapter::Adapter;
use crate::error::ChangeError;

struct Path<'i>(&'i Vec<Cow<'static, str>>);

impl<'i> Display for Path<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (index, value) in self.0.iter().enumerate().rev() {
            f.write_str(value)?;
            if index != 0 {
                f.write_str(".")?;
            }
        }
        Ok(())
    }
}

/// A change in JSON format.
pub struct Change<A: Adapter> {
    pub path_rev: Vec<Cow<'static, str>>,
    pub operation: Operation<A>,
}

impl<A: Adapter> Change<A> {
    /// Apply the change to a JSON value.
    pub fn apply(self, value: &mut A::Replace) -> Result<(), ChangeError> {
        A::apply_change(value, self, &mut vec![])
    }
}

impl<A: Adapter> Debug for Change<A>
where
    A::Replace: Debug,
    A::Append: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Change")
            .field("path", &Path(&self.path_rev).to_string())
            .field("operation", &self.operation)
            .finish()
    }
}

impl<A: Adapter> Clone for Change<A>
where
    A::Replace: Clone,
    A::Append: Clone,
{
    fn clone(&self) -> Self {
        Self {
            path_rev: self.path_rev.clone(),
            operation: self.operation.clone(),
        }
    }
}

impl<A: Adapter> PartialEq for Change<A>
where
    A::Replace: PartialEq,
    A::Append: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.path_rev == other.path_rev && self.operation == other.operation
    }
}

impl<A: Adapter> Eq for Change<A>
where
    A::Replace: Eq,
    A::Append: Eq,
{
}

/// A change in JSON format.
pub enum Operation<A: Adapter> {
    /// `Replace` is the default change for `DerefMut` operations.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// foo.a.b = 1;        // Replace .a.b
    /// foo.num *= 2;       // Replace .num
    /// foo.vec.clear();    // Replace .vec
    /// ```
    ///
    /// If an operation triggers `Append`, no `Replace` change is emitted.
    Replace(A::Replace),

    /// `Append` represents a `String` or `Vec` append operation.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// foo.a.b += "text";          // Append .a.b
    /// foo.a.b.push_str("text");   // Append .a.b
    /// foo.vec.push(1);            // Append .vec
    /// foo.vec.extend(iter);       // Append .vec
    /// ```
    Append(A::Append),

    /// `Batch` represents a sequence of changes.
    Batch(Vec<Change<A>>),
}

impl<A: Adapter> Debug for Operation<A>
where
    A::Replace: Debug,
    A::Append: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Replace(value) => f.debug_tuple("Replace").field(value).finish(),
            Operation::Append(value) => f.debug_tuple("Append").field(value).finish(),
            Operation::Batch(changes) => f.debug_tuple("Batch").field(changes).finish(),
        }
    }
}

impl<A: Adapter> Clone for Operation<A>
where
    A::Replace: Clone,
    A::Append: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Operation::Replace(value) => Operation::Replace(value.clone()),
            Operation::Append(value) => Operation::Append(value.clone()),
            Operation::Batch(changes) => Operation::Batch(changes.clone()),
        }
    }
}

impl<A: Adapter> PartialEq for Operation<A>
where
    A::Replace: PartialEq,
    A::Append: PartialEq,
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

impl<A: Adapter> Eq for Operation<A>
where
    A::Replace: Eq,
    A::Append: Eq,
{
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::{ChangeError, JsonAdapter};

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Replace(json!({})),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({}));

        let mut value = json!({});
        Change::<JsonAdapter> {
            path_rev: vec!["a".into()],
            operation: Operation::Replace(json!(1)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": 1}));

        let mut value = json!({"a": 1});
        Change::<JsonAdapter> {
            path_rev: vec!["a".into()],
            operation: Operation::Replace(json!(2)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": 2}));

        let error = Change::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut json!({}))
        .unwrap_err();
        assert_eq!(error, ChangeError::IndexError { path: vec!["a".into()] });

        let error = Change::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut json!({"a": 1}))
        .unwrap_err();
        assert_eq!(
            error,
            ChangeError::IndexError {
                path: vec!["a".into(), "b".into()],
            }
        );

        let error = Change::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut json!({"a": []}))
        .unwrap_err();
        assert_eq!(
            error,
            ChangeError::IndexError {
                path: vec!["a".into(), "b".into()],
            }
        );

        let mut value = json!({"a": {}});
        Change::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: Operation::Replace(json!(3)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": 3}}));
    }

    #[test]
    fn apply_append() {
        let mut value = json!("2");
        Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Append(json!("34")),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!("234"));

        let mut value = json!([2]);
        Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Append(json!(["3", "4"])),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        let error = Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Append(json!(3)),
        }
        .apply(&mut json!(""))
        .unwrap_err();
        assert_eq!(error, ChangeError::OperationError { path: vec![] });

        let error = Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Append(json!("3")),
        }
        .apply(&mut json!({}))
        .unwrap_err();
        assert_eq!(error, ChangeError::OperationError { path: vec![] });

        let error = Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Append(json!("3")),
        }
        .apply(&mut json!([]))
        .unwrap_err();
        assert_eq!(error, ChangeError::OperationError { path: vec![] });

        let error = Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Append(json!([3])),
        }
        .apply(&mut json!(""))
        .unwrap_err();
        assert_eq!(error, ChangeError::OperationError { path: vec![] });
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Change::<JsonAdapter> {
            path_rev: vec![],
            operation: Operation::Batch(vec![]),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": {}}}}));

        let mut value = json!({"a": {"b": {"c": "1"}}});
        let error = Change::<JsonAdapter> {
            path_rev: vec!["d".into(), "a".into()],
            operation: Operation::Batch(vec![]),
        }
        .apply(&mut value)
        .unwrap_err();
        assert_eq!(
            error,
            ChangeError::IndexError {
                path: vec!["a".into(), "d".into()],
            }
        );

        let mut value = json!({"a": {"b": {"c": "1"}}});
        Change::<JsonAdapter> {
            path_rev: vec!["a".into()],
            operation: Operation::Batch(vec![
                Change::<JsonAdapter> {
                    path_rev: vec!["c".into(), "b".into()],
                    operation: Operation::Append(json!("2")),
                },
                Change::<JsonAdapter> {
                    path_rev: vec!["d".into()],
                    operation: Operation::Replace(json!(3)),
                },
            ]),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": "12"}, "d": 3}}));
    }
}
