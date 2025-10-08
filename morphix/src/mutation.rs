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

/// A mutation representing a change to a value at a specific path.
///
/// `Mutation` captures both the location where a change occurred (via `path_rev`) and the kind of
/// change that was made (via `operation`). Mutations can be applied to values to reproduce the
/// changes they represent.
///
/// ## Path Representation
///
/// The path is stored in reverse order for efficiency during collection.
/// For example, a change at `foo.bar.baz` would have `path_rev = ["baz", "bar", "foo"]`.
///
/// ## Example
///
/// ```
/// use morphix::{JsonAdapter, Mutation, MutationKind};
/// use serde_json::json;
///
/// // A mutation that replaces the value at path "user.name"
/// let mutation = Mutation::<JsonAdapter> {
///     path_rev: vec!["name".into(), "user".into()],
///     operation: MutationKind::Replace(json!("Alice")),
/// };
///
/// // Apply the mutation to a JSON value
/// let mut data = json!({"user": {"name": "Bob", "age": 30}});
/// mutation.apply(&mut data).unwrap();
/// assert_eq!(data, json!({"user": {"name": "Alice", "age": 30}}));
/// ```
pub struct Mutation<A: Adapter> {
    /// The path to the mutated value, stored in reverse order.
    ///
    /// An empty vec indicates a mutation at the root level.
    pub path_rev: Vec<Cow<'static, str>>,

    /// The kind of mutation that occurred.
    pub operation: MutationKind<A>,
}

impl<A: Adapter> Mutation<A> {
    /// Applies this mutation to a value.
    ///
    /// ## Arguments
    ///
    /// - `value` - value to mutate
    ///
    /// ## Errors
    ///
    /// - Returns [IndexError](MutationError::IndexError) if the path doesn't exist in the value.
    /// - Returns [OperationError](MutationError::OperationError) if the mutation cannot be
    ///   performed.
    ///
    /// # Example
    ///
    /// ```
    /// use morphix::{Mutation, MutationKind, JsonAdapter};
    /// use serde_json::json;
    ///
    /// let mut value = json!({"count": 0});
    ///
    /// Mutation::<JsonAdapter> {
    ///     path_rev: vec!["count".into()],
    ///     operation: MutationKind::Replace(json!(42)),
    /// }
    /// .apply(&mut value)
    /// .unwrap();
    ///
    /// assert_eq!(value, json!({"count": 42}));
    /// ```
    pub fn apply(self, value: &mut A::Value) -> Result<(), MutationError> {
        A::apply_mutation(value, self, &mut vec![])
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

/// The kind of mutation that occurred.
///
/// `MutationKind` represents the specific type of change made to a value.
/// Different kinds enable optimizations and more precise change descriptions.
///
/// ## Variants
///
/// - [`Replace`](MutationKind::Replace): Complete replacement of a value
/// - [`Append`](MutationKind::Append): Append operation for strings and vectors
/// - [`Batch`](MutationKind::Batch): Multiple mutations combined
///
/// ## Example
///
/// ```
/// use morphix::{JsonAdapter, Mutation, MutationKind, Observe, observe};
/// use serde::Serialize;
/// use serde_json::json;
///
/// #[derive(Serialize, Observe)]
/// struct Document {
///     title: String,
///     content: String,
///     tags: Vec<String>,
/// }
///
/// let mut doc = Document {
///     title: "Draft".to_string(),
///     content: "Hello".to_string(),
///     tags: vec!["todo".to_string()],
/// };
///
/// let mutation = observe!(JsonAdapter, |mut doc| {
///     doc.title = "Final".to_string();      // Replace
///     doc.content.push_str(" World");       // Append
///     doc.tags.push("done".to_string());    // Append
/// }).unwrap().unwrap();
///
/// // The mutation contains a Batch with three operations
/// matches!(mutation.operation, MutationKind::Batch(_));
/// ```
pub enum MutationKind<A: Adapter> {
    /// `Replace` is the default mutation for `DerefMut` operations.
    ///
    /// ## Examples
    ///
    /// ```
    /// # #[derive(Default)]
    /// # struct Foo {
    /// #   a: FooA,
    /// #   num: i32,
    /// #   vec: Vec<i32>,
    /// # }
    /// # #[derive(Default)]
    /// # struct FooA {
    /// #   b: i32,
    /// # }
    /// # let mut foo = Foo::default();
    /// foo.a.b = 1;        // Replace at .a.b
    /// foo.num *= 2;       // Replace at .num
    /// foo.vec.clear();    // Replace at .vec
    /// ```
    ///
    /// ## Note
    ///
    /// If an operation can be represented as `Append`, it will be preferred
    /// over `Replace` for efficiency.
    Replace(A::Value),

    /// `Append` represents adding data to the end of a string or vector. This is more efficient
    /// than `Replace` because only the appended portion needs to be serialized and transmitted.
    ///
    /// ## Examples
    ///
    /// ```
    /// # #[derive(Default)]
    /// # struct Foo {
    /// #   a: FooA,
    /// #   vec: Vec<i32>,
    /// # }
    /// # #[derive(Default)]
    /// # struct FooA {
    /// #   b: String,
    /// # }
    /// # let mut foo = Foo::default();
    /// # let iter = vec![2, 3].into_iter();
    /// foo.a.b += "text";          // Append .a.b
    /// foo.a.b.push_str("text");   // Append .a.b
    /// foo.vec.push(1);            // Append .vec
    /// foo.vec.extend(iter);       // Append .vec
    /// ```
    Append(A::Value),

    /// `Batch` combines multiple mutations that occurred during a single observation period. This
    /// is automatically created when multiple independent changes are detected.
    ///
    /// ## Optimization
    ///
    /// The batch collector ([`Batch`]) automatically optimizes mutations:
    /// - Consecutive appends are merged
    /// - Redundant changes are eliminated
    /// - Nested paths are consolidated when possible
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
