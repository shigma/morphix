use std::fmt::Debug;

use crate::{Path, PathSegment};

/// A mutation representing a change to a value at a specific path.
///
/// [`Mutation`] captures both the location where a change occurred (via `path`) and the kind of
/// change that was made (via `kind`). Mutations can be applied to values to reproduce the changes
/// they represent.
///
/// ## Path Representation
///
/// The path is stored in *reverse order* for efficiency during collection.
/// For example, a change at `foo.bar.baz` would have `path = ["baz", "bar", "foo"]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mutation<V> {
    /// The path to the mutated value, stored in *reverse order*.
    ///
    /// An empty vec indicates a mutation at the root level.
    pub path: Path<true>,

    /// The kind of mutation that occurred.
    pub kind: MutationKind<V>,
}

impl<V> Mutation<V> {
    fn make_batch(&mut self, capacity: usize) -> &mut Vec<Self> {
        if self.path.is_empty()
            && let MutationKind::Batch(ref mut batch) = self.kind
        {
            return batch;
        }
        let old = std::mem::replace(
            self,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(Vec::with_capacity(capacity)),
            },
        );
        let MutationKind::Batch(batch) = &mut self.kind else {
            unreachable!()
        };
        batch.push(old);
        batch
    }
}

/// A collection of mutations collected during observation. It is the return type for
/// [`flush`](crate::observe::SerializeObserver::flush) operations.
///
/// ## Behavior
///
/// - If no mutations are pushed, [`into_inner`](Mutations::into_inner) returns [`None`].
/// - If exactly one mutation is pushed, it is returned as-is.
/// - If multiple mutations are pushed, they are wrapped in a [`Batch`](MutationKind::Batch).
///
/// ## Example
///
/// ```
/// use morphix::{Mutation, MutationKind, Mutations};
///
/// let mut mutations = Mutations::new();
///
/// mutations.insert("a", MutationKind::Replace(42).into());
/// mutations.insert("b", MutationKind::Truncate(1).into());
///
/// let result = mutations.into_inner();
/// assert!(matches!(result, Some(Mutation { kind: MutationKind::Batch(_), .. })));
/// ```
pub struct Mutations<V> {
    inner: Option<Mutation<V>>,
    capacity: usize,
}

impl<V> Default for Mutations<V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<V> From<MutationKind<V>> for Mutations<V> {
    #[inline]
    fn from(kind: MutationKind<V>) -> Self {
        Self {
            inner: Some(Mutation {
                path: Default::default(),
                kind,
            }),
            capacity: 2,
        }
    }
}

impl<V> From<Mutations<V>> for Option<Mutation<V>> {
    #[inline]
    fn from(value: Mutations<V>) -> Self {
        value.into_inner()
    }
}

impl<V> Mutations<V> {
    /// Creates a new empty batch.
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: None,
            capacity: 2,
        }
    }

    /// Creates a new empty batch with a specified capacity hint.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self { inner: None, capacity }
    }

    /// Consumes the batch and returns the collected mutation.
    #[inline]
    pub fn into_inner(self) -> Option<Mutation<V>> {
        self.inner
    }

    pub fn extend(&mut self, mutations: impl Into<Self>) {
        let Some(incoming) = mutations.into().into_inner() else {
            return;
        };
        let Some(existing) = &mut self.inner else {
            self.inner = Some(incoming);
            return;
        };
        let existing_batch: &mut Vec<Mutation<V>> = existing.make_batch(self.capacity);
        if incoming.path.is_empty()
            && let MutationKind::Batch(incoming_batch) = incoming.kind
        {
            existing_batch.extend(incoming_batch);
        } else {
            existing_batch.push(incoming);
        }
    }

    #[inline]
    pub fn insert(&mut self, segment: impl Into<PathSegment>, mutations: Self) {
        self.__insert(Some(segment.into()), mutations);
    }

    #[inline]
    pub fn insert2(&mut self, segment1: impl Into<PathSegment>, segment2: impl Into<PathSegment>, mutations: Self) {
        self.__insert(
            Some(segment1.into()).into_iter().chain(Some(segment2.into())),
            mutations,
        );
    }

    fn __insert(&mut self, segments: impl IntoIterator<Item = PathSegment>, mutations: Self) {
        let Some(mut incoming) = mutations.into_inner() else {
            return;
        };
        incoming.path.extend(segments);
        let Some(existing) = &mut self.inner else {
            self.inner = Some(incoming);
            return;
        };
        let existing_batch = existing.make_batch(self.capacity);
        existing_batch.push(incoming);
    }

    pub fn len(&self) -> usize {
        match &self.inner {
            None => 0,
            Some(mutation) => match &mutation.kind {
                MutationKind::Batch(batch) if mutation.path.is_empty() => batch.len(),
                _ => 1,
            },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The kind of mutation that occurred.
///
/// [`MutationKind`] represents the specific type of change made to a value. Different kinds enable
/// optimizations and more precise change descriptions.
///
/// ## Variants
///
/// - [`Replace`](MutationKind::Replace): Complete replacement of a value
/// - [`Append`](MutationKind::Append): Append operation for strings and vectors
/// - [`Batch`](MutationKind::Batch): Multiple mutations combined
///
/// ## Example
///
/// ```
/// use morphix::adapter::Json;
/// use morphix::{Mutation, MutationKind, Observe, observe};
/// use serde::Serialize;
/// use serde_json::json;
///
/// #[derive(Serialize, Observe)]
/// struct Document {
///     title: String,
///     content: String,
///     tags: Vec<String>,
/// }
///
/// let mut doc = Document {
///     title: "Draft".to_string(),
///     content: "Hello".to_string(),
///     tags: vec!["todo".to_string()],
/// };
///
/// let Json(mutation) = observe!(doc => {
///     doc.title = "Final".to_string();      // Replace
///     doc.content.push_str(" World");       // Append
///     doc.tags.push("done".to_string());    // Append
/// }).unwrap();
///
/// // The mutation contains a Batch with three kinds
/// assert!(matches!(mutation.unwrap().kind, MutationKind::Batch(_)));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationKind<T> {
    /// [`Replace`](MutationKind::Replace) is the default mutation for
    /// [`DerefMut`](std::ops::DerefMut) operations.
    ///
    /// ## Examples
    ///
    /// ```
    /// # #[derive(Default)]
    /// # struct Foo {
    /// #   a: A,
    /// #   num: i32,
    /// #   vec: Vec<i32>,
    /// # }
    /// # #[derive(Default)]
    /// # struct A {
    /// #   b: i32,
    /// # }
    /// # let mut foo = Foo::default();
    /// foo.a.b = 1;        // Replace at .a.b
    /// foo.num *= 2;       // Replace at .num
    /// foo.vec.clear();    // Replace at .vec
    /// ```
    Replace(T),

    /// [`Append`](MutationKind::Append) represents adding data to the end of a string or vector.
    /// This is more efficient than [`Replace`](MutationKind::Replace) because only the appended
    /// portion needs to be serialized and transmitted.
    ///
    /// ## Examples
    ///
    /// ```
    /// # #[derive(Default)]
    /// # struct Foo {
    /// #   a: A,
    /// #   vec: Vec<i32>,
    /// # }
    /// # #[derive(Default)]
    /// # struct A {
    /// #   b: String,
    /// # }
    /// # let mut foo = Foo::default();
    /// # let iter = vec![2, 3].into_iter();
    /// foo.a.b += "text";          // Append to .a.b
    /// foo.a.b.push_str("text");   // Append to .a.b
    /// foo.vec.push(1);            // Append to .vec
    /// foo.vec.extend(iter);       // Append to .vec
    /// ```
    #[cfg(feature = "append")]
    Append(T),

    /// [`Truncate`](MutationKind::Truncate) represents removing elements from the end of a string
    /// or vector. This is more efficient than [`Replace`](MutationKind::Replace) because only
    /// the truncation length needs to be serialized and transmitted.
    ///
    /// ## Examples
    ///
    /// ```
    /// # #[derive(Default)]
    /// # struct Foo {
    /// #   a: A,
    /// #   vec: Vec<i32>,
    /// # }
    /// # #[derive(Default)]
    /// # struct A {
    /// #   b: String,
    /// # }
    /// let mut foo = Foo {
    ///     a: A { b: "Hello, World!".to_string() },
    ///     vec: vec![1, 2, 3, 4, 5],
    /// };
    /// foo.a.b.truncate(5);        // Truncate 8 chars from .a.b
    /// foo.vec.pop();              // Truncate 1 element from .vec
    #[cfg(feature = "truncate")]
    Truncate(usize),

    /// [`Batch`](MutationKind::Batch) combines multiple mutations that occurred during a single
    /// observation period. This is automatically created when multiple independent changes are
    /// detected.
    ///
    /// ## Optimization
    ///
    /// The batch collector ([`BatchTree`](crate::BatchTree)) automatically optimizes mutations:
    /// - Consecutive appends are merged
    /// - Redundant changes are eliminated
    /// - Nested paths are consolidated when possible
    Batch(Vec<Mutation<T>>),
}
