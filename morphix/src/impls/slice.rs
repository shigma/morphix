use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, PathSegment};

enum MutationState {
    Replace,
    Append(usize),
}

/// An observer for [`[T]`] that tracks both replacements and appends.
///
/// `SliceObserver` provides special handling for vector append operations, distinguishing them from
/// complete replacements for efficiency.
pub struct SliceObserver<'i, O, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    obs: UnsafeCell<Vec<O>>,
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

impl<'i, O, S: ?Sized, D, T> Observer<'i> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
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

impl<'i, O, S: ?Sized, D, T> SerializeObserver<'i> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]> + 'i,
    O: SerializeObserver<'i, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        let mut max_index = None;
        if let Some(mutation) = this.mutation.take() {
            mutations.push(Mutation {
                path: Default::default(),
                kind: match mutation {
                    MutationState::Replace => MutationKind::Replace(A::serialize_value(this.as_deref())?),
                    MutationState::Append(start_index) => {
                        max_index = Some(start_index);
                        MutationKind::Append(A::serialize_value(&this.as_deref()[start_index..])?)
                    }
                },
            });
        };
        let obs = take(unsafe { &mut *this.obs.get() });
        for (index, mut observer) in obs.into_iter().enumerate() {
            if let Some(max_index) = max_index
                && index >= max_index
            {
                // already included in append
                continue;
            }
            if let Some(mut mutation) = SerializeObserver::collect::<A>(&mut observer)? {
                mutation.path.push(PathSegment::NegIndex(this.as_deref().len() - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<'i, O, S: ?Sized, D, T> Debug for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.as_deref()).finish()
    }
}

impl<'i, O, S: ?Sized, D, T, U> PartialEq<U> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'i, O, S: ?Sized, D, T, U> PartialOrd<U> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

pub(super) trait SliceIndexImpl<'i, T, O, Output: ?Sized> {
    #[track_caller]
    #[expect(clippy::mut_from_ref)]
    fn index_impl<'j, S, D>(this: &'j SliceObserver<'i, O, S, D>, index: Self) -> &'j mut Output
    where
        D: Unsigned,
        S: AsDerefMut<D, Target = [T]> + ?Sized + 'i;
}

impl<'i, O, S: ?Sized, D, T, I, Output: ?Sized> Index<I> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    I: SliceIndexImpl<'i, T, O, Output> + SliceIndex<[O], Output = Output>,
{
    type Output = Output;

    fn index(&self, index: I) -> &Self::Output {
        SliceIndexImpl::index_impl(self, index)
    }
}

impl<'i, O, S: ?Sized, D, T, I, Output: ?Sized> IndexMut<I> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    I: SliceIndexImpl<'i, T, O, Output> + SliceIndex<[O], Output = Output>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        SliceIndexImpl::index_impl(self, index)
    }
}

impl<'i, O, T> SliceIndexImpl<'i, T, O, O> for usize
where
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: 'i,
{
    fn index_impl<'j, S, D>(this: &'j SliceObserver<'i, O, S, D>, index: Self) -> &'j mut O
    where
        D: Unsigned,
        S: AsDerefMut<D, Target = [T]> + ?Sized + 'i,
    {
        let value = &mut Observer::as_inner(this)[index];
        let obs = unsafe { &mut *this.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        if *O::as_ptr(&obs[index]) != ObserverPointer::new(value) {
            obs[index] = O::observe(value);
        }
        &mut obs[index]
    }
}

impl<'i, O, T, I> SliceIndexImpl<'i, T, O, [O]> for I
where
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: 'i,
    I: RangeBounds<usize> + SliceIndex<[O], Output = [O]>,
{
    fn index_impl<'j, S, D>(this: &'j SliceObserver<'i, O, S, D>, index: Self) -> &'j mut [O]
    where
        D: Unsigned,
        S: AsDerefMut<D, Target = [T]> + ?Sized + 'i,
    {
        let obs = unsafe { &mut *this.obs.get() };
        let start = match index.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match index.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => this.as_deref().len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = &mut Observer::as_inner(this)[start + i];
            if *O::as_ptr(obs_item) != ObserverPointer::new(value) {
                *obs_item = O::observe(value);
            }
        }
        &mut obs[index]
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
