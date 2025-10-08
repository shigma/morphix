use std::borrow::Cow;
use std::fmt::{Debug, Display};

use crate::{Adapter, MutationError};

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

/// A mutation in JSON format.
pub struct Mutation<A: Adapter> {
    pub path_rev: Vec<Cow<'static, str>>,
    pub operation: MutationKind<A>,
}

impl<A: Adapter> Mutation<A> {
    /// Apply the mutation to a JSON value.
    pub fn apply(self, value: &mut A::Value) -> Result<(), MutationError> {
        A::apply_change(value, self, &mut vec![])
    }
}

impl<A: Adapter> Debug for Mutation<A>
where
    A::Value: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutation")
            .field("path", &Path(&self.path_rev).to_string())
            .field("operation", &self.operation)
            .finish()
    }
}

impl<A: Adapter> Clone for Mutation<A>
where
    A::Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            path_rev: self.path_rev.clone(),
            operation: self.operation.clone(),
        }
    }
}

impl<A: Adapter> PartialEq for Mutation<A>
where
    A::Value: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.path_rev == other.path_rev && self.operation == other.operation
    }
}

impl<A: Adapter> Eq for Mutation<A> where A::Value: Eq {}

/// A mutation in JSON format.
pub enum MutationKind<A: Adapter> {
    /// `Replace` is the default mutation for `DerefMut` operations.
    ///
    /// ## Example
    ///
    /// ```ignore
    /// foo.a.b = 1;        // Replace .a.b
    /// foo.num *= 2;       // Replace .num
    /// foo.vec.clear();    // Replace .vec
    /// ```
    ///
    /// If an operation triggers `Append`, no `Replace` mutation is emitted.
    Replace(A::Value),

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
    Append(A::Value),

    /// `Batch` represents a sequence of mutations.
    Batch(Vec<Mutation<A>>),
}

impl<A: Adapter> Debug for MutationKind<A>
where
    A::Value: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutationKind::Replace(value) => f.debug_tuple("Replace").field(value).finish(),
            MutationKind::Append(value) => f.debug_tuple("Append").field(value).finish(),
            MutationKind::Batch(mutations) => f.debug_tuple("Batch").field(mutations).finish(),
        }
    }
}

impl<A: Adapter> Clone for MutationKind<A>
where
    A::Value: Clone,
{
    fn clone(&self) -> Self {
        match self {
            MutationKind::Replace(value) => MutationKind::Replace(value.clone()),
            MutationKind::Append(value) => MutationKind::Append(value.clone()),
            MutationKind::Batch(mutations) => MutationKind::Batch(mutations.clone()),
        }
    }
}

impl<A: Adapter> PartialEq for MutationKind<A>
where
    A::Value: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (MutationKind::Replace(a), MutationKind::Replace(b)) => a == b,
            (MutationKind::Append(a), MutationKind::Append(b)) => a == b,
            (MutationKind::Batch(a), MutationKind::Batch(b)) => a == b,
            _ => false,
        }
    }
}

impl<A: Adapter> Eq for MutationKind<A> where A::Value: Eq {}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::{JsonAdapter, MutationError};

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Replace(json!({})),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({}));

        let mut value = json!({});
        Mutation::<JsonAdapter> {
            path_rev: vec!["a".into()],
            operation: MutationKind::Replace(json!(1)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": 1}));

        let mut value = json!({"a": 1});
        Mutation::<JsonAdapter> {
            path_rev: vec!["a".into()],
            operation: MutationKind::Replace(json!(2)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": 2}));

        let error = Mutation::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: MutationKind::Replace(json!(3)),
        }
        .apply(&mut json!({}))
        .unwrap_err();
        assert_eq!(error, MutationError::IndexError { path: vec!["a".into()] });

        let error = Mutation::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: MutationKind::Replace(json!(3)),
        }
        .apply(&mut json!({"a": 1}))
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into(), "b".into()],
            }
        );

        let error = Mutation::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: MutationKind::Replace(json!(3)),
        }
        .apply(&mut json!({"a": []}))
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into(), "b".into()],
            }
        );

        let mut value = json!({"a": {}});
        Mutation::<JsonAdapter> {
            path_rev: vec!["b".into(), "a".into()],
            operation: MutationKind::Replace(json!(3)),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": 3}}));
    }

    #[test]
    fn apply_append() {
        let mut value = json!("2");
        Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Append(json!("34")),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!("234"));

        let mut value = json!([2]);
        Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Append(json!(["3", "4"])),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        let error = Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Append(json!(3)),
        }
        .apply(&mut json!(""))
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![] });

        let error = Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Append(json!("3")),
        }
        .apply(&mut json!({}))
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![] });

        let error = Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Append(json!("3")),
        }
        .apply(&mut json!([]))
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![] });

        let error = Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Append(json!([3])),
        }
        .apply(&mut json!(""))
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![] });
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Mutation::<JsonAdapter> {
            path_rev: vec![],
            operation: MutationKind::Batch(vec![]),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": {}}}}));

        let mut value = json!({"a": {"b": {"c": "1"}}});
        let error = Mutation::<JsonAdapter> {
            path_rev: vec!["d".into(), "a".into()],
            operation: MutationKind::Batch(vec![]),
        }
        .apply(&mut value)
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into(), "d".into()],
            }
        );

        let mut value = json!({"a": {"b": {"c": "1"}}});
        Mutation::<JsonAdapter> {
            path_rev: vec!["a".into()],
            operation: MutationKind::Batch(vec![
                Mutation::<JsonAdapter> {
                    path_rev: vec!["c".into(), "b".into()],
                    operation: MutationKind::Append(json!("2")),
                },
                Mutation::<JsonAdapter> {
                    path_rev: vec!["d".into()],
                    operation: MutationKind::Replace(json!(3)),
                },
            ]),
        }
        .apply(&mut value)
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": "12"}, "d": 3}}));
    }
}
