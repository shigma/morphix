use std::collections::BTreeMap;
use std::fmt::Debug;
use std::mem::take;

use crate::{Adapter, Mutation, MutationError, MutationKind, Path, PathSegment};

/// A batch collector for aggregating and optimizing multiple mutations.
///
/// `Batch` is used internally to collect multiple mutations and optimize them before creating the
/// final mutation. It can merge consecutive append operations and eliminate redundant mutations.
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
///     path: vec!["field".into()].into(),
///     kind: MutationKind::Replace(json!(1)),
/// }).unwrap();
///
/// // Dump optimized mutations
/// let optimized = batch.dump();
/// ```
pub struct Batch<A: Adapter> {
    kind: Option<MutationKind<A>>,
    children: BTreeMap<PathSegment, Self>,
}

impl<A: Adapter> Default for Batch<A> {
    fn default() -> Self {
        Self {
            kind: None,
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
            .field("operation", &self.kind)
            .field("children", &self.children)
            .finish()
    }
}

impl<A: Adapter> Batch<A> {
    /// Creates a new empty batch.
    pub fn new() -> Self {
        Default::default()
    }

    /// Loads a [`Mutation`] into the batch, potentially merging with existing mutations.
    ///
    /// ## Arguments
    ///
    /// - `mutation` - mutation to add to the batch
    ///
    /// ## Errors
    ///
    /// - Returns an [MutationError] if the mutation cannot be applied.
    pub fn load(&mut self, mutation: Mutation<A>) -> Result<(), MutationError> {
        self.load_with_stack(mutation, &mut Default::default())
    }

    fn load_with_stack(
        &mut self,
        mut mutation: Mutation<A>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        let mut batch = self;
        if let Some(MutationKind::Replace(value)) = &mut batch.kind {
            A::apply_mutation(value, mutation, path_stack)?;
            return Ok(());
        }
        while let Some(segment) = mutation.path.pop() {
            // We cannot avoid allocation here because `BTreeMap::entry` requires owned key.
            path_stack.push(segment.clone());
            batch = batch.children.entry(segment).or_default();
            if let Some(MutationKind::Replace(value)) = &mut batch.kind {
                A::apply_mutation(value, mutation, path_stack)?;
                return Ok(());
            }
        }

        match mutation.kind {
            MutationKind::Replace(_) => {
                batch.kind = Some(mutation.kind);
                batch.children.clear();
            }
            MutationKind::Append(new_value) => match &mut batch.kind {
                Some(MutationKind::Append(old_value)) => {
                    A::merge_append(old_value, new_value, path_stack)?;
                }
                Some(_) => unreachable!(),
                None => batch.kind = Some(MutationKind::Append(new_value)),
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
        if let Some(operation) = self.kind.take() {
            mutations.push(Mutation {
                path: vec![].into(),
                kind: operation,
            });
        }
        for (key, mut batch) in take(&mut self.children) {
            if let Some(mut mutation) = batch.dump() {
                mutation.path.push(key);
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
                path: vec![].into(),
                kind: MutationKind::Batch(mutations),
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
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(1)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(1))
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(1)),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(2)),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(2)),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("2")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!({"qux": "12"})),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!({"qux": "1"})),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!({"qux": "1"})),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into()].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec!["bar".into()].into(),
                        kind: MutationKind::Append(json!("1")),
                    },
                    Mutation {
                        path: vec!["bar".into()].into(),
                        kind: MutationKind::Append(json!("2")),
                    },
                ]),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Append(json!("12")),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec!["bar".into()].into(),
                kind: MutationKind::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["qux".into()].into(),
                kind: MutationKind::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec!["bar".into()].into(),
                        kind: MutationKind::Append(json!("2")),
                    },
                    Mutation {
                        path: vec!["qux".into()].into(),
                        kind: MutationKind::Append(json!("1")),
                    },
                ]),
            }),
        );

        let mut batch = Batch::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Append(json!("2")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("1")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec!["foo".into()].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec!["bar".into()].into(),
                        kind: MutationKind::Append(json!("2")),
                    },
                    Mutation {
                        path: vec!["qux".into()].into(),
                        kind: MutationKind::Append(json!("1")),
                    },
                ]),
            }),
        );
    }
}
