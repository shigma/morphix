use std::borrow::Cow;
use std::mem::take;

use serde::Serialize;
use serde_json::value::Serializer;
use serde_json::{Error, Value};

use crate::{Adapter, Change, ChangeError, Observe, Operation};

/// JSON adapter for morphix change serialization.
///
/// `JsonAdapter` implements the `Adapter` trait using `serde_json::Value` for both
/// replacement and append operations.
///
/// ## Example
///
/// ```rust
/// use morphix::{JsonAdapter, Observe, observe};
/// use serde::Serialize;
///
/// #[derive(Serialize, Observe)]
/// struct Data {
///     value: i32,
/// }
///
/// let mut data = Data { value: 42 };
/// let change = observe!(JsonAdapter, |mut data| {
///     data.value += 1;
/// }).unwrap();
/// ```
pub struct JsonAdapter;

impl Adapter for JsonAdapter {
    type Replace = Value;
    type Append = Value;
    type Error = Error;

    fn new_replace<T: Serialize + ?Sized>(value: &T) -> Result<Self::Replace, Self::Error> {
        value.serialize(Serializer)
    }

    fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Append, Self::Error> {
        value.serialize_append(Serializer, start_index)
    }

    fn apply_change(
        mut curr_value: &mut Self::Replace,
        mut change: Change<Self>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError> {
        let is_replace = matches!(change.operation, Operation::Replace { .. });
        while let Some(key) = change.path_rev.pop() {
            let next_value = match curr_value {
                Value::Array(vec) => key.parse::<usize>().ok().and_then(|index| vec.get_mut(index)),
                Value::Object(map) => match is_replace && change.path_rev.is_empty() {
                    true => Some(map.entry(&*key).or_insert(Value::Null)),
                    false => map.get_mut(&*key),
                },
                _ => None,
            };
            path_stack.push(key);
            match next_value {
                Some(value) => curr_value = value,
                None => {
                    return Err(ChangeError::IndexError { path: take(path_stack) });
                }
            }
        }

        match change.operation {
            Operation::Replace(value) => {
                *curr_value = value;
            }
            Operation::Append(value) => {
                Self::merge_append(curr_value, value, path_stack)?;
            }
            Operation::Batch(changes) => {
                let len = path_stack.len();
                for change in changes {
                    Self::apply_change(curr_value, change, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }

    fn merge_append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError> {
        match (old_value, new_value) {
            (Value::String(lhs), Value::String(rhs)) => {
                *lhs += &rhs;
            }
            (Value::Array(lhs), Value::Array(rhs)) => {
                lhs.extend(rhs);
            }
            _ => return Err(ChangeError::OperationError { path: take(path_stack) }),
        }
        Ok(())
    }
}
