use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Pointer, RangeLike, Unsigned, Zero};
use crate::observe::DefaultSpec;
use crate::{Adapter, Mutation, MutationKind, Observe, Observer, PathSegment};

enum MutationState {
    Replace,
    Append(usize),
}

/// An observer for [`Vec`] that tracks both replacements and appends.
///
/// `VecObserver` provides special handling for vector append operations, distinguishing them from
/// complete replacements for efficiency.
///
/// ## Supported Operations
///
/// The following mutations are tracked as [`Append`](MutationKind::Append):
///
/// - [Vec::push](std::vec::Vec::push)
/// - [Vec::append](std::vec::Vec::append)
/// - [Vec::extend](std::iter::Extend)
/// - [Vec::extend_from_slice](std::vec::Vec::extend_from_slice)
/// - [Vec::extend_from_within](std::vec::Vec::extend_from_within)
pub struct VecObserver<'i, O, S: ?Sized, N> {
    ptr: Pointer<S>,
    mutation: Option<MutationState>,
    obs: UnsafeCell<Vec<O>>,
    phantom: PhantomData<&'i mut N>,
}

impl<'i, O, S: ?Sized, N> VecObserver<'i, O, S, N> {
    fn mark_replace(&mut self) {
        self.mutation = Some(MutationState::Replace);
    }

    fn mark_append(&mut self, start_index: usize) {
        if self.mutation.is_some() {
            return;
        }
        self.mutation = Some(MutationState::Append(start_index));
    }
}

impl<'i, O, S: ?Sized, N> Default for VecObserver<'i, O, S, N> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: Pointer::default(),
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'i, O, S: ?Sized, N> Deref for VecObserver<'i, O, S, N> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, O, S: ?Sized, N> DerefMut for VecObserver<'i, O, S, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        take(&mut self.obs);
        &mut self.ptr
    }
}

impl<'i, O, S: ?Sized, N> Assignable for VecObserver<'i, O, S, N> {}

impl<'i, O, S: ?Sized, N, T> Observer for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T>,
    T: Serialize,
{
    type UpperDepth = N;
    type LowerDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &mut Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }

    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        let mut max_index = None;
        if let Some(mutation) = this.mutation.take() {
            mutations.push(Mutation {
                path: Default::default(),
                kind: match mutation {
                    MutationState::Replace => MutationKind::Replace(A::serialize_value((***this).as_deref())?),
                    MutationState::Append(start_index) => {
                        max_index = Some(start_index);
                        MutationKind::Append(A::serialize_value(&(***this).as_deref()[start_index..])?)
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
            if let Some(mut mutation) = Observer::collect::<A>(&mut observer)? {
                mutation
                    .path
                    .push(PathSegment::NegIndex((***this).as_deref().len() - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'i, S, N>
        = VecObserver<'i, T::Observer<'i, T, Zero>, S, N>
    where
        Self: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}

impl<'i, O, S: ?Sized, N, T> VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T>,
{
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        (*self.ptr).as_deref_mut().reserve(additional);
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        (*self.ptr).as_deref_mut().reserve_exact(additional);
    }

    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        (*self.ptr).as_deref_mut().try_reserve(additional)
    }

    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        (*self.ptr).as_deref_mut().try_reserve_exact(additional)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        (*self.ptr).as_deref_mut().shrink_to_fit();
    }

    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        (*self.ptr).as_deref_mut().shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: T) {
        Self::mark_append(self, (***self).as_deref().len());
        (*self.ptr).as_deref_mut().push(value);
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, (***self).as_deref().len());
        (*self.ptr).as_deref_mut().append(other);
    }
}

impl<'i, O, S: ?Sized, N, T> VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T>,
    T: Clone,
{
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, (***self).as_deref().len());
        (*self.ptr).as_deref_mut().extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Self::mark_append(self, (***self).as_deref().len());
        (*self.ptr).as_deref_mut().extend_from_within(range);
    }
}

impl<'i, O, S: ?Sized, N, T, U> Extend<U> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T>,
    Vec<T>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        Self::mark_append(self, (***self).as_deref().len());
        (*self.ptr).as_deref_mut().extend(other);
    }
}

impl<'i, O, S: ?Sized, N, T> Index<usize> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
{
    type Output = O;

    fn index(&self, index: usize) -> &Self::Output {
        let value = &mut self.ptr.as_mut().as_deref_mut()[index];
        let obs: &mut Vec<O> = unsafe { &mut *self.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        if *O::as_ptr(&obs[index]) != Pointer::new(value) {
            obs[index] = O::observe(value);
        }
        &obs[index]
    }
}

impl<'i, O, S: ?Sized, N, T> IndexMut<usize> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = &mut (*self.ptr).as_deref_mut()[index];
        let obs: &mut Vec<O> = unsafe { &mut *self.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        if *O::as_ptr(&obs[index]) != Pointer::new(value) {
            obs[index] = O::observe(value);
        }
        &mut obs[index]
    }
}

impl<'i, O, S: ?Sized, N, T, I> Index<I> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
    I: RangeLike<usize> + SliceIndex<[O], Output = [O]>,
{
    type Output = [O];

    fn index(&self, index: I) -> &Self::Output {
        let obs = unsafe { &mut *self.obs.get() };
        let start = match index.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match index.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => (***self).as_deref().len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = &mut self.ptr.as_mut().as_deref_mut()[start + i];
            if *O::as_ptr(obs_item) != Pointer::new(value) {
                *obs_item = O::observe(value);
            }
        }
        &obs[index]
    }
}

impl<'i, O, S: ?Sized, N, T, I> IndexMut<I> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
    I: RangeLike<usize> + SliceIndex<[O], Output = [O]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let obs = unsafe { &mut *self.obs.get() };
        let start = match index.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match index.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => (*self.ptr).as_deref().len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = &mut self.ptr.as_mut().as_deref_mut()[start + i];
            if *O::as_ptr(obs_item) != Pointer::new(value) {
                *obs_item = O::observe(value);
            }
        }
        &mut obs[index]
    }
}

impl<'i, O, S: ?Sized, N, T> Debug for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecObserver").field((***self).as_deref()).finish()
    }
}

impl<'i, O, S: ?Sized, N, T, U> PartialEq<U> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
    Vec<T>: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        (***self).as_deref().eq(other)
    }
}

impl<'i, O, S: ?Sized, N, T, U> PartialOrd<U> for VecObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Vec<T>>,
    O: Observer<UpperDepth = Zero, Head = T> + Default,
    Vec<T>: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (***self).as_deref().partial_cmp(other)
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::helper::ObserveExt;
    use crate::observe::ShallowObserver;
    use crate::{JsonAdapter, Observer};

    #[derive(Debug, Serialize, Clone, PartialEq, Eq)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'i, S, N>
            = ShallowObserver<'i, S, N>
        where
            Self: 'i,
            N: Unsigned,
            S: AsDerefMut<N, Target = Self> + ?Sized + 'i;

        type Spec = DefaultSpec;
    }

    #[test]
    fn no_change_returns_none() {
        let mut vec: Vec<Number> = vec![];
        let mut ob = vec.__observe();
        assert!(Observer::collect::<JsonAdapter>(&mut ob).unwrap().is_none());
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.clear();
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!([])));
    }

    #[test]
    fn push_triggers_append() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.push(Number(2));
        ob.push(Number(3));
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([2, 3])));
    }

    #[test]
    fn append_vec() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        let mut extra = vec![Number(4), Number(5)];
        ob.append(&mut extra);
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([4, 5])));
    }

    #[test]
    fn extend_from_slice() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.extend_from_slice(&[Number(6), Number(7)]);
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([6, 7])));
    }

    #[test]
    fn index_by_usize() {
        let mut vec: Vec<Number> = vec![Number(1), Number(2)];
        let mut ob = vec.__observe();
        assert_eq!(ob[0].0, 1);
        ob.reserve(100); // force reallocation
        ob[0].0 = 99;
        ob.reserve(100); // force reallocation
        assert_eq!(ob[0].0, 99);
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.path, vec![(-2).into()].into());
        assert_eq!(mutation.kind, MutationKind::Replace(json!(99)));
    }

    #[test]
    fn append_and_index() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob[0].0 = 11;
        ob.push(Number(2));
        ob[1].0 = 12;
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.path, vec![].into());
        assert_eq!(
            mutation.kind,
            MutationKind::Batch(vec![
                Mutation {
                    path: vec![].into(),
                    kind: MutationKind::Append(json!([12])),
                },
                Mutation {
                    path: vec![(-2).into()].into(),
                    kind: MutationKind::Replace(json!(11)),
                },
            ])
        );
    }

    #[test]
    fn index_by_range() {
        let mut vec: Vec<Number> = vec![Number(1), Number(2), Number(3), Number(4)];
        let mut ob = vec.__observe();
        {
            let slice = &mut ob[1..];
            slice[0].0 = 222;
            slice[1].0 = 333;
        }
        assert_eq!(ob, vec![Number(1), Number(222), Number(333), Number(4)]);
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.path, vec![].into());
        assert_eq!(
            mutation.kind,
            MutationKind::Batch(vec![
                Mutation {
                    path: vec![(-3).into()].into(),
                    kind: MutationKind::Replace(json!(222)),
                },
                Mutation {
                    path: vec![(-2).into()].into(),
                    kind: MutationKind::Replace(json!(333)),
                }
            ]),
        )
    }
}
