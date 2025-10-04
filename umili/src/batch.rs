use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::change::{Change, Operation};
use crate::operation::{Operator, ValueOperator};

pub struct Batch<O: Operator = ValueOperator> {
    operation: Option<Operation<O>>,
    children: BTreeMap<String, Self>,
}

impl<O: Operator> Default for Batch<O> {
    fn default() -> Self {
        Self {
            operation: None,
            children: BTreeMap::new(),
        }
    }
}

impl<O: Operator> Debug for Batch<O>
where
    O::Replace: Debug,
    O::Append: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batch")
            .field("operation", &self.operation)
            .field("children", &self.children)
            .finish()
    }
}

impl<O: Operator> Batch<O> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load(&mut self, change: Change<O>) -> Result<(), O::Error> {
        self.load_with_stack(change, &mut vec![])
    }

    fn load_with_stack(&mut self, mut change: Change<O>, path_stack: &mut Vec<String>) -> Result<(), O::Error> {
        let mut batch = self;
        if let Some(Operation::Replace(value)) = &mut batch.operation {
            O::apply(change, value, path_stack)?;
            return Ok(());
        }
        while let Some(key) = change.path_rev.pop() {
            path_stack.push(key.clone()); // TODO: avoid clone
            batch = batch.children.entry(key).or_default();
            if let Some(Operation::Replace(value)) = &mut batch.operation {
                O::apply(change, value, path_stack)?;
                return Ok(());
            }
        }

        match change.operation {
            Operation::Replace(_) => {
                batch.operation = Some(change.operation);
                batch.children.clear();
            }
            Operation::Append(new_value) => match &mut batch.operation {
                Some(Operation::Append(old_value)) => {
                    O::append(old_value, new_value, path_stack)?;
                }
                Some(_) => unreachable!(),
                None => batch.operation = Some(Operation::Append(new_value)),
            },
            Operation::Batch(changes) => {
                let len = path_stack.len();
                for change in changes {
                    batch.load_with_stack(change, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }

    pub fn dump(self) -> Option<Change<O>> {
        let mut changes = vec![];
        if let Some(operation) = self.operation {
            changes.push(Change {
                path_rev: vec![],
                operation,
            });
        }
        for (key, batch) in self.children {
            if let Some(mut change) = batch.dump() {
                change.path_rev.push(key);
                changes.push(change);
            }
        }
        match changes.len() {
            0 => None,
            1 => Some(changes.swap_remove(0)),
            _ => Some(Change {
                path_rev: vec![],
                operation: Operation::Batch(changes),
            }),
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn batch() {
        let batch = Batch::<ValueOperator>::new();
        assert_eq!(batch.dump(), None);

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!(1)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!(1))
            }),
        );

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!(1)),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!(2)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!(2)),
            }),
        );

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string(), "qux".to_string()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!({"qux": "12"})),
            }),
        );

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string(), "qux".to_string()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Replace(json!({"qux": "1"})),
            }),
        );

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string()],
                operation: Operation::Batch(vec![
                    Change {
                        path_rev: vec!["bar".to_string()],
                        operation: Operation::Append(json!("1")),
                    },
                    Change {
                        path_rev: vec!["bar".to_string()],
                        operation: Operation::Append(json!("2")),
                    },
                ]),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Append(json!("12")),
            }),
        );

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["bar".to_string()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["qux".to_string()],
                operation: Operation::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec![],
                operation: Operation::Batch(vec![
                    Change {
                        path_rev: vec!["bar".to_string()],
                        operation: Operation::Append(json!("2")),
                    },
                    Change {
                        path_rev: vec!["qux".to_string()],
                        operation: Operation::Append(json!("1")),
                    },
                ]),
            }),
        );

        let mut batch = Batch::<ValueOperator>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "bar".to_string()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["foo".to_string(), "qux".to_string()],
                operation: Operation::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec![],
                operation: Operation::Batch(vec![
                    Change {
                        path_rev: vec!["bar".to_string()],
                        operation: Operation::Append(json!("2")),
                    },
                    Change {
                        path_rev: vec!["qux".to_string()],
                        operation: Operation::Append(json!("1")),
                    },
                ]),
            }),
        );
    }
}
