use std::collections::BTreeMap;
use std::mem::take;

use crate::{Adapter, Mutation, MutationError, MutationKind, Path, PathSegment};

#[derive(Default)]
enum BatchMutationKind<A: Adapter> {
    #[default]
    None,
    Replace(A::Value),
    #[cfg(feature = "append")]
    Append(A::Value, usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BatchSegmentKind {
    String,
    PosIndex,
    NegIndex,
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
/// use morphix::adapter::Json;
/// use morphix::{BatchTree, Mutation, MutationKind};
/// use serde_json::json;
///
/// let mut batch = BatchTree::<Json>::new();
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
    children: Option<(BatchSegmentKind, BTreeMap<PathSegment, BatchTree<A>>)>,
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
    pub fn load(&mut self, mutation: Mutation<A::Value>) -> Result<(), MutationError> {
        self.load_with_stack(mutation, &mut Default::default())
    }

    fn load_with_stack(
        &mut self,
        mut mutation: Mutation<A::Value>,
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
            #[cfg(feature = "append")]
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
            let new_segment_kind = match &segment {
                PathSegment::String(_) => BatchSegmentKind::String,
                PathSegment::PosIndex(_) => BatchSegmentKind::PosIndex,
                PathSegment::NegIndex(_) => BatchSegmentKind::NegIndex,
            };
            let (old_segment_kind, map) = batch
                .children
                .get_or_insert_with(|| (new_segment_kind, BTreeMap::new()));
            if *old_segment_kind != new_segment_kind {
                return Err(MutationError::IndexError { path: take(path_stack) });
            }
            batch = map.entry(segment).or_default();
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
            #[cfg(feature = "append")]
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
    pub fn dump(&mut self) -> Option<Mutation<A::Value>> {
        let mut mutations = vec![];
        if let Some((_, children)) = take(&mut self.children) {
            for (segment, mut batch) in children {
                if let Some(mut mutation) = batch.dump() {
                    mutation.path.push(segment);
                    mutations.push(mutation);
                }
            }
        }
        match take(&mut self.kind) {
            BatchMutationKind::Replace(value) => {
                mutations.push(Mutation {
                    path: vec![].into(),
                    kind: MutationKind::Replace(value),
                });
            }
            #[cfg(feature = "append")]
            BatchMutationKind::Append(value, _) => {
                mutations.push(Mutation {
                    path: vec![].into(),
                    kind: MutationKind::Append(value),
                });
            }
            BatchMutationKind::None => {}
        }
        Mutation::coalesce(mutations)
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;

    #[test]
    fn batch() {
        let mut batch = BatchTree::<Json>::new();
        assert_eq!(batch.dump(), None);

        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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
        let mut batch = BatchTree::<Json>::new();
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

        let mut batch = BatchTree::<Json>::new();
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
