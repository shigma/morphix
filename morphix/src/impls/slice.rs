use std::array::from_fn;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::{
    ChunkByMut, ChunksExactMut, ChunksMut, IterMut, RChunksExactMut, RChunksMut, RSplitMut, RSplitNMut, SliceIndex,
    SplitInclusiveMut, SplitMut, SplitNMut,
};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, PathSegment};

enum MutationState {
    Replace,
    Append(usize),
}

pub trait SliceObserverInner<'i> {
    type Item: Observer<'i, InnerDepth = Zero, Head: Sized>;
    fn as_slice(&self) -> &[Self::Item];
    fn as_slice_mut(&mut self) -> &mut [Self::Item];
    fn init_empty() -> Self;
    fn init_range(&self, start: usize, end: usize, slice: &'i mut [<Self::Item as Observer<'i>>::Head]);
}

impl<'i, O, const N: usize> SliceObserverInner<'i> for [O; N]
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    type Item = O;

    fn as_slice(&self) -> &[O] {
        self
    }

    fn as_slice_mut(&mut self) -> &mut [O] {
        self
    }

    fn init_empty() -> Self {
        from_fn(|_| Default::default())
    }

    fn init_range(&self, _start: usize, _end: usize, _slice: &'i mut [<Self::Item as Observer<'i>>::Head]) {
        // No need to re-initialize fixed-size array.
    }
}

impl<'i, O> SliceObserverInner<'i> for UnsafeCell<Vec<O>>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    type Item = O;

    fn as_slice(&self) -> &[Self::Item] {
        unsafe { &*self.get() }
    }

    fn as_slice_mut(&mut self) -> &mut [Self::Item] {
        unsafe { &mut *self.get() }
    }

    fn init_empty() -> Self {
        Default::default()
    }

    fn init_range(&self, start: usize, end: usize, slice: &'i mut [<Self::Item as Observer<'i>>::Head]) {
        let inner = unsafe { &mut *self.get() };
        if end > inner.len() {
            inner.resize_with(end, Default::default);
        }
        let ob_iter = inner[start..end].iter_mut();
        let value_iter = slice[start..end].iter_mut();
        for (ob, value) in ob_iter.zip(value_iter) {
            ObserverPointer::set(O::as_ptr(ob), value);
        }
    }
}

pub(super) trait SliceIndexImpl<T: ?Sized, Output: ?Sized> {
    fn start_bound_inclusive(&self) -> usize;
    fn end_bound_exclusive(&self, len: usize) -> usize;
}

impl<T> SliceIndexImpl<[T], T> for usize {
    #[inline]
    fn start_bound_inclusive(&self) -> usize {
        *self
    }

    #[inline]
    fn end_bound_exclusive(&self, _len: usize) -> usize {
        self + 1
    }
}

impl<T, I: SliceIndex<[T], Output = [T]> + RangeBounds<usize>> SliceIndexImpl<[T], [T]> for I {
    #[inline]
    fn start_bound_inclusive(&self) -> usize {
        match self.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        }
    }

    #[inline]
    fn end_bound_exclusive(&self, len: usize) -> usize {
        match self.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        }
    }
}

/// Observer implementation for [slice](core::slice).
///
/// `SliceObserver` provides element-level change tracking for slices through indexing operations.
/// It serves as the foundation for both [`VecObserver`](crate::impls::vec::VecObserver) and
/// [`ArrayObserver`](crate::impls::array::ArrayObserver), enabling them to track mutations to
/// individual elements.
pub struct SliceObserver<'i, V, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    pub(super) obs: V,
    mutation: Option<MutationState>,
    phantom: PhantomData<&'i mut D>,
}

impl<'i, V, S: ?Sized, D> SliceObserver<'i, V, S, D> {
    pub(super) fn mark_replace(&mut self) {
        self.mutation = Some(MutationState::Replace);
    }

    pub(super) fn mark_append(&mut self, start_index: usize) {
        if self.mutation.is_some() {
            return;
        }
        self.mutation = Some(MutationState::Append(start_index));
    }
}

impl<'i, V, S: ?Sized, D> Default for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i>,
{
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            obs: V::init_empty(),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, V, S: ?Sized, D> Deref for SliceObserver<'i, V, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, V, S: ?Sized, D> DerefMut for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_replace();
        self.obs = V::init_empty();
        &mut self.ptr
    }
}

impl<'i, V, S> Assignable for SliceObserver<'i, V, S>
where
    V: SliceObserverInner<'i>,
{
    type Depth = Succ<Zero>;
}

impl<'i, V, S: ?Sized, D, O, T> Observer<'i> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            obs: V::init_empty(),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, V, S: ?Sized, D, O, T> SerializeObserver<'i> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: SerializeObserver<'i, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        let mut max_index = usize::MAX;
        if let Some(mutation) = this.mutation.take() {
            mutations.push(Mutation {
                path: Default::default(),
                kind: match mutation {
                    MutationState::Replace => {
                        max_index = 0;
                        MutationKind::Replace(A::serialize_value(this.as_deref().as_ref())?)
                    }
                    MutationState::Append(start_index) => {
                        max_index = start_index;
                        MutationKind::Append(A::serialize_value(&this.as_deref().as_ref()[start_index..])?)
                    }
                },
            });
        };
        let len = this.as_deref().as_ref().len();
        for (index, observer) in this.obs.as_slice_mut().iter_mut().take(max_index).enumerate() {
            if let Some(mut mutation) = SerializeObserver::collect::<A>(observer)? {
                mutation.path.push(PathSegment::NegIndex(len - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<'i, V, S: ?Sized, D, O, T> SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: 'i,
{
    fn obs_range(&mut self, start: usize, end: usize) -> Option<&mut [O]> {
        let current_len = Self::as_inner(self).as_mut().len();
        if current_len < end {
            return None;
        }
        self.obs.init_range(start, end, Self::as_inner(self).as_mut());
        Some(self.obs.as_slice_mut())
    }

    fn obs_full(&mut self) -> &mut [O] {
        let len = Self::as_inner(self).as_mut().len();
        self.obs.init_range(0, len, Self::as_inner(self).as_mut());
        self.obs.as_slice_mut()
    }

    pub fn first_mut(&mut self) -> Option<&mut O> {
        self.obs_range(0, 1)?.first_mut()
    }

    pub fn split_first_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.obs_full().split_first_mut()
    }

    pub fn split_last_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.obs_full().split_last_mut()
    }

    pub fn last_mut(&mut self) -> Option<&mut O> {
        self.obs_full().last_mut()
    }

    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        self.obs_range(0, N)?.first_chunk_mut()
    }

    pub fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O; N], &mut [O])> {
        self.obs_full().split_first_chunk_mut()
    }

    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O], &mut [O; N])> {
        self.obs_full().split_last_chunk_mut()
    }

    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        self.obs_full().last_chunk_mut()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut O> {
        self.obs_range(index, index + 1)?.get_mut(index)
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        Self::as_inner(self).as_mut().swap(a, b);
        // manually trigger `DerefMut` down to `T`
        self.obs.init_range(a, a + 1, Self::as_inner(self).as_mut());
        self.obs.init_range(b, b + 1, Self::as_inner(self).as_mut());
        let obs = self.obs.as_slice_mut();
        obs[a].as_deref_mut_coinductive();
        obs[b].as_deref_mut_coinductive();
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, O> {
        self.obs_full().iter_mut()
    }

    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, O> {
        self.obs_full().chunks_mut(chunk_size)
    }

    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, O> {
        self.obs_full().chunks_exact_mut(chunk_size)
    }

    pub fn as_chunks_mut<const N: usize>(&mut self) -> (&mut [[O; N]], &mut [O]) {
        self.obs_full().as_chunks_mut()
    }

    pub fn as_rchunks_mut<const N: usize>(&mut self) -> (&mut [O], &mut [[O; N]]) {
        self.obs_full().as_rchunks_mut()
    }

    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, O> {
        self.obs_full().rchunks_mut(chunk_size)
    }

    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, O> {
        self.obs_full().rchunks_exact_mut(chunk_size)
    }

    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, O, F>
    where
        F: FnMut(&O, &O) -> bool,
    {
        self.obs_full().chunk_by_mut(pred)
    }

    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [O], &mut [O]) {
        self.obs_full().split_at_mut(mid)
    }

    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [O], &mut [O])> {
        self.obs_full().split_at_mut_checked(mid)
    }

    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.obs_full().split_mut(pred)
    }

    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.obs_full().split_inclusive_mut(pred)
    }

    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.obs_full().rsplit_mut(pred)
    }

    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.obs_full().splitn_mut(n, pred)
    }

    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.obs_full().rsplitn_mut(n, pred)
    }
}

impl<'i, V, S: ?Sized, D, O, T> Debug for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.as_deref().as_ref()).finish()
    }
}

impl<'i, V, S: ?Sized, D, O, T, U> PartialEq<U> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().as_ref().eq(other)
    }
}

impl<'i, V, S: ?Sized, D, O, T, U> PartialOrd<U> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().as_ref().partial_cmp(other)
    }
}

impl<'i, V, S: ?Sized, D, O, T, I> Index<I> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        let len = Self::as_inner(self).as_mut().len();
        let start = index.start_bound_inclusive();
        let end = index.end_bound_exclusive(len);
        if end > len {
            panic!("index out of bounds");
        }
        self.obs.init_range(start, end, Self::as_inner(self).as_mut());
        self.obs.as_slice().index(index)
    }
}

impl<'i, V, S: ?Sized, D, O, T, I> IndexMut<I> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let len = Self::as_inner(self).as_mut().len();
        let start = index.start_bound_inclusive();
        let end = index.end_bound_exclusive(len);
        if end > len {
            panic!("index out of bounds");
        }
        self.obs.init_range(start, end, Self::as_inner(self).as_mut());
        self.obs.as_slice_mut().index_mut(index)
    }
}

impl<T: Observe> Observe for [T] {
    type Observer<'i, S, D>
        = SliceObserver<'i, UnsafeCell<Vec<T::Observer<'i, T, Zero>>>, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}
