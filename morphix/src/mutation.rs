use std::fmt::Debug;

use crate::Path;

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
    /// Coalesce many mutations as a single mutation.
    ///
    /// - Returns [`None`] if no mutations exist.
    /// - Returns a single mutation if only one mutation exists.
    /// - Returns a [`Batch`](MutationKind::Batch) mutation if multiple mutations exist.
    #[deprecated(note = "use `MutationBatch` instead for incremental collection")]
    pub fn coalesce(mut mutations: Vec<Mutation<V>>) -> Option<Mutation<V>> {
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

/// Helper for incrementally collecting multiple mutations into a single mutation.
///
/// ## Behavior
///
/// - If no mutations are pushed, [`into_inner`](MutationBatch::into_inner) returns [`None`].
/// - If exactly one mutation is pushed, it is returned as-is.
/// - If multiple mutations are pushed, they are wrapped in a [`Batch`](MutationKind::Batch).
///
/// ## Example
///
/// ```
/// use morphix::{MutationBatch, Mutation, MutationKind};
///
/// let mut mutations = MutationBatch::new();
///
/// mutations.push(Mutation {
///     path: vec!["a".into()].into(),
///     kind: MutationKind::Replace(42),
/// });
///
/// mutations.push(Mutation {
///     path: vec!["b".into()].into(),
///     kind: MutationKind::Replace(43),
/// });
///
/// let result = mutations.into_inner();
/// assert!(matches!(result, Some(Mutation { kind: MutationKind::Batch(_), .. })));
/// ```
pub struct MutationBatch<V> {
    inner: Option<Mutation<V>>,
    capacity: usize,
}

impl<V> Default for MutationBatch<V> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<V> MutationBatch<V> {
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

    /// Pushes a mutation into the batch.
    ///
    /// If this is the first mutation, it is stored directly. Subsequent mutations cause the
    /// internal storage to be converted to a [`Batch`](MutationKind::Batch).
    pub fn push(&mut self, mutation: Mutation<V>) {
        let Some(existing) = &mut self.inner else {
            self.inner = Some(mutation);
            return;
        };
        let existing_batch = match &mut existing.kind {
            MutationKind::Batch(batch) if existing.path.is_empty() => batch,
            _ => {
                let old = std::mem::replace(
                    existing,
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Batch(Vec::with_capacity(self.capacity)),
                    },
                );
                let MutationKind::Batch(batch) = &mut existing.kind else {
                    unreachable!()
                };
                batch.push(old);
                batch
            }
        };
        if mutation.path.is_empty()
            && let MutationKind::Batch(batch) = mutation.kind
        {
            existing_batch.extend(batch);
        } else {
            existing_batch.push(mutation);
        }
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
