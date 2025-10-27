use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::{swap, take};
use std::ops::{Deref, DerefMut};
use std::slice::{
    ChunkByMut, ChunksExactMut, ChunksMut, IterMut, RChunksExactMut, RChunksMut, RSplitMut, RSplitNMut,
    SplitInclusiveMut, SplitMut, SplitNMut,
};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::impls::index::SliceObserverImpl;
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, PathSegment};

enum MutationState {
    Replace,
    Append(usize),
}

/// An observer for [`[T]`](core::slice) that tracks both replacements and appends.
///
/// `SliceObserver` provides special handling for vector append operations, distinguishing them from
/// complete replacements for efficiency.
pub struct SliceObserver<'i, O, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    pub(super) obs: UnsafeCell<Vec<O>>,
    mutation: Option<MutationState>,
    phantom: PhantomData<&'i mut D>,
}

impl<'i, O, S: ?Sized, D> SliceObserver<'i, O, S, D> {
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

impl<'i, O, S: ?Sized, D> Default for SliceObserver<'i, O, S, D> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            obs: Default::default(),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, O, S: ?Sized, D> Deref for SliceObserver<'i, O, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, O, S: ?Sized, D> DerefMut for SliceObserver<'i, O, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_replace();
        take(&mut self.obs);
        &mut self.ptr
    }
}

impl<'i, O, S> Assignable for SliceObserver<'i, O, S> {
    type Depth = Succ<Zero>;
}

impl<'i, O, S: ?Sized, D> Observer<'i> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            obs: Default::default(),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, O, S: ?Sized, D> SerializeObserver<'i> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: SerializeObserver<'i, InnerDepth = Zero>,
    O::Head: Serialize + Sized,
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
                        MutationKind::Replace(A::serialize_value(this.as_deref())?)
                    }
                    MutationState::Append(start_index) => {
                        max_index = start_index;
                        MutationKind::Append(A::serialize_value(&this.as_deref()[start_index..])?)
                    }
                },
            });
        };
        let len = this.as_deref().len();
        let obs = unsafe { &mut *this.obs.get() };
        for (index, observer) in obs.iter_mut().take(max_index).enumerate() {
            if let Some(mut mutation) = SerializeObserver::collect::<A>(observer)? {
                mutation.path.push(PathSegment::NegIndex(len - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<'i, O, S: ?Sized, D> SliceObserverImpl<'i, O> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    unsafe fn as_obs_unchecked(&self, len: usize) -> &mut [O] {
        let obs = unsafe { &mut *self.obs.get() };
        if len >= obs.len() {
            obs.resize_with(len, Default::default);
        }
        obs.as_mut()
    }
}

impl<'i, O, S: ?Sized, D> SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Sized,
{
    pub fn first_mut(&mut self) -> Option<&mut O> {
        self.as_obs_checked(0)?.first_mut()
    }

    pub fn split_first_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.as_obs_full().split_first_mut()
    }

    pub fn split_last_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.as_obs_full().split_last_mut()
    }

    pub fn last_mut(&mut self) -> Option<&mut O> {
        self.as_obs_full().last_mut()
    }

    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        self.as_obs_checked(N - 1)?.first_chunk_mut()
    }

    pub fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O; N], &mut [O])> {
        self.as_obs_full().split_first_chunk_mut()
    }

    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O], &mut [O; N])> {
        self.as_obs_full().split_last_chunk_mut()
    }

    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        self.as_obs_full().last_chunk_mut()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut O> {
        self.as_obs_checked(index)?.get_mut(index)
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        let obs = self.as_obs_checked(a.max(b)).expect("index out of bounds");
        unsafe {
            let pa = ObserverPointer::as_mut(O::as_ptr(&obs[a]));
            let pb = ObserverPointer::as_mut(O::as_ptr(&obs[b]));
            swap(pa, pb);
        }
        // manually trigger `DerefMut` down to `O::Head`
        obs[a].as_deref_mut_coinductive();
        obs[b].as_deref_mut_coinductive();
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, O> {
        self.as_obs_full().iter_mut()
    }

    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, O> {
        self.as_obs_full().chunks_mut(chunk_size)
    }

    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, O> {
        self.as_obs_full().chunks_exact_mut(chunk_size)
    }

    pub fn as_chunks_mut<const N: usize>(&mut self) -> (&mut [[O; N]], &mut [O]) {
        self.as_obs_full().as_chunks_mut()
    }

    pub fn as_rchunks_mut<const N: usize>(&mut self) -> (&mut [O], &mut [[O; N]]) {
        self.as_obs_full().as_rchunks_mut()
    }

    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, O> {
        self.as_obs_full().rchunks_mut(chunk_size)
    }

    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, O> {
        self.as_obs_full().rchunks_exact_mut(chunk_size)
    }

    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, O, F>
    where
        F: FnMut(&O, &O) -> bool,
    {
        self.as_obs_full().chunk_by_mut(pred)
    }

    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [O], &mut [O]) {
        self.as_obs_full().split_at_mut(mid)
    }

    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [O], &mut [O])> {
        self.as_obs_full().split_at_mut_checked(mid)
    }

    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.as_obs_full().split_mut(pred)
    }

    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.as_obs_full().split_inclusive_mut(pred)
    }

    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.as_obs_full().rsplit_mut(pred)
    }

    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.as_obs_full().splitn_mut(n, pred)
    }

    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.as_obs_full().rsplitn_mut(n, pred)
    }
}

impl<'i, O, S: ?Sized, D> Debug for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]>,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Debug + Sized,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.as_deref()).finish()
    }
}

impl<'i, O, S: ?Sized, D, U> PartialEq<U> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]>,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Sized,
    [O::Head]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'i, O, S: ?Sized, D, U> PartialOrd<U> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]>,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Sized,
    [O::Head]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<T: Observe> Observe for [T] {
    type Observer<'i, S, D>
        = SliceObserver<'i, T::Observer<'i, T, Zero>, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}
