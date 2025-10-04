use std::borrow::Cow;
use std::convert::Infallible;
use std::mem::take;

use serde_json::Value;

use crate::Operation;
use crate::change::Change;
use crate::error::ChangeError;

pub trait Adapter: Sized {
    type Replace;
    type Append;
    type Error;

    fn apply(
        change: Change<Self>,
        root_value: &mut Self::Replace,
        path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error>;

    fn append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonAdapter;

impl Adapter for JsonAdapter {
    type Replace = Value;
    type Append = Value;
    type Error = ChangeError;

    fn apply(
        mut change: Change<Self>,
        mut root_value: &mut Self::Replace,
        path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error> {
        let is_replace = matches!(change.operation, Operation::Replace { .. });
        while let Some(key) = change.path_rev.pop() {
            let next_value = json_index(root_value, &key, is_replace && change.path_rev.is_empty());
            path_stack.push(key);
            match next_value {
                Some(value) => root_value = value,
                None => {
                    return Err(ChangeError::IndexError { path: take(path_stack) });
                }
            }
        }

        match change.operation {
            Operation::Replace(value) => {
                *root_value = value;
            }
            Operation::Append(value) => {
                Self::append(root_value, value, path_stack)?;
            }
            Operation::Batch(changes) => {
                let len = path_stack.len();
                for change in changes {
                    Self::apply(change, root_value, path_stack)?;
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
    ) -> Result<(), Self::Error> {
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

fn json_index<'v>(value: &'v mut Value, key: &str, insert: bool) -> Option<&'v mut Value> {
    match value {
        Value::Array(vec) => key.parse::<usize>().ok().and_then(|index| vec.get_mut(index)),
        Value::Object(map) => match insert {
            true => Some(map.entry(Cow::Borrowed(key)).or_insert(Value::Null)),
            false => map.get_mut(key),
        },
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationAdapter;

impl Adapter for MutationAdapter {
    type Replace = ();
    type Append = usize;
    type Error = Infallible;

    fn apply(
        _change: Change<Self>,
        _root_value: &mut Self::Replace,
        _path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn append(
        _old_value: &mut Self::Append,
        _new_value: Self::Append,
        _path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
