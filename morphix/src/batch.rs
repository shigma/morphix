use std::borrow::Cow;
use std::collections::BTreeMap;
use std::mem::take;

use crate::{Adapter, Mutation, MutationError, MutationKind, Path, PathSegment};

#[derive(Default)]
enum BatchMutationKind<A: Adapter> {
    #[default]
    None,
    Replace(A::Value),
    Append(A::Value, usize),
}

#[derive(Default)]
enum BatchChildren<A: Adapter> {
    #[default]
    None,
    String(BTreeMap<Cow<'static, str>, BatchTree<A>>),
    PosIndex(BTreeMap<usize, BatchTree<A>>),
    NegIndex(BTreeMap<usize, BatchTree<A>>),
}

impl<A: Adapter> BatchChildren<A> {
    fn try_entry_or_default(&mut self, segment: PathSegment) -> Option<&mut BatchTree<A>> {
        if let Self::None = &self {
            *self = match &segment {
                PathSegment::String(_) => Self::String(Default::default()),
                PathSegment::PosIndex(_) => Self::PosIndex(Default::default()),
                PathSegment::NegIndex(_) => Self::NegIndex(Default::default()),
            };
        }
        Some(match (self, segment) {
            (Self::String(map), PathSegment::String(key)) => map.entry(key).or_default(),
            (Self::PosIndex(map), PathSegment::PosIndex(key)) => map.entry(key).or_default(),
            (Self::NegIndex(map), PathSegment::NegIndex(key)) => map.entry(key).or_default(),
            _ => return None,
        })
    }
}

impl<A: Adapter> IntoIterator for BatchChildren<A> {
    type Item = (PathSegment, BatchTree<A>);
    type IntoIter = BatchChildrenIntoIter<A>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::None => BatchChildrenIntoIter::None,
            Self::String(map) => BatchChildrenIntoIter::String(map.into_iter()),
            Self::PosIndex(map) => BatchChildrenIntoIter::PosIndex(map.into_iter()),
            Self::NegIndex(map) => BatchChildrenIntoIter::NegIndex(map.into_iter()),
        }
    }
}

enum BatchChildrenIntoIter<A: Adapter> {
    None,
    String(std::collections::btree_map::IntoIter<Cow<'static, str>, BatchTree<A>>),
    PosIndex(std::collections::btree_map::IntoIter<usize, BatchTree<A>>),
    NegIndex(std::collections::btree_map::IntoIter<usize, BatchTree<A>>),
}

impl<A: Adapter> Iterator for BatchChildrenIntoIter<A> {
    type Item = (PathSegment, BatchTree<A>);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::None => None,
            Self::String(iter) => iter.next().map(|(k, v)| (PathSegment::String(k), v)),
            Self::PosIndex(iter) => iter.next().map(|(k, v)| (PathSegment::PosIndex(k), v)),
            Self::NegIndex(iter) => iter.next().map(|(k, v)| (PathSegment::NegIndex(k), v)),
        }
    }
}

/// A batch collector for aggregating and optimizing multiple mutations.
///
/// `BatchTree` is used internally to collect multiple mutations and optimize them before creating
/// the final mutation. It can merge consecutive append operations and eliminate redundant
/// mutations.
///
/// ## Example
///
/// ```
/// use morphix::{BatchTree, JsonAdapter, Mutation, MutationKind};
/// use serde_json::json;
///
/// let mut batch = BatchTree::<JsonAdapter>::new();
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
pub struct BatchTree<A: Adapter> {
    kind: BatchMutationKind<A>,
    children: BatchChildren<A>,
}

impl<A: Adapter> Default for BatchTree<A> {
    fn default() -> Self {
        Self {
            kind: Default::default(),
            children: Default::default(),
        }
    }
}

impl<A: Adapter> BatchTree<A> {
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
        if let BatchMutationKind::Replace(value) = &mut batch.kind {
            A::apply_mutation(value, mutation, path_stack)?;
            return Ok(());
        }
        while let Some(mut segment) = mutation.path.pop() {
            // We cannot avoid clone here because `BTreeMap::entry` requires owned key.
            path_stack.push(segment.clone());
            if let PathSegment::NegIndex(index) = &mut segment
                && let BatchMutationKind::Append(value, len) = &mut batch.kind
            {
                if *index <= *len {
                    mutation.path.push(segment);
                    path_stack.pop();
                    A::apply_mutation(value, mutation, path_stack)?;
                    return Ok(());
                } else {
                    *index -= *len;
                }
            }
            batch = batch
                .children
                .try_entry_or_default(segment)
                .ok_or_else(|| MutationError::IndexError { path: take(path_stack) })?;
            if let BatchMutationKind::Replace(value) = &mut batch.kind {
                A::apply_mutation(value, mutation, path_stack)?;
                return Ok(());
            }
        }

        match mutation.kind {
            MutationKind::Replace(value) => {
                batch.kind = BatchMutationKind::Replace(value);
                take(&mut batch.children);
            }
            MutationKind::Append(append_value) => {
                let append_len = A::get_len(&append_value, path_stack)?;
                match &mut batch.kind {
                    BatchMutationKind::Append(value, len) => {
                        *len += append_len;
                        A::merge_append(value, append_value, path_stack)?;
                    }
                    BatchMutationKind::Replace(_) => unreachable!(),
                    BatchMutationKind::None => batch.kind = BatchMutationKind::Append(append_value, append_len),
                }
            }
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
    /// - Returns [`None`] if no mutations have been accumulated.
    /// - Returns a single mutation if only one mutation exists.
    /// - Returns a [`Batch`](MutationKind::Batch) mutation if multiple mutations exist.
    pub fn dump(&mut self) -> Option<Mutation<A>> {
        let mut mutations = vec![];
        for (segment, mut batch) in take(&mut self.children) {
            if let Some(mut mutation) = batch.dump() {
                mutation.path.push(segment);
                mutations.push(mutation);
            }
        }
        match take(&mut self.kind) {
            BatchMutationKind::Replace(value) => {
                mutations.push(Mutation {
                    path: vec![].into(),
                    kind: MutationKind::Replace(value),
                });
            }
            BatchMutationKind::Append(value, _) => {
                mutations.push(Mutation {
                    path: vec![].into(),
                    kind: MutationKind::Append(value),
                });
            }
            BatchMutationKind::None => {}
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
        let mut batch = BatchTree::<JsonAdapter>::new();
        assert_eq!(batch.dump(), None);

        let mut batch = BatchTree::<JsonAdapter>::new();
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

        let mut batch = BatchTree::<JsonAdapter>::new();
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

        let mut batch = BatchTree::<JsonAdapter>::new();
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

        let mut batch = BatchTree::<JsonAdapter>::new();
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

        let mut batch = BatchTree::<JsonAdapter>::new();
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

        let mut batch = BatchTree::<JsonAdapter>::new();
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

        let mut batch = BatchTree::<JsonAdapter>::new();
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

    #[test]
    fn index_rev() {
        let mut batch = BatchTree::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec![(-1).into()].into(),
                kind: MutationKind::Append(json!("c")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(["a", "b"])),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![(-1).into()].into(),
                kind: MutationKind::Append(json!("d")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![(-3).into()].into(),
                kind: MutationKind::Append(json!("e")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![(-1).into()].into(),
                        kind: MutationKind::Append(json!("ce")),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!(["a", "bd"])),
                    },
                ]),
            }),
        );

        let mut batch = BatchTree::<JsonAdapter>::new();
        batch
            .load(Mutation {
                path: vec![(-1).into()].into(),
                kind: MutationKind::Append(json!("c")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(["a", "b"])),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![(-2).into()].into(),
                kind: MutationKind::Append(json!("d")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(["e", "f"])),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![(-3).into()].into(),
                kind: MutationKind::Append(json!("g")),
            })
            .unwrap();
        assert_eq!(
            batch.dump(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![(-1).into()].into(),
                        kind: MutationKind::Append(json!("c")),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!(["ad", "bg", "e", "f"])),
                    },
                ]),
            }),
        );
    }
}
