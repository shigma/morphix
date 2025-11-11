use std::mem::take;

use serde::Serialize;
use serde_json::value::Serializer;
use serde_json::{Error, Value};

use crate::{Adapter, Mutation, MutationError, MutationKind, Path, PathSegment};

/// JSON adapter for morphix mutation serialization.
///
/// `Json` implements the [`Adapter`] trait using [`serde_json::Value`] for both
/// [`Replace`](MutationKind::Replace) and [`Append`](MutationKind::Append) operations. This adapter
/// is available when the `json` feature is enabled.
///
/// ## Example
///
/// ```
/// use morphix::adapter::Json;
/// use morphix::{Observe, observe};
/// use serde::Serialize;
///
/// #[derive(Serialize, Observe)]
/// struct Data {
///     value: i32,
/// }
///
/// let mut data = Data { value: 42 };
/// let Json(mutation) = observe!(|mut data| {
///     data.value += 1;
/// }).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Json(pub Option<Mutation<Value>>);

impl Adapter for Json {
    type Value = Value;
    type Error = Error;

    fn from_mutation(mutation: Option<Mutation<Self::Value>>) -> Self {
        Json(mutation)
    }

    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error> {
        value.serialize(Serializer)
    }

    fn apply_mutation(
        mut curr_value: &mut Self::Value,
        mut mutation: Mutation<Self::Value>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        let is_replace = matches!(mutation.kind, MutationKind::Replace { .. });

        while let Some(key) = mutation.path.pop() {
            let new_value = match (curr_value, &key) {
                (Value::Array(vec), PathSegment::PosIndex(index)) => vec.get_mut(*index),
                (Value::Array(vec), PathSegment::NegIndex(index)) => {
                    vec.len().checked_sub(*index).and_then(|i| vec.get_mut(i))
                }
                (Value::Object(map), PathSegment::String(key)) => {
                    if is_replace && mutation.path.is_empty() {
                        Some(map.entry(&**key).or_insert(Value::Null))
                    } else {
                        map.get_mut(&**key)
                    }
                }
                _ => None,
            };
            path_stack.push(key);
            match new_value {
                Some(value) => curr_value = value,
                None => {
                    return Err(MutationError::IndexError { path: take(path_stack) });
                }
            }
        }

        match mutation.kind {
            MutationKind::Replace(value) => {
                *curr_value = value;
            }
            MutationKind::Append(value) => {
                Self::merge_append(curr_value, value, path_stack)?;
            }
            MutationKind::Batch(mutations) => {
                let len = path_stack.len();
                for mutation in mutations {
                    Self::apply_mutation(curr_value, mutation, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }

    fn merge_append(
        old_value: &mut Self::Value,
        new_value: Self::Value,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        match (old_value, new_value) {
            (Value::String(lhs), Value::String(rhs)) => {
                *lhs += &rhs;
            }
            (Value::Array(lhs), Value::Array(rhs)) => {
                lhs.extend(rhs);
            }
            _ => return Err(MutationError::OperationError { path: take(path_stack) }),
        }
        Ok(())
    }

    fn get_len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError> {
        match value {
            Value::String(str) => Ok(str.len()),
            Value::Array(vec) => Ok(vec.len()),
            _ => Err(MutationError::OperationError { path: take(path_stack) }),
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::MutationError;

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Replace(json!({})),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({}));

        let mut value = json!({});
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Replace(json!(1)),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({"a": 1}));

        let mut value = json!({"a": 1});
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Replace(json!(2)),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({"a": 2}));

        let error = Json::apply_mutation(
            &mut json!({}),
            Mutation {
                path: vec!["a".into(), "b".into()].into(),
                kind: MutationKind::Replace(json!(3)),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into()].into()
            }
        );

        let error = Json::apply_mutation(
            &mut json!({"a": 1}),
            Mutation {
                path: vec!["a".into(), "b".into()].into(),
                kind: MutationKind::Replace(json!(3)),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into(), "b".into()].into(),
            }
        );

        let error = Json::apply_mutation(
            &mut json!({"a": []}),
            Mutation {
                path: vec!["a".into(), "b".into()].into(),
                kind: MutationKind::Replace(json!(3)),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into(), "b".into()].into(),
            }
        );

        let mut value = json!({"a": {}});
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec!["a".into(), "b".into()].into(),
                kind: MutationKind::Replace(json!(3)),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({"a": {"b": 3}}));
    }

    #[test]
    fn apply_append() {
        let mut value = json!("2");
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("34")),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!("234"));

        let mut value = json!([2]);
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(["3", "4"])),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        let error = Json::apply_mutation(
            &mut json!(""),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(3)),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::OperationError {
                path: Default::default()
            }
        );

        let error = Json::apply_mutation(
            &mut json!({}),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("3")),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![].into() });

        let error = Json::apply_mutation(
            &mut json!([]),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("3")),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![].into() });

        let error = Json::apply_mutation(
            &mut json!(""),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!([3])),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![].into() });
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![]),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": {}}}}));

        let mut value = json!({"a": {"b": {"c": "1"}}});
        let error = Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec!["a".into(), "d".into()].into(),
                kind: MutationKind::Batch(vec![]),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::IndexError {
                path: vec!["a".into(), "d".into()].into(),
            }
        );

        let mut value = json!({"a": {"b": {"c": "1"}}});
        Json::apply_mutation(
            &mut value,
            Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec!["b".into(), "c".into()].into(),
                        kind: MutationKind::Append(json!("2")),
                    },
                    Mutation {
                        path: vec!["d".into()].into(),
                        kind: MutationKind::Replace(json!(3)),
                    },
                ]),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({"a": {"b": {"c": "12"}, "d": 3}}));
    }
}
