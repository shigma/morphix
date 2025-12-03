use std::borrow::Cow;
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

enum BatchChildren<A: Adapter> {
    String(BTreeMap<Cow<'static, str>, BatchTree<A>>),
    Positive(BTreeMap<usize, BatchTree<A>>),
    Negative(BTreeMap<usize, BatchTree<A>>),
}

type MappedIntoIter<A, K> = std::iter::Map<
    std::collections::btree_map::IntoIter<K, BatchTree<A>>,
    fn((K, BatchTree<A>)) -> (PathSegment, BatchTree<A>),
>;

enum BatchChildrenIntoIter<A: Adapter> {
    String(MappedIntoIter<A, Cow<'static, str>>),
    Positive(MappedIntoIter<A, usize>),
    Negative(MappedIntoIter<A, usize>),
}

impl<A: Adapter> Iterator for BatchChildrenIntoIter<A> {
    type Item = (PathSegment, BatchTree<A>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BatchChildrenIntoIter::String(iter) => iter.next(),
            BatchChildrenIntoIter::Positive(iter) => iter.next(),
            BatchChildrenIntoIter::Negative(iter) => iter.next(),
        }
    }
}

impl<A: Adapter> IntoIterator for BatchChildren<A> {
    type Item = (PathSegment, BatchTree<A>);
    type IntoIter = BatchChildrenIntoIter<A>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::String(map) => {
                BatchChildrenIntoIter::String(map.into_iter().map(|(k, v)| (PathSegment::String(k), v)))
            }
            Self::Positive(map) => {
                BatchChildrenIntoIter::Positive(map.into_iter().map(|(k, v)| (PathSegment::Positive(k), v)))
            }
            Self::Negative(map) => {
                BatchChildrenIntoIter::Negative(map.into_iter().map(|(k, v)| (PathSegment::Negative(k), v)))
            }
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
    children: Option<BatchChildren<A>>,
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
            if let PathSegment::Negative(index) = &mut segment {
                match &mut batch.kind {
                    #[cfg(feature = "append")]
                    BatchMutationKind::Append(append_value, append_len) => {
                        if *index <= *append_len {
                            mutation.path.push(segment);
                            A::apply_mutation(append_value, mutation, path_stack)?;
                            return Ok(());
                        } else {
                            *index -= *append_len;
                        }
                    }
                    #[cfg(feature = "truncate")]
                    BatchMutationKind::Truncate(truncate_len) => {
                        *index += *truncate_len;
                    }
                    _ => {}
                }
            }

            // We cannot avoid clone here because `BTreeMap::entry` requires owned key.
            path_stack.push(segment.clone());
            let children: &mut BatchChildren<A> = batch.children.get_or_insert_with(|| match &segment {
                PathSegment::String(_) => BatchChildren::String(BTreeMap::new()),
                PathSegment::Positive(_) => BatchChildren::Positive(BTreeMap::new()),
                PathSegment::Negative(_) => BatchChildren::Negative(BTreeMap::new()),
            });
            batch = match (segment, children) {
                (PathSegment::String(key), BatchChildren::String(children)) => children.entry(key).or_default(),
                (PathSegment::Positive(key), BatchChildren::Positive(children)) => children.entry(key).or_default(),
                (PathSegment::Negative(key), BatchChildren::Negative(children)) => children.entry(key).or_default(),
                _ => return Err(MutationError::IndexError { path: take(path_stack) }),
            };
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
            MutationKind::Append(value) => match &mut batch.kind {
                BatchMutationKind::None => {
                    let append_len = A::len(&value, path_stack)?;
                    if append_len == 0 {
                        return Ok(());
                    }
                    batch.kind = BatchMutationKind::Append(value, append_len);
                }
                BatchMutationKind::Replace(_) => unreachable!(),
                BatchMutationKind::Append(old_value, old_len) => {
                    *old_len += A::apply_append(old_value, value, path_stack)?;
                }
                #[cfg(feature = "truncate")]
                BatchMutationKind::Truncate(truncate_len) => {
                    let Some(mut values) = A::into_values(value) else {
                        return Err(MutationError::OperationError { path: take(path_stack) });
                    };
                    let BatchChildren::Negative(children) = batch
                        .children
                        .get_or_insert_with(|| BatchChildren::Negative(BTreeMap::new()))
                    else {
                        return Err(MutationError::IndexError { path: take(path_stack) });
                    };
                    while *truncate_len > 0
                        && let Some(value) = values.next()
                    {
                        children.insert(
                            *truncate_len,
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
            },

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
        if let Some(children) = &mut self.children {
            let BatchChildren::Negative(children) = children else {
                return Err(MutationError::IndexError { path: take(path_stack) });
            };
            children.retain(|index, _| *index > truncate_len);
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
        if let Some(children) = take(&mut self.children) {
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
