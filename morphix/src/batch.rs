use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::mem::take;

use crate::adapter::Adapter;
use crate::change::{Change, Operation};
use crate::error::ChangeError;

pub struct Batch<A: Adapter> {
    operation: Option<Operation<A>>,
    children: BTreeMap<Cow<'static, str>, Self>,
}

impl<A: Adapter> Default for Batch<A> {
    fn default() -> Self {
        Self {
            operation: None,
            children: BTreeMap::new(),
        }
    }
}

impl<A: Adapter> Debug for Batch<A>
where
    A::Replace: Debug,
    A::Append: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batch")
            .field("operation", &self.operation)
            .field("children", &self.children)
            .finish()
    }
}

impl<A: Adapter> Batch<A> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn load(&mut self, change: Change<A>) -> Result<(), ChangeError> {
        self.load_with_stack(change, &mut vec![])
    }

    fn load_with_stack(
        &mut self,
        mut change: Change<A>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError> {
        let mut batch = self;
        if let Some(Operation::Replace(value)) = &mut batch.operation {
            A::apply_change(value, change, path_stack)?;
            return Ok(());
        }
        while let Some(key) = change.path_rev.pop() {
            // We cannot avoid allocation here because `BTreeMap::entry` requires owned key.
            path_stack.push(key.clone());
            batch = batch.children.entry(key).or_default();
            if let Some(Operation::Replace(value)) = &mut batch.operation {
                A::apply_change(value, change, path_stack)?;
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
                    A::merge_append(old_value, new_value, path_stack)?;
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

    pub fn dump(&mut self) -> Option<Change<A>> {
        let mut changes = vec![];
        if let Some(operation) = self.operation.take() {
            changes.push(Change {
                path_rev: vec![],
                operation,
            });
        }
        for (key, mut batch) in take(&mut self.children) {
            if let Some(mut change) = batch.dump() {
                change.path_rev.push(key);
                changes.push(change);
            }
        }
        Self::build(changes)
    }

    pub fn build(mut changes: Vec<Change<A>>) -> Option<Change<A>> {
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
    use crate::JsonAdapter;

    #[test]
    fn batch() {
        let mut batch = Batch::<JsonAdapter>::new();
        assert_eq!(batch.dump(), None);

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!(1)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!(1))
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!(1)),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!(2)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!(2)),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["qux".into(), "bar".into(), "foo".into()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!({"qux": "12"})),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["qux".into(), "bar".into(), "foo".into()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Replace(json!({"qux": "1"})),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["foo".into()],
                operation: Operation::Batch(vec![
                    Change {
                        path_rev: vec!["bar".into()],
                        operation: Operation::Append(json!("1")),
                    },
                    Change {
                        path_rev: vec!["bar".into()],
                        operation: Operation::Append(json!("2")),
                    },
                ]),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Append(json!("12")),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["bar".into()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["qux".into()],
                operation: Operation::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec![],
                operation: Operation::Batch(vec![
                    Change {
                        path_rev: vec!["bar".into()],
                        operation: Operation::Append(json!("2")),
                    },
                    Change {
                        path_rev: vec!["qux".into()],
                        operation: Operation::Append(json!("1")),
                    },
                ]),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Change {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: Operation::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Change {
                path_rev: vec!["qux".into(), "foo".into()],
                operation: Operation::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Change {
                path_rev: vec!["foo".into()],
                operation: Operation::Batch(vec![
                    Change {
                        path_rev: vec!["bar".into()],
                        operation: Operation::Append(json!("2")),
                    },
                    Change {
                        path_rev: vec!["qux".into()],
                        operation: Operation::Append(json!("1")),
                    },
                ]),
            }),
        );
    }
}
