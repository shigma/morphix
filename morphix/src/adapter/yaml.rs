use std::mem::take;

use serde::Serialize;
use serde_yaml_ng::value::Serializer;
use serde_yaml_ng::{Error, Value};

use crate::{Adapter, Mutation, MutationError, Path, PathSegment};

/// YAML adapter for morphix mutation serialization.
///
/// `Yaml` implements the [`Adapter`] trait using [`serde_yaml_ng::Value`] for both
/// [`Replace`](MutationKind::Replace) and [`Append`](MutationKind::Append) operations. This adapter
/// is available when the `yaml` feature is enabled.
///
/// ## Example
///
/// ```
/// use morphix::adapter::Yaml;
/// use morphix::{Observe, observe};
/// use serde::Serialize;
///
/// #[derive(Serialize, Observe)]
/// struct Config {
///     host: String,
///     port: u16,
///     tags: Vec<String>,
/// }
///
/// let mut config = Config {
///     host: "localhost".to_string(),
///     port: 8080,
///     tags: vec!["web".to_string()],
/// };
///
/// let Yaml(mutation) = observe!(config => {
///     config.port = 8081;
///     config.tags.push("api".to_string());
/// }).unwrap();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yaml(pub Option<Mutation<Value>>);

impl Adapter for Yaml {
    type Value = Value;
    type Error = Error;
    type IntoValues = std::vec::IntoIter<Self::Value>;

    fn from_mutation(mutation: Option<Mutation<Self::Value>>) -> Self {
        Yaml(mutation)
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
            (Value::Sequence(vec), PathSegment::Positive(index)) => vec.get_mut(*index),
            (Value::Sequence(vec), PathSegment::Negative(index)) => {
                vec.len().checked_sub(*index).and_then(|i| vec.get_mut(i))
            }
            (Value::Mapping(map), PathSegment::String(key)) => {
                if allow_create {
                    Some(map.entry(Value::String(key.to_string())).or_insert(Value::Null))
                } else {
                    map.get_mut(&**key)
                }
            }
            _ => None,
        }
    }

    #[cfg(feature = "append")]
    fn apply_append(
        value: &mut Self::Value,
        append_value: Self::Value,
        path_stack: &mut Path<false>,
    ) -> Result<usize, MutationError> {
        match (value, append_value) {
            (Value::String(lhs), Value::String(rhs)) => {
                let len = rhs.chars().count();
                *lhs += &rhs;
                Ok(len)
            }
            (Value::Sequence(lhs), Value::Sequence(rhs)) => {
                let len = rhs.len();
                lhs.extend(rhs);
                Ok(len)
            }
            _ => Err(MutationError::OperationError { path: take(path_stack) }),
        }
    }

    #[cfg(feature = "truncate")]
    fn apply_truncate(
        value: &mut Self::Value,
        mut truncate_len: usize,
        path_stack: &mut Path<false>,
    ) -> Result<Option<usize>, MutationError> {
        match value {
            Value::String(str) => {
                let mut chars = str.char_indices();
                let mut byte_len = str.len();
                let mut char_len = 0;
                while truncate_len > 0
                    && let Some((index, _)) = chars.next_back()
                {
                    truncate_len -= 1;
                    byte_len = index;
                    char_len += 1;
                }
                if truncate_len > 0 {
                    Ok(Some(char_len))
                } else {
                    str.truncate(byte_len);
                    Ok(None)
                }
            }
            Value::Sequence(vec) => {
                let actual_len = vec.len();
                if actual_len >= truncate_len {
                    vec.truncate(actual_len - truncate_len);
                    Ok(None)
                } else {
                    Ok(Some(actual_len))
                }
            }
            _ => Err(MutationError::OperationError { path: take(path_stack) }),
        }
    }

    fn into_values(value: Self::Value) -> Option<Self::IntoValues> {
        match value {
            Value::Sequence(vec) => Some(vec.into_iter()),
            _ => None,
        }
    }

    fn from_values(values: Self::IntoValues) -> Self::Value {
        Value::Sequence(values.collect())
    }

    fn len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError> {
        // FIXME: str should have char length instead of byte length
        match value {
            Value::String(str) => Ok(str.len()),
            Value::Sequence(vec) => Ok(vec.len()),
            _ => Err(MutationError::OperationError { path: take(path_stack) }),
        }
    }
}
