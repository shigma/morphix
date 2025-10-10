use std::borrow::Cow;
use std::mem::take;

use serde::Serialize;
use serde_json::value::Serializer;
use serde_json::{Error, Value};

use crate::{Adapter, Mutation, MutationError, MutationKind};

/// JSON adapter for morphix mutation serialization.
///
/// `JsonAdapter` implements the [`Adapter`] trait using [`serde_json::Value`] for both
/// [`Replace`](MutationKind::Replace) and [`Append`](MutationKind::Append) operations. This adapter
/// is available when the `json` feature is enabled.
///
/// ## Example
///
/// ```
/// use morphix::{JsonAdapter, Observe, observe};
/// use serde::Serialize;
///
/// #[derive(Serialize, Observe)]
/// struct Data {
///     value: i32,
/// }
///
/// let mut data = Data { value: 42 };
/// let mutation = observe!(JsonAdapter, |mut data| {
///     data.value += 1;
/// }).unwrap();
/// ```
pub struct JsonAdapter;

impl Adapter for JsonAdapter {
    type Value = Value;
    type Error = Error;

    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error> {
        value.serialize(Serializer)
    }

    fn apply_mutation(
        mut curr_value: &mut Self::Value,
        mut mutation: Mutation<Self>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), MutationError> {
        let is_replace = matches!(mutation.operation, MutationKind::Replace { .. });

        while let Some(key) = mutation.path_rev.pop() {
            let next_value = match curr_value {
                Value::Array(vec) => key.parse::<usize>().ok().and_then(|index| vec.get_mut(index)),
                Value::Object(map) => match is_replace && mutation.path_rev.is_empty() {
                    true => Some(map.entry(&*key).or_insert(Value::Null)),
                    false => map.get_mut(&*key),
                },
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

        match mutation.operation {
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
        path_stack: &mut Vec<Cow<'static, str>>,
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
}
