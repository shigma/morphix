use std::borrow::Cow;
use std::mem::take;

use serde_json::Value;

use crate::adapter::Adapter;
use crate::change::{Change, Operation};
use crate::error::ChangeError;
use crate::{Observe, ObserveAdapter};

pub struct JsonAdapter;

impl Adapter for JsonAdapter {
    type Replace = Value;
    type Append = Value;
    type Error = serde_json::Error;

    fn apply_change(
        mut change: Change<Self>,
        mut curr_value: &mut Self::Replace,
        path_stack: &mut Vec<String>,
    ) -> Result<(), ChangeError> {
        let is_replace = matches!(change.operation, Operation::Replace { .. });
        while let Some(key) = change.path_rev.pop() {
            let next_value = match curr_value {
                Value::Array(vec) => key.parse::<usize>().ok().and_then(|index| vec.get_mut(index)),
                Value::Object(map) => match is_replace && change.path_rev.is_empty() {
                    true => Some(map.entry(Cow::Borrowed(key.as_str())).or_insert(Value::Null)),
                    false => map.get_mut(&key),
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
        path_stack: &mut Vec<String>,
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

    fn from_observe<T: Observe>(value: &T, change: Change<ObserveAdapter>) -> Result<Change<Self>, Self::Error> {
        let v = value.serialize_at::<serde_json::value::Serializer>(change.clone())?;
        Ok(match change.operation {
            Operation::Replace(_) => Change {
                path_rev: change.path_rev,
                operation: Operation::Replace(v),
            },
            Operation::Append(_) => Change {
                path_rev: change.path_rev,
                operation: Operation::Append(v),
            },
            _ => unreachable!(),
        })
    }
}
