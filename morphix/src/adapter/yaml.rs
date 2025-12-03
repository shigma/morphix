use std::mem::take;

use serde::Serialize;
use serde_yaml_ng::value::Serializer;
use serde_yaml_ng::{Error, Value};

use crate::{Adapter, Mutation, MutationError, MutationKind, Path, PathSegment};

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

    fn apply_mutation(
        mut curr_value: &mut Self::Value,
        mut mutation: Mutation<Self::Value>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        let is_replace = matches!(mutation.kind, MutationKind::Replace { .. });

        while let Some(key) = mutation.path.pop() {
            let next_value = match (curr_value, &key) {
                (Value::Sequence(vec), PathSegment::Positive(index)) => vec.get_mut(*index),
                (Value::Sequence(vec), PathSegment::Negative(index)) => {
                    vec.len().checked_sub(*index).and_then(|i| vec.get_mut(i))
                }
                (Value::Mapping(map), PathSegment::String(key)) => {
                    if is_replace && mutation.path.is_empty() {
                        Some(map.entry(Value::String(key.to_string())).or_insert(Value::Null))
                    } else {
                        map.get_mut(&**key)
                    }
                }
                _ => None,
            };
            path_stack.push(key);
            match next_value {
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
            #[cfg(feature = "append")]
            MutationKind::Append(value) => {
                Self::merge_append(curr_value, value, path_stack)?;
            }
            #[cfg(feature = "truncate")]
            MutationKind::Truncate(truncate_len) => {
                if let Some(actual_len) = Self::apply_truncate(curr_value, truncate_len, path_stack)? {
                    return Err(MutationError::TruncateError {
                        path: take(path_stack),
                        actual_len,
                        truncate_len,
                    });
                }
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

    #[cfg(feature = "append")]
    fn merge_append(
        old_value: &mut Self::Value,
        new_value: Self::Value,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        match (old_value, new_value) {
            (Value::String(lhs), Value::String(rhs)) => {
                *lhs += &rhs;
            }
            (Value::Sequence(lhs), Value::Sequence(rhs)) => {
                lhs.extend(rhs);
            }
            _ => return Err(MutationError::OperationError { path: take(path_stack) }),
        }
        Ok(())
    }

    #[cfg(feature = "truncate")]
    fn apply_truncate(
        value: &mut Self::Value,
        truncate_len: usize,
        path_stack: &mut Path<false>,
    ) -> Result<Option<usize>, MutationError> {
        match value {
            Value::String(_str) => {
                todo!()
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

    fn get_len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError> {
        // FIXME: str should have char length instead of byte length
        match value {
            Value::String(str) => Ok(str.len()),
            Value::Sequence(vec) => Ok(vec.len()),
            _ => Err(MutationError::OperationError { path: take(path_stack) }),
        }
    }
}
