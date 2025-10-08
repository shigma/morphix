use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::mem::take;

use crate::{Adapter, Mutation, MutationError, MutationKind};

/// A batch collector for aggregating and optimizing multiple mutations.
///
/// `Batch` is used internally to collect multiple mutations and optimize them
/// before creating the final mutation. It can merge consecutive append
/// operations and eliminate redundant mutations.
///
/// ## Type Parameters
///
/// - `A` - The adapter type used for serialization
///
/// ## Example
///
/// ```
/// use morphix::{Batch, JsonAdapter, Mutation, MutationKind};
/// use serde_json::json;
///
/// let mut batch = Batch::<JsonAdapter>::new();
///
/// // Load multiple mutations
/// batch.load(Mutation {
///     path_rev: vec!["field".into()],
///     operation: MutationKind::Replace(json!(1)),
/// }).unwrap();
///
/// // Dump optimized mutations
/// let optimized = batch.dump();
/// ```
pub struct Batch<A: Adapter> {
    operation: Option<MutationKind<A>>,
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
    A::Value: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Batch")
            .field("operation", &self.operation)
            .field("children", &self.children)
            .finish()
    }
}

impl<A: Adapter> Batch<A> {
    /// Creates a new empty batch.
    pub fn new() -> Self {
        Default::default()
    }

    /// Loads a [Mutation] into the batch, potentially merging with existing mutations.
    ///
    /// ## Arguments
    ///
    /// - `mutation` - mutation to add to the batch
    ///
    /// ## Errors
    ///
    /// - Returns an [MutationError] if the mutation cannot be applied.
    pub fn load(&mut self, mutation: Mutation<A>) -> Result<(), MutationError> {
        self.load_with_stack(mutation, &mut vec![])
    }

    fn load_with_stack(
        &mut self,
        mut mutation: Mutation<A>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), MutationError> {
        let mut batch = self;
        if let Some(MutationKind::Replace(value)) = &mut batch.operation {
            A::apply_change(value, mutation, path_stack)?;
            return Ok(());
        }
        while let Some(key) = mutation.path_rev.pop() {
            // We cannot avoid allocation here because `BTreeMap::entry` requires owned key.
            path_stack.push(key.clone());
            batch = batch.children.entry(key).or_default();
            if let Some(MutationKind::Replace(value)) = &mut batch.operation {
                A::apply_change(value, mutation, path_stack)?;
                return Ok(());
            }
        }

        match mutation.operation {
            MutationKind::Replace(_) => {
                batch.operation = Some(mutation.operation);
                batch.children.clear();
            }
            MutationKind::Append(new_value) => match &mut batch.operation {
                Some(MutationKind::Append(old_value)) => {
                    A::merge_append(old_value, new_value, path_stack)?;
                }
                Some(_) => unreachable!(),
                None => batch.operation = Some(MutationKind::Append(new_value)),
            },
            MutationKind::Batch(mutations) => {
                let len = path_stack.len();
                for mutation in mutations {
                    batch.load_with_stack(mutation, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }

    /// Dumps all accumulated mutations as a single optimized mutation.
    ///
    /// - Returns `None` if no mutations have been accumulated.
    /// - Returns a single mutation if only one mutation exists.
    /// - Returns a `Batch` mutation if multiple mutations exist.
    pub fn dump(&mut self) -> Option<Mutation<A>> {
        let mut mutations = vec![];
        if let Some(operation) = self.operation.take() {
            mutations.push(Mutation {
                path_rev: vec![],
                operation,
            });
        }
        for (key, mut batch) in take(&mut self.children) {
            if let Some(mut mutation) = batch.dump() {
                mutation.path_rev.push(key);
                mutations.push(mutation);
            }
        }
        Self::build(mutations)
    }

    #[doc(hidden)]
    pub fn build(mut mutations: Vec<Mutation<A>>) -> Option<Mutation<A>> {
        match mutations.len() {
            0 => None,
            1 => Some(mutations.swap_remove(0)),
            _ => Some(Mutation {
                path_rev: vec![],
                operation: MutationKind::Batch(mutations),
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
            .load(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!(1)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!(1))
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!(1)),
            })
            .unwrap();
        batch
            .load(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!(2)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!(2)),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        batch
            .load(Mutation {
                path_rev: vec!["qux".into(), "bar".into(), "foo".into()],
                operation: MutationKind::Append(json!("2")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!({"qux": "12"})),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path_rev: vec!["qux".into(), "bar".into(), "foo".into()],
                operation: MutationKind::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Replace(json!({"qux": "1"})),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path_rev: vec!["foo".into()],
                operation: MutationKind::Batch(vec![
                    Mutation {
                        path_rev: vec!["bar".into()],
                        operation: MutationKind::Append(json!("1")),
                    },
                    Mutation {
                        path_rev: vec!["bar".into()],
                        operation: MutationKind::Append(json!("2")),
                    },
                ]),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Append(json!("12")),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path_rev: vec!["bar".into()],
                operation: MutationKind::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path_rev: vec!["qux".into()],
                operation: MutationKind::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec![],
                operation: MutationKind::Batch(vec![
                    Mutation {
                        path_rev: vec!["bar".into()],
                        operation: MutationKind::Append(json!("2")),
                    },
                    Mutation {
                        path_rev: vec!["qux".into()],
                        operation: MutationKind::Append(json!("1")),
                    },
                ]),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path_rev: vec!["bar".into(), "foo".into()],
                operation: MutationKind::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path_rev: vec!["qux".into(), "foo".into()],
                operation: MutationKind::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path_rev: vec!["foo".into()],
                operation: MutationKind::Batch(vec![
                    Mutation {
                        path_rev: vec!["bar".into()],
                        operation: MutationKind::Append(json!("2")),
                    },
                    Mutation {
                        path_rev: vec!["qux".into()],
                        operation: MutationKind::Append(json!("1")),
                    },
                ]),
            }),
        );
    }
}
