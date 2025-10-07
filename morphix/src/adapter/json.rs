use std::borrow::Cow;
use std::mem::take;

use serde_json::value::Serializer;
use serde_json::{Error, Value};

use crate::adapter::Adapter;
use crate::change::{Change, Operation};
use crate::error::ChangeError;
use crate::{Observe, ObserveAdapter, Observer};

pub struct JsonAdapter;

impl Adapter for JsonAdapter {
    type Replace = Value;
    type Append = Value;
    type Error = Error;

    fn apply_change(
        mut change: Change<Self>,
        mut curr_value: &mut Self::Replace,
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
                Self::append(curr_value, value, path_stack)?;
            }
            Operation::Batch(changes) => {
                let len = path_stack.len();
                for change in changes {
                    Self::apply_change(change, curr_value, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }

    fn append(
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

    fn new_replace<T: Observe + ?Sized>(value: &T) -> Result<Self::Replace, Self::Error> {
        value.serialize(Serializer)
    }

    fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Append, Self::Error> {
        value.serialize_append(Serializer, start_index)
    }

    fn try_from_observe<'i, T: Observe + ?Sized>(
        observer: &mut T::Target<'i>,
        operation: Operation<ObserveAdapter>,
    ) -> Result<Operation<Self>, Self::Error> {
        Ok(match operation {
            Operation::Replace(()) => Operation::Replace(observer.get_ref().serialize(Serializer)?),
            Operation::Append(start_index) => {
                Operation::Append(observer.get_ref().serialize_append(Serializer, start_index)?)
            }
            Operation::Batch(changes) => Operation::Batch(
                changes
                    .into_iter()
                    .map(|change| -> Result<Change<Self>, Self::Error> {
                        Ok(Change {
                            path_rev: change.path_rev,
                            operation: Self::try_from_observe::<T>(observer, change.operation)?,
                        })
                    })
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
}
