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
    #[cfg(feature = "truncate")]
    Truncate(usize),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BatchSegmentKind {
    String,
    Positive,
    Negative,
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
        if let BatchMutationKind::Replace(value) = &mut self.kind {
            A::apply_mutation(value, mutation, path_stack)?;
            return Ok(());
        }

        let mut batch = self;
        while let Some(mut segment) = mutation.path.pop() {
            // We cannot avoid clone here because `BTreeMap::entry` requires owned key.
            path_stack.push(segment.clone());
            if let PathSegment::Negative(index) = &mut segment {
                match &mut batch.kind {
                    #[cfg(feature = "append")]
                    BatchMutationKind::Append(value, len) => {
                        if *index <= *len {
                            mutation.path.push(segment);
                            path_stack.pop();
                            A::apply_mutation(value, mutation, path_stack)?;
                            return Ok(());
                        } else {
                            *index -= *len;
                        }
                    }
                    #[cfg(feature = "truncate")]
                    BatchMutationKind::Truncate(truncate_len) => {
                        *index += *truncate_len;
                    }
                    _ => {}
                }
            }
            let new_segment_kind = match &segment {
                PathSegment::String(_) => BatchSegmentKind::String,
                PathSegment::Positive(_) => BatchSegmentKind::Positive,
                PathSegment::Negative(_) => BatchSegmentKind::Negative,
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
                batch.children.take();
            }

            MutationKind::Batch(mutations) => {
                let len = path_stack.len();
                for mutation in mutations {
                    batch.load_with_stack(mutation, path_stack)?;
                    path_stack.truncate(len);
                }
            }

            #[cfg(feature = "append")]
            MutationKind::Append(value) => {
                let append_len = A::get_len(&value, path_stack)?;
                if append_len == 0 {
                    return Ok(());
                }
                match &mut batch.kind {
                    BatchMutationKind::None => batch.kind = BatchMutationKind::Append(value, append_len),
                    BatchMutationKind::Replace(_) => unreachable!(),
                    BatchMutationKind::Append(old_value, old_len) => {
                        *old_len += append_len;
                        A::merge_append(old_value, value, path_stack)?;
                    }
                    #[cfg(feature = "truncate")]
                    BatchMutationKind::Truncate(truncate_len) => {
                        let Some(mut values) = A::into_values(value) else {
                            return Err(MutationError::OperationError { path: take(path_stack) });
                        };
                        let (old_segment_kind, children) = batch
                            .children
                            .get_or_insert_with(|| (BatchSegmentKind::Negative, BTreeMap::new()));
                        if *old_segment_kind != BatchSegmentKind::Negative {
                            return Err(MutationError::IndexError { path: take(path_stack) });
                        }
                        while *truncate_len > 0
                            && let Some(value) = values.next()
                        {
                            children.insert(
                                PathSegment::Negative(*truncate_len),
                                BatchTree {
                                    kind: BatchMutationKind::Replace(value),
                                    children: None,
                                },
                            );
                            *truncate_len -= 1;
                        }
                        if *truncate_len > 0 {
                            return Ok(());
                        }
                        let append_len = values.len();
                        if append_len == 0 {
                            batch.kind = BatchMutationKind::None;
                        } else {
                            batch.kind = BatchMutationKind::Append(A::from_values(values), append_len);
                        }
                    }
                }
            }

            #[cfg(feature = "truncate")]
            MutationKind::Truncate(mut truncate_len) => {
                if truncate_len == 0 {
                    return Ok(());
                }
                match &mut batch.kind {
                    BatchMutationKind::None => batch.kind = BatchMutationKind::Truncate(truncate_len),
                    BatchMutationKind::Replace(_) => unreachable!(),
                    #[cfg(feature = "append")]
                    BatchMutationKind::Append(value, old_len) => {
                        if A::apply_truncate(value, truncate_len, path_stack)?.is_some() {
                            truncate_len -= *old_len;
                            batch.kind = BatchMutationKind::Truncate(truncate_len);
                            batch.apply_truncate(truncate_len, path_stack)?;
                        } else if *old_len == truncate_len {
                            batch.kind = BatchMutationKind::None;
                        }
                    }
                    BatchMutationKind::Truncate(old_len) => {
                        *old_len += truncate_len;
                        let truncate_len = *old_len;
                        batch.apply_truncate(truncate_len, path_stack)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn apply_truncate(&mut self, truncate_len: usize, path_stack: &mut Path<false>) -> Result<(), MutationError> {
        if let Some((kind, children)) = &mut self.children {
            if *kind != BatchSegmentKind::Negative {
                return Err(MutationError::IndexError { path: take(path_stack) });
            }
            children.retain(|segment, _| {
                let PathSegment::Negative(index) = segment else {
                    unreachable!()
                };
                *index > truncate_len
            });
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
            BatchMutationKind::None => {}
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
            #[cfg(feature = "truncate")]
            BatchMutationKind::Truncate(truncate_len) => {
                mutations.push(Mutation {
                    path: vec![].into(),
                    kind: MutationKind::Truncate(truncate_len),
                });
            }
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
