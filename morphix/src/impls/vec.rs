use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;
use std::vec::{Drain, ExtractIf, Splice};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::impls::slice::{ObserverSlice, SliceIndexImpl, SliceObserver};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, Mutation, Observe};

/// Observer implementation for [`Vec`].
///
/// `VecObserver` provides comprehensive change tracking for vectors, including both element-level
/// mutations and append operations. It builds on [`SliceObserver`] for element tracking while
/// adding specialized support for dynamic operations.
pub struct VecObserver<'i, O, S: ?Sized, D = Zero> {
    inner: SliceObserver<'i, UnsafeCell<Vec<O>>, S, Succ<D>>,
}

impl<'i, O, S: ?Sized, D> Default for VecObserver<'i, O, S, D>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<'i, O, S: ?Sized, D> Deref for VecObserver<'i, O, S, D> {
    type Target = SliceObserver<'i, UnsafeCell<Vec<O>>, S, Succ<D>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'i, O, S: ?Sized, D> DerefMut for VecObserver<'i, O, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'i, O, S> Assignable for VecObserver<'i, O, S>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    type Depth = Succ<Succ<Zero>>;
}

impl<'i, O, S: ?Sized, D, T> Observer<'i> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Succ<Zero>;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            inner: SliceObserver::<UnsafeCell<Vec<O>>, S, Succ<D>>::observe(value),
        }
    }
}

impl<'i, O, S: ?Sized, D, T> SerializeObserver<'i> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: SerializeObserver<'i, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        unsafe { SliceObserver::collect_unchecked(&mut this.inner) }
    }
}

impl<'i, O, S: ?Sized, D, T> VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
{
    /// See [`Vec::reserve`].
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        Observer::as_inner(self).reserve(additional);
    }

    /// See [`Vec::reserve_exact`].
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        Observer::as_inner(self).reserve_exact(additional);
    }

    /// See [`Vec::try_reserve`].
    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        Observer::as_inner(self).try_reserve(additional)
    }

    /// See [`Vec::try_reserve_exact`].
    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        Observer::as_inner(self).try_reserve_exact(additional)
    }

    /// See [`Vec::shrink_to_fit`].
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        Observer::as_inner(self).shrink_to_fit();
    }

    /// See [`Vec::shrink_to`].
    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        Observer::as_inner(self).shrink_to(min_capacity);
    }

    /// See [`Vec::truncate`].
    pub fn truncate(&mut self, len: usize) {
        if len >= self.__initial_len() {
            Observer::as_inner(self).truncate(len)
        } else {
            Observer::track_inner(self).truncate(len)
        }
    }

    /// See [`Vec::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &[O] {
        self.inner.obs.as_slice()
    }

    /// See [`Vec::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [O] {
        self.inner.obs.as_mut_slice()
    }

    /// See [`Vec::swap_remove`].
    pub fn swap_remove(&mut self, index: usize) -> T {
        let initial_len = self.__initial_len();
        if self.as_deref().len() <= initial_len {
            Observer::track_inner(self).swap_remove(index)
        } else {
            if index < initial_len {
                self[index].as_deref_mut_coinductive();
            }
            Observer::as_inner(self).swap_remove(index)
        }
    }

    /// See [`Vec::insert`].
    pub fn insert(&mut self, index: usize, element: T) {
        if index >= self.__initial_len() {
            Observer::as_inner(self).insert(index, element)
        } else {
            Observer::track_inner(self).insert(index, element)
        }
    }

    /// See [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.__initial_len() {
            Observer::as_inner(self).remove(index)
        } else {
            Observer::track_inner(self).remove(index)
        }
    }

    /// See [`Vec::push`].
    #[inline]
    pub fn push(&mut self, value: T) {
        Observer::as_inner(self).push(value);
    }

    /// See [`Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        if self.as_deref().len() > self.__initial_len() {
            Observer::as_inner(self).pop()
        } else {
            Observer::track_inner(self).pop()
        }
    }

    /// See [`Vec::pop_if`].
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut O) -> bool) -> Option<T> {
        let last = self.last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    /// See [`Vec::append`].
    #[inline]
    pub fn append(&mut self, other: &mut Vec<T>) {
        Observer::as_inner(self).append(other);
    }

    /// See [`Vec::drain`].
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= self.__initial_len() {
            Observer::as_inner(self).drain(range)
        } else {
            Observer::track_inner(self).drain(range)
        }
    }

    /// See [`Vec::clear`].
    pub fn clear(&mut self) {
        if self.__initial_len() == 0 {
            Observer::as_inner(self).clear()
        } else {
            Observer::track_inner(self).clear()
        }
    }

    /// See [`Vec::split_off`].
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        if at >= self.__initial_len() {
            Observer::as_inner(self).split_off(at)
        } else {
            Observer::track_inner(self).split_off(at)
        }
    }

    /// See [`Vec::resize_with`].
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        if new_len >= self.__initial_len() {
            Observer::as_inner(self).resize_with(new_len, f)
        } else {
            Observer::track_inner(self).resize_with(new_len, f)
        }
    }

    /// See [`Vec::splice`].
    pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, I::IntoIter>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>,
    {
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= self.__initial_len() {
            Observer::as_inner(self).splice(range, replace_with)
        } else {
            Observer::track_inner(self).splice(range, replace_with)
        }
    }

    /// See [`Vec::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, filter: F) -> ExtractIf<'_, T, F>
    where
        F: FnMut(&mut T) -> bool,
        R: RangeBounds<usize>,
    {
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= self.__initial_len() {
            Observer::as_inner(self).extract_if(range, filter)
        } else {
            Observer::track_inner(self).extract_if(range, filter)
        }
    }
}

impl<'i, O, S: ?Sized, D, T> VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: Clone + 'i,
{
    /// See [`Vec::resize`].
    pub fn resize(&mut self, new_len: usize, value: T) {
        if new_len >= self.__initial_len() {
            Observer::as_inner(self).resize(new_len, value)
        } else {
            Observer::track_inner(self).resize(new_len, value)
        }
    }

    /// See [`Vec::extend_from_slice`].
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[T]) {
        Observer::as_inner(self).extend_from_slice(other);
    }

    /// See [`Vec::extend_from_within`].
    #[inline]
    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Observer::as_inner(self).extend_from_within(range);
    }
}

impl<'i, O, S: ?Sized, D, T, U> Extend<U> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: 'i,
    Vec<T>: Extend<U>,
{
    #[inline]
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        Observer::as_inner(self).extend(other);
    }
}

impl<'i, O, S: ?Sized, D, T> Debug for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecObserver").field(self.as_deref()).finish()
    }
}

impl<'i, O, S: ?Sized, D, T, U> PartialEq<U> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    Vec<T>: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'i, O, S: ?Sized, D, T, U> PartialOrd<U> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    Vec<T>: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<'i, O, S: ?Sized, D, T, I> Index<I> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<'i, O, S: ?Sized, D, T, I> IndexMut<I> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'i, S, D>
        = VecObserver<'i, T::Observer<'i, T, Zero>, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::observe::{ObserveExt, SerializeObserverExt, ShallowObserver};
    use crate::{JsonAdapter, MutationKind};

    #[derive(Debug, Serialize, Clone, PartialEq, Eq)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'i, S, D>
            = ShallowObserver<'i, S, D>
        where
            Self: 'i,
            D: Unsigned,
            S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

        type Spec = DefaultSpec;
    }

    #[test]
    fn no_change_returns_none() {
        let mut vec: Vec<Number> = vec![];
        let mut ob = vec.__observe();
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.clear();
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!([])));
    }

    #[test]
    fn push_triggers_append() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.push(Number(2));
        ob.push(Number(3));
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([2, 3])));
    }

    #[test]
    fn append_vec() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        let mut extra = vec![Number(4), Number(5)];
        ob.append(&mut extra);
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([4, 5])));
    }

    #[test]
    fn extend_from_slice() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.extend_from_slice(&[Number(6), Number(7)]);
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!([6, 7])));
    }

    #[test]
    fn index_by_usize() {
        let mut vec: Vec<Number> = vec![Number(1), Number(2)];
        let mut ob = vec.__observe();
        assert_eq!(ob[0].0, 1);
        ob.reserve(4); // force reallocation
        ob[0].0 = 99;
        ob.reserve(64); // force reallocation
        assert_eq!(ob[0].0, 99);
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
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
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
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
        assert_eq!(ob[..], vec![Number(1), Number(222), Number(333), Number(4)]);
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
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
