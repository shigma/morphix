use std::borrow::Cow;
use std::collections::BTreeMap;
use std::mem::take;

use crate::{Adapter, Mutation, MutationError, MutationKind, Mutations, Path, PathSegment};

#[derive(Default)]
enum BatchMutationKind<A: Adapter> {
    #[default]
    None,
    Replace(A::Value),
    #[cfg(any(feature = "append", feature = "truncate"))]
    TruncateAppend {
        truncate_len: usize,
        append_len: usize,
        append_value: Option<A::Value>,
    },
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
            Self::String(iter) => iter.next(),
            Self::Positive(iter) => iter.next(),
            Self::Negative(iter) => iter.next(),
        }
    }
}

impl<A: Adapter> IntoIterator for BatchChildren<A> {
    type Item = (PathSegment, BatchTree<A>);
    type IntoIter = BatchChildrenIntoIter<A>;

    #[inline]
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
/// [`BatchTree`] is used internally to collect multiple mutations and optimize them before creating
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
            A::mutate(value, mutation, path_stack)?;
            return Ok(());
        }

        let mut batch = self;
        while let Some(mut segment) = mutation.path.pop() {
            #[cfg(any(feature = "append", feature = "truncate"))]
            if let PathSegment::Negative(index) = &mut segment
                && let BatchMutationKind::TruncateAppend {
                    truncate_len,
                    append_len,
                    append_value,
                } = &mut batch.kind
            {
                if *index <= *append_len {
                    mutation.path.push(segment);
                    // SAFETY: negative index must be non-zero, so `append_len` is non-zero here,
                    // which means `append_value` must be `Some`.
                    A::mutate(append_value.as_mut().unwrap(), mutation, path_stack)?;
                    return Ok(());
                } else {
                    *index -= *append_len;
                }
                *index += *truncate_len;
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
                A::mutate(value, mutation, path_stack)?;
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
                BatchMutationKind::Replace(_) => unreachable!(),
                BatchMutationKind::None => {
                    let Some(append_len) = A::len(&value) else {
                        return Err(MutationError::OperationError { path: take(path_stack) });
                    };
                    if append_len == 0 {
                        return Ok(());
                    }
                    batch.kind = BatchMutationKind::TruncateAppend {
                        truncate_len: 0,
                        append_len,
                        append_value: Some(value),
                    };
                }
                BatchMutationKind::TruncateAppend {
                    truncate_len: _,
                    append_len,
                    append_value,
                } => {
                    if let Some(append_value) = append_value {
                        let Some(len) = A::append(append_value, value) else {
                            return Err(MutationError::OperationError { path: take(path_stack) });
                        };
                        *append_len += len;
                    } else {
                        let Some(len) = A::len(&value) else {
                            return Err(MutationError::OperationError { path: take(path_stack) });
                        };
                        *append_len = len;
                        *append_value = Some(value);
                    }
                }
            },

            #[cfg(feature = "truncate")]
            MutationKind::Truncate(len) => match &mut batch.kind {
                BatchMutationKind::Replace(_) => unreachable!(),
                BatchMutationKind::None => {
                    if len == 0 {
                        return Ok(());
                    }
                    batch.kind = BatchMutationKind::TruncateAppend {
                        truncate_len: len,
                        append_len: 0,
                        append_value: None,
                    };
                    if let Some(children) = &mut batch.children {
                        let BatchChildren::Negative(children) = children else {
                            return Err(MutationError::IndexError { path: take(path_stack) });
                        };
                        children.retain(|index, _| *index > len);
                    }
                }
                BatchMutationKind::TruncateAppend {
                    truncate_len,
                    append_len,
                    append_value,
                } => {
                    let remaining = if let Some(append_value) = append_value {
                        let Some(remaining) = A::truncate(append_value, len) else {
                            return Err(MutationError::OperationError { path: take(path_stack) });
                        };
                        *append_len -= len - remaining;
                        remaining
                    } else {
                        len
                    };
                    *truncate_len += remaining;
                    if *append_len == 0 && *truncate_len == 0 {
                        batch.kind = BatchMutationKind::None;
                    } else if remaining > 0
                        && let Some(children) = &mut batch.children
                    {
                        let BatchChildren::Negative(children) = children else {
                            return Err(MutationError::IndexError { path: take(path_stack) });
                        };
                        children.retain(|index, _| *index > *truncate_len);
                    }
                }
            },

            #[cfg(feature = "delete")]
            MutationKind::Delete => todo!(),
        }

        Ok(())
    }

    /// Dumps all accumulated mutations as a single optimized mutation.
    ///
    /// - Returns [`None`] if no mutations have been accumulated.
    /// - Returns a single mutation if only one mutation exists.
    /// - Returns a [`Batch`](MutationKind::Batch) mutation if multiple mutations exist.
    pub fn dump(&mut self) -> Mutations<A::Value> {
        let mut mutations = Mutations::new();
        if let Some(children) = take(&mut self.children) {
            for (segment, mut batch) in children {
                mutations.insert(segment, batch.dump());
            }
        }
        match take(&mut self.kind) {
            BatchMutationKind::None => {}
            BatchMutationKind::Replace(value) => {
                mutations.extend(MutationKind::Replace(value));
            }
            #[cfg(any(feature = "append", feature = "truncate"))]
            BatchMutationKind::TruncateAppend {
                truncate_len,
                append_len,
                append_value,
            } => {
                #[cfg(feature = "truncate")]
                if truncate_len > 0 {
                    mutations.extend(MutationKind::Truncate(truncate_len));
                }
                #[cfg(feature = "append")]
                if append_len > 0
                    && let Some(value) = append_value
                {
                    mutations.extend(MutationKind::Append(value));
                }
            }
        }
        mutations
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;

    #[test]
    fn empty_batch() {
        let mut batch = BatchTree::<Json>::new();
        assert_eq!(batch.dump().into_inner(), None);
    }

    #[test]
    fn replace() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(1)),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(1))
            }),
        );
    }

    #[test]
    fn replace_on_replace() {
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
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!(2)),
            }),
        );
    }

    #[test]
    fn append_on_replace() {
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
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!({"qux": "12"})),
            }),
        );
    }

    #[test]
    fn replace_after_append() {
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
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Replace(json!({"qux": "1"})),
            }),
        );
    }

    #[test]
    fn merge_append() {
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
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Append(json!("12")),
            }),
        );
    }

    #[test]
    fn basic_batch() {
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
            batch.dump().into_inner(),
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
    }

    #[test]
    fn nested_batch() {
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
            batch.dump().into_inner(),
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
    fn append_with_neg_index_1() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
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
                path: vec![PathSegment::Negative(1)].into(),
                kind: MutationKind::Append(json!("d")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(3)].into(),
                kind: MutationKind::Append(json!("e")),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![PathSegment::Negative(1)].into(),
                        kind: MutationKind::Append(json!("ce")),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!(["a", "bd"])),
                    },
                ]),
            }),
        );
    }

    #[test]
    fn append_with_neg_index_2() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
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
                path: vec![PathSegment::Negative(2)].into(),
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
                path: vec![PathSegment::Negative(3)].into(),
                kind: MutationKind::Append(json!("g")),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![PathSegment::Negative(1)].into(),
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

    #[test]
    fn merge_truncate() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into()].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec!["bar".into()].into(),
                        kind: MutationKind::Truncate(1),
                    },
                    Mutation {
                        path: vec!["bar".into()].into(),
                        kind: MutationKind::Truncate(2),
                    },
                ]),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into()].into(),
                kind: MutationKind::Truncate(3),
            }),
        );
    }

    #[test]
    fn truncate_on_append_1() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("42")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(1),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("4")),
            }),
        );
    }

    #[test]
    fn truncate_on_append_2() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("42")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(2),
            })
            .unwrap();
        assert_eq!(batch.dump().into_inner(), None);
    }

    #[test]
    fn truncate_on_append_3() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("42")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(3),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(1),
            }),
        );
    }

    #[test]
    fn append_after_truncate_1() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(3),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("Hello, World!")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(1),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Truncate(3),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!("Hello, World")),
                    },
                ]),
            }),
        );
    }

    #[test]
    fn append_after_truncate_2() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(3),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Append(json!("42")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(3),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec!["foo".into(), "bar".into(), "qux".into()].into(),
                kind: MutationKind::Truncate(4),
            }),
        );
    }

    #[test]
    fn truncate_with_neg_index_1() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
                kind: MutationKind::Truncate(1),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Truncate(2),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
                kind: MutationKind::Truncate(3),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![PathSegment::Negative(3)].into(),
                        kind: MutationKind::Truncate(3),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Truncate(2),
                    },
                ]),
            }),
        );
    }

    #[test]
    fn truncate_with_neg_index_2() {
        let mut batch = BatchTree::<Json>::new();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Truncate(2),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(2)].into(),
                kind: MutationKind::Truncate(3),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Truncate(1),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
                kind: MutationKind::Append(json!("Hello, world!")),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!(["114", "514"])),
            })
            .unwrap();
        batch
            .load(Mutation {
                path: vec![PathSegment::Negative(3)].into(),
                kind: MutationKind::Truncate(8),
            })
            .unwrap();
        assert_eq!(
            batch.dump().into_inner(),
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![PathSegment::Negative(4)].into(),
                        kind: MutationKind::Batch(vec![
                            Mutation {
                                path: vec![].into(),
                                kind: MutationKind::Truncate(3),
                            },
                            Mutation {
                                path: vec![].into(),
                                kind: MutationKind::Append(json!("Hello")),
                            },
                        ]),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Truncate(3),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!(["114", "514"])),
                    },
                ]),
            }),
        );
    }
}
