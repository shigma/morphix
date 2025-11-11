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
                (Value::Sequence(vec), PathSegment::PosIndex(index)) => vec.get_mut(*index),
                (Value::Sequence(vec), PathSegment::NegIndex(index)) => {
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
            (Value::Sequence(lhs), Value::Sequence(rhs)) => {
                lhs.extend(rhs);
            }
            _ => return Err(MutationError::OperationError { path: take(path_stack) }),
        }
        Ok(())
    }

    fn get_len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError> {
        match value {
            Value::String(str) => Ok(str.len()),
            Value::Sequence(vec) => Ok(vec.len()),
            _ => Err(MutationError::OperationError { path: take(path_stack) }),
        }
    }
}
