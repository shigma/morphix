use serde::Serialize;
use serde_json::value::Serializer;
use serde_json::{Error, Value};

use crate::{Adapter, Mutation, PathSegment};

/// JSON adapter for morphix mutation serialization.
///
/// [`Json`] implements the [`Adapter`] trait using [`serde_json::Value`].
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
/// let Json(mutation) = observe!(data => {
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

    fn get_mut<'a>(
        value: &'a mut Self::Value,
        segment: &PathSegment,
        allow_create: bool,
    ) -> Option<&'a mut Self::Value> {
        match (value, segment) {
            (Value::Array(vec), PathSegment::Positive(index)) => vec.get_mut(*index),
            (Value::Array(vec), PathSegment::Negative(index)) => {
                vec.len().checked_sub(*index).and_then(|i| vec.get_mut(i))
            }
            (Value::Object(map), PathSegment::String(key)) => {
                if allow_create {
                    Some(map.entry(&**key).or_insert(Value::Null))
                } else {
                    map.get_mut(&**key)
                }
            }
            _ => None,
        }
    }

    #[cfg(feature = "append")]
    fn append(value: &mut Self::Value, append_value: Self::Value) -> Option<usize> {
        match (value, append_value) {
            (Value::String(lhs), Value::String(rhs)) => {
                let len = rhs.chars().count();
                *lhs += &rhs;
                Some(len)
            }
            (Value::Array(lhs), Value::Array(rhs)) => {
                let len = rhs.len();
                lhs.extend(rhs);
                Some(len)
            }
            _ => None,
        }
    }

    #[cfg(feature = "append")]
    fn len(value: &Self::Value) -> Option<usize> {
        match value {
            Value::String(str) => Some(str.chars().count()),
            Value::Array(vec) => Some(vec.len()),
            _ => None,
        }
    }

    #[cfg(feature = "truncate")]
    fn truncate(value: &mut Self::Value, mut truncate_len: usize) -> Option<usize> {
        match value {
            Value::String(str) => {
                let mut chars = str.char_indices();
                let mut new_len = str.len();
                while truncate_len > 0
                    && let Some((index, _)) = chars.next_back()
                {
                    truncate_len -= 1;
                    new_len = index;
                }
                str.truncate(new_len);
                Some(truncate_len)
            }
            Value::Array(vec) => {
                let actual_len = vec.len();
                let new_len = actual_len.saturating_sub(truncate_len);
                vec.truncate(new_len);
                Some(truncate_len.saturating_sub(actual_len))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::{MutationError, MutationKind};

    #[test]
    fn apply_set() {
        let mut value = json!({"a": 1});
        Json::mutate(
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
        Json::mutate(
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
        Json::mutate(
            &mut value,
            Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Replace(json!(2)),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!({"a": 2}));

        let error = Json::mutate(
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

        let error = Json::mutate(
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

        let error = Json::mutate(
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
        Json::mutate(
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
        Json::mutate(
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
        Json::mutate(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(["3", "4"])),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!([2, "3", "4"]));

        let error = Json::mutate(
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

        let error = Json::mutate(
            &mut json!({}),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("3")),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![].into() });

        let error = Json::mutate(
            &mut json!([]),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("3")),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(error, MutationError::OperationError { path: vec![].into() });

        let error = Json::mutate(
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
    fn apply_truncate() {
        let mut value = json!("Hello, World!");
        Json::mutate(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Truncate(8),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!("Hello"));

        let mut value = json!("我是谁");
        Json::mutate(
            &mut value,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Truncate(2),
            },
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(value, json!("我"));

        let error = Json::mutate(
            &mut json!("Hello, World!"),
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Truncate(20),
            },
            &mut Default::default(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            MutationError::TruncateError {
                path: vec![].into(),
                actual_len: 13,
                truncate_len: 20,
            }
        );
    }

    #[test]
    fn apply_batch() {
        let mut value = json!({"a": {"b": {"c": {}}}});
        Json::mutate(
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
        let error = Json::mutate(
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
        Json::mutate(
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
