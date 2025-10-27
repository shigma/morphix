use std::array::from_fn;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::swap;
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

pub trait AsSlice: AsRef<[Self::Item]> + AsMut<[Self::Item]> {
    type Item;

    fn resize_default(&mut self, new_len: usize)
    where
        Self: Sized,
        Self::Item: Default;
}

pub trait InitDefault {
    fn init_default() -> Self;
}

impl<T> AsSlice for [T] {
    type Item = T;
}

impl<T, const N: usize> AsSlice for [T; N] {
    type Item = T;

    fn resize_default(&mut self, _new_len: usize)
    where
        Self: Sized,
        Self::Item: Default,
    {
        // No need to resize fixed-size array.
    }
}

impl<T: Default, const N: usize> InitDefault for [T; N] {
    fn init_default() -> Self {
        from_fn(|_| Default::default())
    }
}

impl<T> InitDefault for Vec<T> {
    fn init_default() -> Self {
        Vec::new()
    }
}

impl<T> AsSlice for Vec<T> {
    type Item = T;

    fn resize_default(&mut self, new_len: usize)
    where
        Self: Sized,
        Self::Item: Default,
    {
        self.resize_with(new_len, Default::default);
    }
}

pub(super) trait SliceIndexImpl<T: ?Sized, Output: ?Sized> {
    fn end_bound_exclusive(&self) -> Option<usize>;
}

impl<T> SliceIndexImpl<[T], T> for usize {
    #[inline]
    fn end_bound_exclusive(&self) -> Option<usize> {
        Some(self + 1)
    }
}

impl<T, I: SliceIndex<[T], Output = [T]> + RangeBounds<usize>> SliceIndexImpl<[T], [T]> for I {
    #[inline]
    fn end_bound_exclusive(&self) -> Option<usize> {
        match self.end_bound() {
            Bound::Included(&end) => Some(end + 1),
            Bound::Excluded(&end) => Some(end),
            Bound::Unbounded => None,
        }
    }
}

/// An observer for [`[T]`](core::slice) that tracks both replacements and appends.
///
/// `SliceObserver` provides special handling for vector append operations, distinguishing them from
/// complete replacements for efficiency.
pub struct SliceObserver<'i, V, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    obs: UnsafeCell<V>,
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
    V: InitDefault,
{
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            obs: UnsafeCell::new(V::init_default()),
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
    V: InitDefault,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_replace();
        self.obs = UnsafeCell::new(V::init_default());
        &mut self.ptr
    }
}

impl<'i, V, S> Assignable for SliceObserver<'i, V, S>
where
    V: InitDefault,
{
    type Depth = Succ<Zero>;
}

impl<'i, V, S: ?Sized, D, O, T> Observer<'i> for SliceObserver<'i, V, S, D>
where
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsSlice<Item = T>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            obs: UnsafeCell::new(V::init_default()),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, V, S: ?Sized, D, O, T> SerializeObserver<'i> for SliceObserver<'i, V, S, D>
where
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsSlice<Item = T>,
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
        let obs = unsafe { &mut *this.obs.get() };
        for (index, observer) in obs.as_mut().iter_mut().take(max_index).enumerate() {
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
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsSlice<Item = T>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: 'i,
{
    #[expect(clippy::mut_from_ref)]
    unsafe fn obs_unchecked(&self, len: usize) -> &mut [O] {
        let obs = unsafe { &mut *self.obs.get() };
        let old_len = obs.as_ref().len();
        if len >= old_len {
            obs.resize_default(len);
            let ob_iter = obs.as_mut().iter_mut();
            let value_iter = Self::as_inner(self).as_mut().iter_mut();
            for (ob, value) in ob_iter.zip(value_iter).skip(old_len) {
                *ob = O::observe(value);
            }
        }
        obs.as_mut()
    }

    fn obs_checked(&mut self, len: usize) -> Option<&mut [O]> {
        let current_len = Self::as_inner(self).as_mut().len();
        (current_len >= len).then(|| unsafe { Self::obs_unchecked(self, len) })
    }

    fn obs_full(&mut self) -> &mut [O] {
        let len = Self::as_inner(self).as_mut().len();
        unsafe { Self::obs_unchecked(self, len) }
    }

    pub fn first_mut(&mut self) -> Option<&mut O> {
        self.obs_checked(1)?.first_mut()
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
        self.obs_checked(N)?.first_chunk_mut()
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
        self.obs_checked(index + 1)?.get_mut(index)
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        let obs = self.obs_checked(a.max(b) + 1).expect("index out of bounds");
        unsafe {
            let pa = ObserverPointer::as_mut(O::as_ptr(&obs[a]));
            let pb = ObserverPointer::as_mut(O::as_ptr(&obs[b]));
            swap(pa, pb);
        }
        // manually trigger `DerefMut` down to `T`
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
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsSlice<Item = T>,
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
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsSlice<Item = T>,
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
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsSlice<Item = T>,
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
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsSlice<Item = T>,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        let current_len = Self::as_inner(self).as_mut().len();
        let len = index.end_bound_exclusive().unwrap_or(current_len);
        if len > current_len {
            panic!("index out of bounds");
        }
        unsafe { self.obs_unchecked(len) }.index(index)
    }
}

impl<'i, V, S: ?Sized, D, O, T, I> IndexMut<I> for SliceObserver<'i, V, S, D>
where
    V: AsSlice<Item = O> + InitDefault,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsSlice<Item = T>,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let current_len = Self::as_inner(self).as_mut().len();
        let len = index.end_bound_exclusive().unwrap_or(current_len);
        if len > current_len {
            panic!("index out of bounds");
        }
        unsafe { self.obs_unchecked(len) }.index_mut(index)
    }
}

impl<T: Observe> Observe for [T] {
    type Observer<'i, S, D>
        = SliceObserver<'i, Vec<T::Observer<'i, T, Zero>>, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}
