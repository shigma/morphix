use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::helper::{Assignable, RangeLike};
use crate::observe::{DefaultSpec, MutationState, StatefulObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, Observer, PathSegment};

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
pub struct VecObserver<'i, O: Observer<'i, Target: Sized>> {
    ptr: *mut Vec<O::Target>,
    mutation: Option<MutationState>,
    obs: UnsafeCell<Vec<O>>,
    phantom: PhantomData<&'i mut O::Target>,
}

impl<'i, O: Observer<'i, Target: Sized>> Default for VecObserver<'i, O> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> Deref for VecObserver<'i, O> {
    type Target = Vec<O::Target>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> DerefMut for VecObserver<'i, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        take(&mut self.obs);
        self.__as_mut()
    }
}

impl<'i, O: Observer<'i, Target: Sized>> Assignable for VecObserver<'i, O> {}

impl<'i, O: Observer<'i, Target: Serialize + Sized>> Observer<'i> for VecObserver<'i, O> {
    type Spec = DefaultSpec;

    fn inner(this: &Self) -> *mut Self::Target {
        this.ptr
    }

    #[inline]
    fn observe(value: &'i mut Vec<O::Target>) -> Self {
        Self {
            ptr: value,
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }

    unsafe fn collect_unchecked<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        let mut max_index = None;
        if let Some(mutation) = Self::mutation_state(&mut this).take() {
            mutations.push(Mutation {
                path: Default::default(),
                kind: match mutation {
                    MutationState::Replace => MutationKind::Replace(A::serialize_value(&*this)?),
                    MutationState::Append(start_index) => {
                        max_index = Some(start_index);
                        MutationKind::Append(A::serialize_value(&(*this)[start_index..])?)
                    }
                },
            });
        };
        let obs = take(unsafe { &mut *this.obs.get() });
        for (index, observer) in obs.into_iter().enumerate() {
            if let Some(max_index) = max_index
                && index >= max_index
            {
                // already included in append
                continue;
            }
            if let Some(mut mutation) = Observer::collect::<A>(observer)? {
                mutation.path.push(PathSegment::NegIndex(this.len() - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<'i, O: Observer<'i, Target: Sized>> StatefulObserver<'i> for VecObserver<'i, O> {
    #[inline]
    fn mutation_state(this: &mut Self) -> &mut Option<MutationState> {
        &mut this.mutation
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'i>
        = VecObserver<'i, T::Observer<'i>>
    where
        Self: 'i;
}

impl<'i, O: Observer<'i, Target: Sized>> VecObserver<'i, O> {
    #[inline]
    fn __as_mut(&mut self) -> &mut Vec<O::Target> {
        unsafe { &mut *self.ptr }
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.__as_mut().reserve(additional);
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.__as_mut().reserve_exact(additional);
    }

    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.__as_mut().try_reserve(additional)
    }

    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.__as_mut().try_reserve_exact(additional)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.__as_mut().shrink_to_fit();
    }

    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.__as_mut().shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: O::Target) {
        Self::mark_append(self, self.len());
        self.__as_mut().push(value);
    }

    pub fn append(&mut self, other: &mut Vec<O::Target>) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__as_mut().append(other);
    }
}

impl<'i, T: Observe + Clone, O: Observer<'i, Target = T>> VecObserver<'i, O> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__as_mut().extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Self::mark_append(self, self.len());
        self.__as_mut().extend_from_within(range);
    }
}

impl<'i, O: Observer<'i, Target: Sized>, U> Extend<U> for VecObserver<'i, O>
where
    Vec<O::Target>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.__as_mut().extend(other);
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default> Index<usize> for VecObserver<'i, O> {
    type Output = O;

    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs: &mut Vec<O> = unsafe { &mut *self.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        if O::inner(&obs[index]) != value {
            obs[index] = O::observe(value);
        }
        &obs[index]
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default> IndexMut<usize> for VecObserver<'i, O> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        if O::inner(&obs[index]) != value {
            obs[index] = O::observe(value);
        }
        &mut obs[index]
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default, I> Index<I> for VecObserver<'i, O>
where
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
            Bound::Unbounded => self.len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = unsafe { &mut (&mut *self.ptr)[start + i] };
            if O::inner(obs_item) != value {
                *obs_item = O::observe(value);
            }
        }
        &obs[index]
    }
}

impl<'i, O: Observer<'i, Target: Debug + Sized>> Debug for VecObserver<'i, O> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecObserver").field(&**self).finish()
    }
}

impl<'i, O, U> PartialEq<Vec<U>> for VecObserver<'i, O>
where
    O: Observer<'i, Target: PartialEq<U> + Sized>,
{
    #[inline]
    fn eq(&self, other: &Vec<U>) -> bool {
        (**self).eq(other)
    }
}

impl<'i, O, P, Q: ?Sized> PartialEq<P> for VecObserver<'i, O>
where
    O: Observer<'i, Target: Sized>,
    P: Observer<'i, Target = Q>,
    Vec<O::Target>: PartialEq<Q>,
{
    #[inline]
    fn eq(&self, other: &P) -> bool {
        (**self).eq(&**other)
    }
}

impl<'i, O: Observer<'i, Target: Eq + Sized>> Eq for VecObserver<'i, O> {}

impl<'i, O: Observer<'i, Target: PartialOrd + Sized>> PartialOrd<Vec<O::Target>> for VecObserver<'i, O> {
    #[inline]
    fn partial_cmp(&self, other: &Vec<O::Target>) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

impl<'i, O, P, Q: ?Sized> PartialOrd<P> for VecObserver<'i, O>
where
    O: Observer<'i, Target: Sized>,
    P: Observer<'i, Target = Q>,
    Vec<O::Target>: PartialOrd<Q>,
{
    #[inline]
    fn partial_cmp(&self, other: &P) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(&**other)
    }
}

impl<'i, O: Observer<'i, Target: Ord + Sized>> Ord for VecObserver<'i, O> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (**self).cmp(&**other)
    }
}

impl<'i, O, I> IndexMut<I> for VecObserver<'i, O>
where
    O: Observer<'i, Target: Sized> + Default,
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
            Bound::Unbounded => self.len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = unsafe { &mut (&mut *self.ptr)[start + i] };
            if O::inner(obs_item) != value {
                *obs_item = O::observe(value);
            }
        }
        &mut obs[index]
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
        type Observer<'i>
            = ShallowObserver<'i, Self>
        where
            Self: 'i;
    }

    #[test]
    fn no_change_returns_none() {
        let mut vec: Vec<Number> = vec![];
        let ob = vec.__observe();
        assert!(Observer::collect::<JsonAdapter>(ob).unwrap().is_none());
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.clear();
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!([])));
    }

    #[test]
    fn push_triggers_append() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.push(Number(2));
        ob.push(Number(3));
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([2, 3])));
    }

    #[test]
    fn append_vec() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        let mut extra = vec![Number(4), Number(5)];
        ob.append(&mut extra);
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([4, 5])));
    }

    #[test]
    fn extend_from_slice() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.extend_from_slice(&[Number(6), Number(7)]);
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
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
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
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
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
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
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
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
