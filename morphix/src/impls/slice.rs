//! Observer implementation for slices `[T]`.
//!
//! ## Stability
//!
//! The [`SliceObserverInner`] and [`SliceObserverDiff`] traits are internal abstractions used by
//! [`SliceObserver`] and may change in future versions without notice.

use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::{
    ChunkByMut, ChunksExactMut, ChunksMut, IterMut, RChunksExactMut, RChunksMut, RSplitMut, RSplitNMut, SliceIndex,
    SplitInclusiveMut, SplitMut, SplitNMut,
};

use serde::Serialize;

use crate::helper::{AsDeref, AsDerefMut, AsDerefMutCoinductive, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe, PathSegment};

/// Trait for managing the internal observer storage within a slice observer.
///
/// This trait abstracts over the storage and initialization of element observers, allowing
/// [`SliceObserver`] to lazily create observers for individual elements as they are accessed.
pub trait SliceObserverInner: Sized {
    /// The observed slice type that this storage corresponds to.
    type Slice: AsRef<[<Self::Item as ObserverExt>::Head]> + ?Sized;

    /// The element observer type.
    type Item: Observer<InnerDepth = Zero, Head: Sized>;

    /// Creates an uninitialized observer collection.
    fn uninit() -> Self;

    /// Creates an observer collection for the given slice.
    fn observe(slice: &Self::Slice) -> Self;

    /// Called by [`SliceObserver`]'s [`DerefMut`] implementation to notify element observers that
    /// the underlying data may have been overwritten.
    ///
    /// For `UnsafeCell<Vec<O>>`, this clears the Vec. For `[O; N]`, this triggers
    /// [`observed_mut()`](QuasiObserver::observed_mut) on each element observer.
    fn mark_replace(&mut self);

    /// Returns a shared slice of element observers.
    fn as_slice(&self) -> &[Self::Item];

    /// Returns a mutable slice of element observers.
    fn as_mut_slice(&mut self) -> &mut [Self::Item];

    /// Initializes element observers for the specified range.
    ///
    /// This method ensures that observers exist and are properly bound for elements in the range
    /// `[start, end)`.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that no references obtained from [`as_slice`](Self::as_slice) are
    /// alive when this method is called, as the implementation may create mutable references to
    /// the same storage through interior mutability.
    unsafe fn init_range(&self, start: usize, end: usize, slice: &Self::Slice);
}

// SAFETY: Unlike map observers which use `Box<O>` for pointer stability across rehashing /
// node-splits, we store observers inline in a `Vec<O>`. This is sound because `init_range` always
// resizes to `values.len()` (the full observed slice length), not just `end`. Since `values.len()`
// can only change through `&mut self` operations (push, pop, etc.), it stays constant for the
// entire duration of any `&self` borrow. Therefore, the first `init_range` call sizes the Vec to
// its final length, and subsequent calls within the same `&self` borrow lifetime never trigger
// reallocation, keeping all previously returned references valid.
impl<O> SliceObserverInner for UnsafeCell<Vec<O>>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type Slice = [O::Head];
    type Item = O;

    #[inline]
    fn uninit() -> Self {
        Default::default()
    }

    #[inline]
    fn observe(_slice: &Self::Slice) -> Self {
        Default::default()
    }

    #[inline]
    fn mark_replace(&mut self) {
        self.get_mut().clear();
    }

    #[inline]
    fn as_slice(&self) -> &[Self::Item] {
        unsafe { &*self.get() }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        self.get_mut()
    }

    unsafe fn init_range(&self, start: usize, end: usize, slice: &Self::Slice) {
        let inner = unsafe { &mut *self.get() };
        inner.resize_with(slice.len(), O::uninit);
        let ob_iter = inner[start..end].iter_mut();
        let value_iter = slice[start..end].iter();
        for (ob, value) in ob_iter.zip(value_iter) {
            unsafe { Observer::force(ob, value) }
        }
    }
}

pub(crate) trait SliceIndexImpl<T: ?Sized, Output: ?Sized> {
    fn start_inclusive(&self) -> usize;
    fn end_exclusive(&self, len: usize) -> usize;
}

impl<T> SliceIndexImpl<[T], T> for usize {
    #[inline]
    fn start_inclusive(&self) -> usize {
        *self
    }

    #[inline]
    fn end_exclusive(&self, _len: usize) -> usize {
        self + 1
    }
}

impl<T, I: SliceIndex<[T], Output = [T]> + RangeBounds<usize>> SliceIndexImpl<[T], [T]> for I {
    #[inline]
    fn start_inclusive(&self) -> usize {
        match self.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        }
    }

    #[inline]
    fn end_exclusive(&self, len: usize) -> usize {
        match self.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        }
    }
}

/// State container for tracking truncate and append boundaries.
///
/// The `append_index` divides the observed slice into two regions: elements before it are
/// "existing" (may have individual observer state), and elements from `append_index` onward are
/// "appended" (new since the last flush).
///
/// ## Replace Semantics
///
/// When `append_index == 0` and `truncate_len > 0`, the entire original content has been truncated
/// away. In this case, [`flush`](SliceObserverDiff::flush) emits a full
/// [`Replace`](MutationKind::Replace) instead of granular mutations, since there are no surviving
/// elements whose inner observers could be flushed.
pub struct TruncateAppend {
    /// Number of elements truncated from the end.
    pub truncate_len: usize,
    /// Starting index of appended elements.
    pub append_index: usize,
}

/// Trait for tracking append and truncate mutations on slices.
///
/// This trait abstracts over the mutation state management, allowing different strategies for
/// tracking length changes. The unit type `()` implements this trait for observers that don't track
/// append / truncate operations.
pub trait SliceObserverDiff: Sized {
    /// Creates an uninitialized diff state.
    fn uninit() -> Self;

    /// Creates the initial mutation state for a slice of the given length.
    fn observe(len: usize) -> Self;

    /// Called by [`SliceObserver`]'s [`DerefMut`] implementation to update the diff state when the
    /// underlying data may have been overwritten.
    ///
    /// For [`TruncateAppend`], this delegates to
    /// [`mark_truncate(0)`](TruncateAppend::mark_truncate), which triggers Replace semantics on
    /// flush. For `()`, this is a no-op.
    fn mark_replace(&mut self);

    /// Consumes the diff state, serializes any tracked mutations, and returns the append index.
    ///
    /// Returns `(mutations, Some(append_index))` for granular tracking, where `append_index` is
    /// the boundary below which inner observers should be flushed. Returns `(mutations, None)` for
    /// a full [`Replace`](MutationKind::Replace), indicating the caller should skip inner observer
    /// flushing.
    #[expect(clippy::type_complexity)]
    fn flush<A: Adapter, T: Serialize>(self, slice: &[T]) -> Result<(Mutations<A::Value>, Option<usize>), A::Error>;
}

impl SliceObserverDiff for () {
    #[inline]
    fn uninit() -> Self {}

    #[inline]
    fn observe(_len: usize) -> Self {}

    #[inline]
    fn mark_replace(&mut self) {}

    #[inline]
    fn flush<A: Adapter, T: Serialize>(self, slice: &[T]) -> Result<(Mutations<A::Value>, Option<usize>), A::Error> {
        Ok((Mutations::<A::Value>::new(), Some(slice.len())))
    }
}

impl SliceObserverDiff for TruncateAppend {
    #[inline]
    fn uninit() -> Self {
        Self {
            truncate_len: 0,
            append_index: 0,
        }
    }

    #[inline]
    fn observe(len: usize) -> Self {
        TruncateAppend {
            truncate_len: 0,
            append_index: len,
        }
    }

    #[inline]
    fn mark_replace(&mut self) {
        self.mark_truncate(0);
    }

    fn flush<A: Adapter, T: Serialize>(self, slice: &[T]) -> Result<(Mutations<A::Value>, Option<usize>), A::Error> {
        let TruncateAppend {
            truncate_len,
            append_index,
        } = self;
        if append_index == 0 && truncate_len > 0 {
            return Ok((MutationKind::Replace(A::serialize_value(slice)?).into(), None));
        };
        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(truncate_len));
        }
        #[cfg(feature = "append")]
        if slice.len() > append_index {
            mutations.extend(MutationKind::Append(A::serialize_value(&slice[append_index..])?));
        }
        Ok((mutations, Some(append_index)))
    }
}

impl TruncateAppend {
    /// Marks elements from `index` onward as truncated.
    ///
    /// Accumulates `append_index - index` into `truncate_len` and moves `append_index` down to
    /// `index`. When `index` is 0, this triggers Replace semantics on flush (see
    /// [`TruncateAppend`] docs).
    pub fn mark_truncate(&mut self, index: usize) {
        self.truncate_len += self.append_index - index;
        self.append_index = index;
    }
}

/// Observer implementation for slices `[T]`.
pub struct SliceObserver<V, M, S: ?Sized, D = Zero> {
    pub(super) ptr: Pointer<S>,
    pub(super) inner: V,
    pub(super) diff: M,
    phantom: PhantomData<D>,
}

impl<V, M, S: ?Sized, D> Deref for SliceObserver<V, M, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<V, M, S: ?Sized, D> DerefMut for SliceObserver<V, M, S, D>
where
    V: SliceObserverInner,
    M: SliceObserverDiff,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.diff.mark_replace();
        self.inner.mark_replace();
        &mut self.ptr
    }
}

impl<V, M, S: ?Sized, D> QuasiObserver for SliceObserver<V, M, S, D>
where
    V: SliceObserverInner,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<V, M, S: ?Sized, D, O, T> Observer for SliceObserver<V, M, S, D>
where
    V: SliceObserverInner<Item = O>,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDeref<D, Target = V::Slice>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            inner: V::uninit(),
            diff: M::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let slice = head.as_deref();
        Self {
            ptr: Pointer::new(head),
            inner: V::observe(slice),
            diff: M::observe(slice.as_ref().len()),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(this, head);
    }
}

impl<V, M, S: ?Sized, D, O, T> SerializeObserver for SliceObserver<V, M, S, D>
where
    V: SliceObserverInner<Item = O>,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDeref<D, Target = V::Slice>,
    O: SerializeObserver<InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let slice = (*this.ptr).as_deref().as_ref();
        let diff = std::mem::replace(&mut this.diff, M::observe(slice.len()));
        let (mut mutations, append_index) = diff.flush::<A, T>(slice)?;
        let Some(append_index) = append_index else {
            return Ok(mutations);
        };
        this.__init_index(&..append_index);
        // Optimization: if all existing elements would produce Replace, collapse into a single
        // whole-slice Replace. A single Replace is always more compact than a batch of N
        // per-element Replace mutations (each carrying a path segment), and supersedes any
        // Truncate/Append mutations from the diff.
        if !slice.is_empty()
            && this.inner.as_slice()[..append_index]
                .iter()
                .all(|ob| unsafe { O::will_replace(ob) })
        {
            this.inner = V::observe((*this.ptr).as_deref());
            return Ok(MutationKind::Replace(A::serialize_value(slice)?).into());
        }
        let (existing, stale) = this.inner.as_mut_slice().split_at_mut(append_index);
        for (index, ob) in existing.iter_mut().enumerate() {
            mutations.insert(
                PathSegment::Negative(slice.len() - index),
                SerializeObserver::flush::<A>(ob)?,
            );
        }
        for observer in stale {
            *observer = O::uninit();
        }
        Ok(mutations)
    }
}

impl<V, M, S: ?Sized, D, T> SliceObserver<V, M, S, D>
where
    V: SliceObserverInner,
    V::Item: Observer<InnerDepth = Zero, Head = T>,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDeref<D, Target = V::Slice>,
{
    fn __init_index<I>(&self, index: &I) -> Option<()>
    where
        I: SliceIndex<[V::Item]> + SliceIndexImpl<[V::Item], I::Output>,
    {
        let len = self.observed_ref().as_ref().len();
        let start = index.start_inclusive();
        let end = index.end_exclusive(len);
        if end > len {
            return None;
        }
        let slice = self.observed_ref();
        unsafe { self.inner.init_range(start, end, slice) };
        Some(())
    }

    #[inline]
    fn __get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[V::Item]> + SliceIndexImpl<[V::Item], I::Output>,
    {
        self.__init_index(&index)?;
        Some(self.inner.as_slice().index(index))
    }

    #[inline]
    fn __get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SliceIndex<[V::Item]> + SliceIndexImpl<[V::Item], I::Output>,
    {
        self.__init_index(&index)?;
        Some(self.inner.as_mut_slice().index_mut(index))
    }

    #[inline]
    pub(crate) fn __force_ref(&self) -> &[V::Item] {
        let slice = self.observed_ref();
        unsafe { self.inner.init_range(0, slice.as_ref().len(), slice) };
        self.inner.as_slice()
    }

    #[inline]
    pub(crate) fn __force_mut(&mut self) -> &mut [V::Item] {
        let slice = (*self).observed_ref();
        unsafe { self.inner.init_range(0, slice.as_ref().len(), slice) };
        self.inner.as_mut_slice()
    }
}

#[expect(clippy::type_complexity)]
impl<V, M, S: ?Sized, D, T> SliceObserver<V, M, S, D>
where
    V: SliceObserverInner,
    V::Item: Observer<InnerDepth = Zero, Head = T>,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Slice>,
    S::Target: AsMut<[T]>,
{
    /// See [`slice::first_mut`].
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut V::Item> {
        self.__get_mut(0)
    }

    /// See [`slice::split_first_mut`].
    #[inline]
    pub fn split_first_mut(&mut self) -> Option<(&mut V::Item, &mut [V::Item])> {
        self.__force_mut().split_first_mut()
    }

    /// See [`slice::split_last_mut`].
    #[inline]
    pub fn split_last_mut(&mut self) -> Option<(&mut V::Item, &mut [V::Item])> {
        self.__force_mut().split_last_mut()
    }

    /// See [`slice::last_mut`].
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut V::Item> {
        self.__get_mut(..)?.last_mut()
    }

    /// See [`slice::first_chunk_mut`].
    #[inline]
    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [V::Item; N]> {
        let len = (*self).observed_ref().as_ref().len();
        if len < N {
            return None;
        }
        self.__get_mut(..N)?.first_chunk_mut()
    }

    /// See [`slice::split_first_chunk_mut`].
    #[inline]
    pub fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [V::Item; N], &mut [V::Item])> {
        self.__force_mut().split_first_chunk_mut()
    }

    /// See [`slice::split_last_chunk_mut`].
    #[inline]
    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [V::Item], &mut [V::Item; N])> {
        self.__force_mut().split_last_chunk_mut()
    }

    /// See [`slice::last_chunk_mut`].
    #[inline]
    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [V::Item; N]> {
        let len = (*self).observed_ref().as_ref().len();
        if len < N {
            return None;
        }
        self.__get_mut(len - N..)?.last_chunk_mut()
    }

    /// See [`slice::get_mut`].
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut V::Item> {
        self.__get_mut(index)
    }

    /// See [`slice::swap`].
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        self[a].as_deref_mut_coinductive();
        self[b].as_deref_mut_coinductive();
        self.untracked_mut().as_mut().swap(a, b);
    }

    /// See [`slice::iter_mut`].
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, V::Item> {
        self.__force_mut().iter_mut()
    }

    /// See [`slice::chunks_mut`].
    #[inline]
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, V::Item> {
        self.__force_mut().chunks_mut(chunk_size)
    }

    /// See [`slice::chunks_exact_mut`].
    #[inline]
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, V::Item> {
        self.__force_mut().chunks_exact_mut(chunk_size)
    }

    /// See [`slice::as_chunks_mut`].
    #[inline]
    pub fn as_chunks_mut<const N: usize>(&mut self) -> (&mut [[V::Item; N]], &mut [V::Item]) {
        self.__force_mut().as_chunks_mut()
    }

    /// See [`slice::as_rchunks_mut`].
    #[inline]
    pub fn as_rchunks_mut<const N: usize>(&mut self) -> (&mut [V::Item], &mut [[V::Item; N]]) {
        self.__force_mut().as_rchunks_mut()
    }

    /// See [`slice::rchunks_mut`].
    #[inline]
    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, V::Item> {
        self.__force_mut().rchunks_mut(chunk_size)
    }

    /// See [`slice::rchunks_exact_mut`].
    #[inline]
    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, V::Item> {
        self.__force_mut().rchunks_exact_mut(chunk_size)
    }

    /// See [`slice::chunk_by_mut`].
    #[inline]
    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, V::Item, F>
    where
        F: FnMut(&V::Item, &V::Item) -> bool,
    {
        self.__force_mut().chunk_by_mut(pred)
    }

    /// See [`slice::split_at_mut`].
    #[inline]
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [V::Item], &mut [V::Item]) {
        self.__force_mut().split_at_mut(mid)
    }

    /// See [`slice::split_at_mut_checked`].
    #[inline]
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [V::Item], &mut [V::Item])> {
        self.__force_mut().split_at_mut_checked(mid)
    }

    /// See [`slice::split_mut`].
    #[inline]
    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, V::Item, F>
    where
        F: FnMut(&V::Item) -> bool,
    {
        self.__force_mut().split_mut(pred)
    }

    /// See [`slice::split_inclusive_mut`].
    #[inline]
    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, V::Item, F>
    where
        F: FnMut(&V::Item) -> bool,
    {
        self.__force_mut().split_inclusive_mut(pred)
    }

    /// See [`slice::rsplit_mut`].
    #[inline]
    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, V::Item, F>
    where
        F: FnMut(&V::Item) -> bool,
    {
        self.__force_mut().rsplit_mut(pred)
    }

    /// See [`slice::splitn_mut`].
    #[inline]
    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, V::Item, F>
    where
        F: FnMut(&V::Item) -> bool,
    {
        self.__force_mut().splitn_mut(n, pred)
    }

    /// See [`slice::rsplitn_mut`].
    #[inline]
    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, V::Item, F>
    where
        F: FnMut(&V::Item) -> bool,
    {
        self.__force_mut().rsplitn_mut(n, pred)
    }
}

impl<V, M, S: ?Sized, D> Debug for SliceObserver<V, M, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.observed_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for [_]);* $(;)?) => {
        $(
            impl<$($($gen)*,)? V, M, S: ?Sized, D> PartialEq<$ty> for SliceObserver<V, M, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
                V: SliceObserverInner,
                M: SliceObserverDiff,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.observed_ref().eq(other)
                }
            }
        )*
    };
}

generic_impl_partial_eq! {
    impl [U] PartialEq<[U]> for [_];
    impl [U] PartialEq<Vec<U>> for [_];
    impl [U, const N: usize] PartialEq<[U; N]> for [_];
}

impl<V1, V2, M1, M2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<SliceObserver<V2, M2, S2, D2>>
    for SliceObserver<V1, M1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialEq<S2::Target>,
    V1: SliceObserverInner,
    V2: SliceObserverInner,
    M1: SliceObserverDiff,
    M2: SliceObserverDiff,
{
    #[inline]
    fn eq(&self, other: &SliceObserver<V2, M2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<V, M, S: ?Sized, D> Eq for SliceObserver<V, M, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
    V: SliceObserverInner,
    M: SliceObserverDiff,
{
}

impl<V, M, S: ?Sized, D, U> PartialOrd<[U]> for SliceObserver<V, M, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialOrd<[U]>,
    V: SliceObserverInner,
    M: SliceObserverDiff,
{
    #[inline]
    fn partial_cmp(&self, other: &[U]) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other)
    }
}

impl<V1, V2, M1, M2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<SliceObserver<V2, M2, S2, D2>>
    for SliceObserver<V1, M1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialOrd<S2::Target>,
    V1: SliceObserverInner,
    V2: SliceObserverInner,
    M1: SliceObserverDiff,
    M2: SliceObserverDiff,
{
    #[inline]
    fn partial_cmp(&self, other: &SliceObserver<V2, M2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<V, M, S: ?Sized, D> Ord for SliceObserver<V, M, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
    V: SliceObserverInner,
    M: SliceObserverDiff,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
    }
}

impl<V, M, S: ?Sized, D, O, T, I> Index<I> for SliceObserver<V, M, S, D>
where
    V: SliceObserverInner<Item = O>,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDeref<D, Target = V::Slice>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.__get(index).expect("index out of bounds")
    }
}

impl<V, M, S: ?Sized, D, O, T, I> IndexMut<I> for SliceObserver<V, M, S, D>
where
    V: SliceObserverInner<Item = O>,
    M: SliceObserverDiff,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Slice>,
    S::Target: AsMut<[T]>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.__get_mut(index).expect("index out of bounds")
    }
}

impl<T: Observe> Observe for [T] {
    type Observer<'ob, S, D>
        = SliceObserver<UnsafeCell<Vec<T::Observer<'ob, T, Zero>>>, TruncateAppend, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Mutation;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn index_by_usize() {
        let slice: &mut [u32] = &mut [0, 1, 2];
        let mut ob = slice.__observe();
        assert_eq!(ob[2], 2);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
        **ob[2] = 42;
        assert_eq!(ob[2], 42);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
                kind: MutationKind::Replace(json!(42))
            })
        );
    }

    #[test]
    fn get_mut() {
        let slice: &mut [u32] = &mut [0, 1, 2];
        let mut ob = slice.__observe();
        assert_eq!(*ob.get_mut(2).unwrap(), 2);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
        ***ob.get_mut(2).unwrap() = 42;
        assert_eq!(*ob.get_mut(2).unwrap(), 42);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![PathSegment::Negative(1)].into(),
                kind: MutationKind::Replace(json!(42))
            })
        );
    }

    #[test]
    fn swap() {
        let slice: &mut [u32] = &mut [0, 1, 2];
        let mut ob = slice.__observe();
        ob.swap(0, 1);
        assert_eq!(**ob, [1, 0, 2]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![PathSegment::Negative(3)].into(),
                        kind: MutationKind::Replace(json!(1)),
                    },
                    Mutation {
                        path: vec![PathSegment::Negative(2)].into(),
                        kind: MutationKind::Replace(json!(0)),
                    }
                ]),
            })
        );
    }

    #[test]
    fn boxed_slice_deref_mut_triggers_replace() {
        let mut boxed: Box<[u32]> = vec![1, 2, 3].into_boxed_slice();
        let mut ob = boxed.__observe();
        // Mutate through the slice observer's DerefMut (e.g. via sort).
        ob.sort();
        let Json(mutation) = ob.flush().unwrap();
        // Even though sort is a no-op here (already sorted), DerefMut was triggered
        // so a Replace should be emitted. With diff type `()`, this bug causes None.
        assert!(mutation.is_some(), "DerefMut on Box<[T]> should trigger Replace");
    }
}
