//! Observer implementation for slices `[T]`.
//!
//! ## Stability
//!
//! The [`SliceObserverState`] trait is an internal abstraction used by [`SliceObserver`] and may
//! change in future versions without notice.

use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::{
    ChunkByMut, ChunksExactMut, ChunksMut, IterMut, RChunksExactMut, RChunksMut, RSplitMut, RSplitNMut, SliceIndex,
    SplitInclusiveMut, SplitMut, SplitNMut,
};

use crate::helper::{
    AsDeref, AsDerefMut, AsDerefMutCoinductive, ObserverState, Pointer, QuasiObserver, Succ, Unsigned, Zero,
};
use crate::impls::vec::VecObserverState;
use crate::observe::{DefaultSpec, Observer, RefObserver, SerializeObserver};
use crate::{Mutations, Observe};

/// Trait for managing the internal observer storage within a slice observer.
///
/// This trait abstracts over the storage and initialization of element observers, allowing
/// [`SliceObserver`] to lazily create observers for individual elements as they are accessed.
pub trait SliceObserverState: ObserverState<Target: AsRef<[<Self::Item as QuasiObserver>::Head]>> + Sized {
    /// The element [`Observer`] type.
    type Item: Observer<InnerDepth = Zero, Head: Sized>;

    /// Creates an uninitialized [`Observer`] collection.
    fn uninit() -> Self;

    /// Creates an [`Observer`] collection for the given slice.
    fn observe(slice: &mut Self::Target) -> Self;

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
    unsafe fn init_range(&self, start: usize, end: usize, slice: &mut Self::Target);
}

/// Shared-reference counterpart to [`SliceObserverState`] for element [`RefObserver`] management.
pub trait SliceRefObserverState: ObserverState<Target: AsRef<[<Self::Item as QuasiObserver>::Head]>> + Sized {
    /// The element [`RefObserver`] type.
    type Item: RefObserver<InnerDepth = Zero, Head: Sized>;

    /// Creates an uninitialized [`RefObserver`] collection.
    fn uninit() -> Self;

    /// Creates an [`RefObserver`] collection for the given slice.
    fn observe(slice: &Self::Target) -> Self;
}

/// Flush logic for slice-backed observer state, parameterized by `S` and `D`.
///
/// This trait is generic over the head type `S` and depth `D`, allowing each implementor to
/// choose its own mutability requirement: [`[O; N]`](prim@array) bounds `S: AsDeref<D>` (shared
/// access), while [`VecObserverState`] bounds `S: AsDerefMut<D>` (mutable access for
/// element relocation).
pub trait SliceSerializeObserverState<S: ?Sized, D>: ObserverState {
    /// Consumes the accumulated mutation state, flushes inner element observers, and returns the
    /// collected [`Mutations`].
    ///
    /// This method must fully reset all internal state so that an immediately subsequent call with
    /// no intervening mutations returns empty.
    fn flush(&mut self, ptr: &mut Pointer<S>) -> Mutations;
}

/// Observer implementation for slices `[T]`.
pub struct SliceObserver<V, S: ?Sized, D = Zero> {
    pub(super) ptr: Pointer<S>,
    pub(super) state: V,
    phantom: PhantomData<D>,
}

impl<V, S: ?Sized, D> Deref for SliceObserver<V, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<V, S: ?Sized, D> DerefMut for SliceObserver<V, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<V, S: ?Sized, D> QuasiObserver for SliceObserver<V, S, D>
where
    V: ObserverState,
    D: Unsigned,
    S: AsDeref<D, Target = V::Target>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        ObserverState::invalidate(&mut this.state, (*this.ptr).as_deref());
    }
}

impl<V, S: ?Sized, D, O, T> Observer for SliceObserver<V, S, D>
where
    V: SliceObserverState<Item = O>,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Target>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            state: V::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &mut Self::Head) -> Self {
        let mut this = Self {
            state: V::observe(head.as_deref_mut()),
            ptr: Pointer::from(head),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, D>(&mut this.ptr, &mut this.state);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &mut Self::Head) {
        Pointer::set(this, head);
    }
}

impl<V, S: ?Sized, D, O, T> RefObserver for SliceObserver<V, S, D>
where
    V: SliceRefObserverState<Item = O>,
    D: Unsigned,
    S: AsDeref<D, Target = V::Target>,
    O: RefObserver<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            state: V::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let mut this = Self {
            state: V::observe(head.as_deref()),
            ptr: Pointer::from(head),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, D>(&mut this.ptr, &mut this.state);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(this, head);
    }
}

impl<V, S: ?Sized, D> SerializeObserver for SliceObserver<V, S, D>
where
    V: SliceSerializeObserverState<S, D>,
    D: Unsigned,
    S: AsDeref<D, Target = V::Target>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        this.state.flush(&mut this.ptr)
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

impl<T, I> SliceIndexImpl<[T], [T]> for I
where
    I: SliceIndex<[T], Output = [T]> + RangeBounds<usize>,
{
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

impl<V, S: ?Sized, D, T> SliceObserver<V, S, D>
where
    V: SliceObserverState,
    V::Item: Observer<InnerDepth = Zero, Head = T>,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Target>,
{
    unsafe fn __init_index<I>(&self, index: &I) -> Option<()>
    where
        I: SliceIndex<[V::Item]> + SliceIndexImpl<[V::Item], I::Output>,
    {
        let len = self.untracked_ref().as_ref().len();
        let start = index.start_inclusive();
        let end = index.end_exclusive(len);
        if end > len {
            return None;
        }
        let slice = unsafe { Pointer::as_mut(&self.ptr).as_deref_mut() };
        unsafe { self.state.init_range(start, end, slice) };
        Some(())
    }

    #[inline]
    fn __get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[V::Item]> + SliceIndexImpl<[V::Item], I::Output>,
    {
        unsafe { self.__init_index(&index)? }
        Some(self.state.as_slice().index(index))
    }

    #[inline]
    fn __get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SliceIndex<[V::Item]> + SliceIndexImpl<[V::Item], I::Output>,
    {
        unsafe { self.__init_index(&index)? }
        Some(self.state.as_mut_slice().index_mut(index))
    }

    #[inline]
    pub(crate) fn __force_ref(&self) -> &[V::Item] {
        let slice = unsafe { Pointer::as_mut(&self.ptr).as_deref_mut() };
        unsafe { self.state.init_range(0, slice.as_ref().len(), slice) };
        self.state.as_slice()
    }

    #[inline]
    pub(crate) fn __force_mut(&mut self) -> &mut [V::Item] {
        let slice = (*self.ptr).as_deref_mut();
        unsafe { self.state.init_range(0, slice.as_ref().len(), slice) };
        self.state.as_mut_slice()
    }
}

#[expect(clippy::type_complexity)]
impl<V, S: ?Sized, D, T> SliceObserver<V, S, D>
where
    V: SliceObserverState,
    V::Item: Observer<InnerDepth = Zero, Head = T>,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Target>,
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
        let len = (*self).untracked_ref().as_ref().len();
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
        let len = (*self).untracked_ref().as_ref().len();
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

impl<V, S: ?Sized, D> Debug for SliceObserver<V, S, D>
where
    V: SliceObserverState,
    D: Unsigned,
    S: AsDeref<D, Target = V::Target>,
    V::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.untracked_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for [_]);* $(;)?) => {
        $(
            impl<$($($gen)*,)? V, S: ?Sized, D> PartialEq<$ty> for SliceObserver<V, S, D>
            where
                D: Unsigned,
                S: AsDeref<D, Target = V::Target>,
                V: SliceObserverState,
                V::Target: PartialEq<$ty>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.untracked_ref().eq(other)
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

impl<V1, V2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<SliceObserver<V2, S2, D2>> for SliceObserver<V1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    V1: SliceObserverState,
    V2: SliceObserverState,
    S1: AsDeref<D1, Target = V1::Target>,
    S2: AsDeref<D2, Target = V2::Target>,
    V1::Target: PartialEq<V2::Target>,
{
    #[inline]
    fn eq(&self, other: &SliceObserver<V2, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<V, S: ?Sized, D> Eq for SliceObserver<V, S, D>
where
    D: Unsigned,
    V: SliceObserverState,
    S: AsDeref<D, Target = V::Target>,
    V::Target: Eq,
{
}

impl<V, S: ?Sized, D, U> PartialOrd<[U]> for SliceObserver<V, S, D>
where
    D: Unsigned,
    V: SliceObserverState,
    S: AsDeref<D, Target = V::Target>,
    V::Target: PartialOrd<[U]>,
{
    #[inline]
    fn partial_cmp(&self, other: &[U]) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other)
    }
}

impl<V1, V2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<SliceObserver<V2, S2, D2>> for SliceObserver<V1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    V1: SliceObserverState,
    V2: SliceObserverState,
    S1: AsDeref<D1, Target = V1::Target>,
    S2: AsDeref<D2, Target = V2::Target>,
    V1::Target: PartialOrd<V2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &SliceObserver<V2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<V, S: ?Sized, D> Ord for SliceObserver<V, S, D>
where
    D: Unsigned,
    V: SliceObserverState,
    S: AsDeref<D, Target = V::Target>,
    V::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
    }
}

impl<V, S: ?Sized, D, O, T, I> Index<I> for SliceObserver<V, S, D>
where
    V: SliceObserverState<Item = O>,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Target>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.__get(index).expect("index out of bounds")
    }
}

impl<V, S: ?Sized, D, O, T, I> IndexMut<I> for SliceObserver<V, S, D>
where
    V: SliceObserverState<Item = O>,
    D: Unsigned,
    S: AsDerefMut<D, Target = V::Target>,
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
        = SliceObserver<VecObserverState<T::Observer<'ob, T, Zero>>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn index_by_usize() {
        let slice: &mut [u32] = &mut [0, 1, 2];
        let mut ob = slice.__observe();
        assert_eq!(ob[2], 2);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
        **ob[2] = 42;
        assert_eq!(ob[2], 42);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(-1, json!(42))));
    }

    #[test]
    fn get_mut() {
        let slice: &mut [u32] = &mut [0, 1, 2];
        let mut ob = slice.__observe();
        assert_eq!(*ob.get_mut(2).unwrap(), 2);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
        ***ob.get_mut(2).unwrap() = 42;
        assert_eq!(*ob.get_mut(2).unwrap(), 42);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(-1, json!(42))));
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
            Some(batch!(_, replace!(-2, json!(0)), replace!(-3, json!(1))))
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
