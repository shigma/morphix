use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;
use std::vec::{Drain, ExtractIf, Splice};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{default_impl_ref_observe, untracked_methods};
use crate::helper::{AsDeref, AsDerefMut, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::slice::{SliceIndexImpl, SliceObserver, TruncateAppend};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

/// Observer implementation for [`Vec<T>`].
pub struct VecObserver<O, S: ?Sized, D = Zero> {
    inner: SliceObserver<UnsafeCell<Vec<O>>, TruncateAppend, S, Succ<D>>,
}

impl<O, S: ?Sized, D> Deref for VecObserver<O, S, D> {
    type Target = SliceObserver<UnsafeCell<Vec<O>>, TruncateAppend, S, Succ<D>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<O, S: ?Sized, D> DerefMut for VecObserver<O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<O, S: ?Sized, D> QuasiObserver for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type OuterDepth = Succ<Succ<Zero>>;
    type InnerDepth = D;
}

impl<O, S: ?Sized, D, T> Observer for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            inner: SliceObserver::uninit(),
        }
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        Self {
            inner: SliceObserver::<UnsafeCell<Vec<O>>, TruncateAppend, S, Succ<D>>::observe(value),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        unsafe { SliceObserver::refresh(&mut this.inner, value) }
    }
}

impl<O, S: ?Sized, D, T> SerializeObserver for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: SerializeObserver<InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        unsafe { SliceObserver::flush_unchecked::<A>(&mut this.inner) }
    }
}

impl<O, S: ?Sized, D, T> VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    untracked_methods! { Vec =>
        pub fn reserve(&mut self, additional: usize);
        pub fn reserve_exact(&mut self, additional: usize);
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn shrink_to_fit(&mut self);
        pub fn shrink_to(&mut self, min_capacity: usize);
    }

    /// See [`Vec::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &[O] {
        self.__force_ref()
    }

    /// See [`Vec::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [O] {
        self.__force_mut()
    }
}

#[cfg(feature = "append")]
impl<O, S: ?Sized, D, T> VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    pub(super) fn __append_index(&mut self) -> usize {
        match &self.inner.mutation {
            Some(m) => m.append_index,
            None => 0,
        }
    }

    untracked_methods! { Vec =>
        pub fn push(&mut self, value: T);
        pub fn append(&mut self, other: &mut Vec<T>);
    }

    /// See [`Vec::insert`].
    #[inline]
    pub fn insert(&mut self, index: usize, element: T) {
        if index >= self.__append_index() {
            self.untracked_mut().insert(index, element)
        } else {
            self.observed_mut().insert(index, element)
        }
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<O, S: ?Sized, D, T> VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn __mark_truncate(&mut self, append_index: usize) {
        let mutation = self.mutation.as_mut().unwrap();
        mutation.truncate_len += mutation.append_index - append_index;
        mutation.append_index = append_index;
    }

    /// See [`Vec::clear`].
    #[inline]
    pub fn clear(&mut self) {
        if self.__append_index() == 0 {
            self.untracked_mut().clear()
        } else {
            self.observed_mut().clear()
        }
    }

    /// See [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        let value = self.untracked_mut().remove(index);
        let append_index = self.__append_index();
        if index >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && index + 1 == append_index {
            self.__mark_truncate(index);
        } else {
            self.__mark_replace();
        }
        value
    }

    /// See [`Vec::swap_remove`].
    pub fn swap_remove(&mut self, index: usize) -> T {
        let value = self.untracked_mut().remove(index);
        let append_index = self.__append_index();
        if index >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && index + 1 == append_index {
            self.__mark_truncate(index);
        } else {
            self.__mark_replace();
        }
        value
    }

    /// See [`Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        let value = self.untracked_mut().pop()?;
        let append_index = self.__append_index();
        let len = (*self).observed_ref().len();
        if len >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && len + 1 == append_index {
            self.__mark_truncate(len);
        } else {
            self.__mark_replace();
        }
        Some(value)
    }

    /// See [`Vec::pop_if`].
    #[inline]
    pub fn pop_if(&mut self, predicate: impl FnOnce(&mut O) -> bool) -> Option<T> {
        let last = self.last_mut()?;
        if predicate(last) { self.pop() } else { None }
    }

    /// See [`Vec::truncate`].
    pub fn truncate(&mut self, len: usize) {
        self.untracked_mut().truncate(len);
        let append_index = self.__append_index();
        if len >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && len > 0 {
            self.__mark_truncate(len);
        } else {
            self.__mark_replace();
        }
    }

    /// See [`Vec::split_off`].
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        let vec = self.untracked_mut().split_off(at);
        let append_index = self.__append_index();
        if at >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && at > 0 {
            self.__mark_truncate(at);
        } else {
            self.__mark_replace();
        }
        vec
    }

    /// See [`Vec::resize_with`].
    #[inline]
    pub fn resize_with<F>(&mut self, new_len: usize, f: F)
    where
        F: FnMut() -> T,
    {
        self.untracked_mut().resize_with(new_len, f);
        let append_index = self.__append_index();
        if new_len >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && new_len > 0 {
            self.__mark_truncate(new_len);
        } else {
            self.__mark_replace();
        }
    }

    /// See [`Vec::drain`].
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
    where
        R: RangeBounds<usize>,
    {
        let append_index = self.__append_index();
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= append_index {
            return self.untracked_mut().drain(range);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return self.observed_mut().drain(range);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).observed_ref().len(),
        };
        if end_index < append_index {
            return self.observed_mut().drain(range);
        }
        self.__mark_truncate(start_index);
        self.observed_mut().drain(range)
    }

    /// See [`Vec::splice`].
    pub fn splice<R, I>(&mut self, range: R, replace_with: I) -> Splice<'_, I::IntoIter>
    where
        R: RangeBounds<usize>,
        I: IntoIterator<Item = T>,
    {
        let append_index = self.__append_index();
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= append_index {
            return self.untracked_mut().splice(range, replace_with);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return self.observed_mut().splice(range, replace_with);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).observed_ref().len(),
        };
        if end_index < append_index {
            return self.observed_mut().splice(range, replace_with);
        }
        self.__mark_truncate(start_index);
        self.untracked_mut().splice(range, replace_with)
    }

    /// See [`Vec::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, filter: F) -> ExtractIf<'_, T, F>
    where
        F: FnMut(&mut T) -> bool,
        R: RangeBounds<usize>,
    {
        let append_index = self.__append_index();
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= append_index {
            return self.untracked_mut().extract_if(range, filter);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return self.observed_mut().extract_if(range, filter);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).observed_ref().len(),
        };
        if end_index < append_index {
            return self.observed_mut().extract_if(range, filter);
        }
        self.__mark_truncate(start_index);
        self.untracked_mut().extract_if(range, filter)
    }
}

#[cfg(feature = "append")]
impl<O, S: ?Sized, D, T> VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
    T: Clone,
{
    untracked_methods! { Vec =>
        pub fn extend_from_slice(&mut self, other: &[T]);
        pub fn extend_from_within<R>(&mut self, src: R)
        where { R: RangeBounds<usize> };
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<O, S: ?Sized, D, T> VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
    T: Clone,
{
    /// See [`Vec::resize`].
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T) {
        self.untracked_mut().resize(new_len, value);
        let append_index = self.__append_index();
        if new_len >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && new_len > 0 {
            self.__mark_truncate(new_len);
        } else {
            self.__mark_replace();
        }
    }
}

#[cfg(feature = "append")]
impl<O, S: ?Sized, D, T, U> Extend<U> for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
    Vec<T>: Extend<U>,
{
    #[inline]
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        self.untracked_mut().extend(other);
    }
}

impl<O, S: ?Sized, D> Debug for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecObserver").field(&self.observed_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for Vec<_>);* $(;)?) => {
        $(
            impl<$($($gen)*,)? O, S: ?Sized, D> PartialEq<$ty> for VecObserver<O, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
                O: Observer<InnerDepth = Zero, Head: Sized>,
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
    impl [U] PartialEq<Vec<U>> for Vec<_>;
    impl [U] PartialEq<[U]> for Vec<_>;
    impl ['a, U] PartialEq<&'a U> for Vec<_>;
    impl ['a, U] PartialEq<&'a mut U> for Vec<_>;
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<VecObserver<O2, S2, D2>> for VecObserver<O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialEq<S2::Target>,
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn eq(&self, other: &VecObserver<O2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<O, S: ?Sized, D> Eq for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
}

impl<O, S: ?Sized, D, U> PartialOrd<Vec<U>> for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialOrd<Vec<U>>,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn partial_cmp(&self, other: &Vec<U>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<VecObserver<O2, S2, D2>> for VecObserver<O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialOrd<S2::Target>,
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn partial_cmp(&self, other: &VecObserver<O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<O, S: ?Sized, D> Ord for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn cmp(&self, other: &VecObserver<O, S, D>) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
    }
}

impl<O, S: ?Sized, D, T, I> Index<I> for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<O, S: ?Sized, D, T, I> IndexMut<I> for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'ob, S, D>
        = VecObserver<T::Observer<'ob, T, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [T] RefObserve for Vec<T>;
}

impl<T: Snapshot> Snapshot for Vec<T> {
    type Snapshot = Vec<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.iter().map(|item| item.to_snapshot()).collect()
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.len() == snapshot.len() && self.iter().zip(snapshot.iter()).all(|(a, b)| a.eq_snapshot(b))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};
    use crate::{Mutation, MutationKind, PathSegment};

    #[test]
    fn no_change_returns_none() {
        let mut vec: Vec<i32> = vec![];
        let mut ob = vec.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!([])));
    }

    #[test]
    fn push_triggers_append() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        ob.push(2);
        ob.push(3);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!([2, 3])));
    }

    #[test]
    fn append_vec() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        let mut extra = vec![4, 5];
        ob.append(&mut extra);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!([4, 5])));
    }

    #[test]
    fn extend_from_slice() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        ob.extend_from_slice(&[6, 7]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!([6, 7])));
    }

    #[test]
    fn index_by_usize_1() {
        let mut vec: Vec<i32> = vec![1, 2];
        let mut ob = vec.__observe();
        assert_eq!(ob[0], 1);
        ob.reserve(4); // force reallocation
        **ob[0] = 99;
        ob.reserve(64); // force reallocation
        assert_eq!(ob[0], 99);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![PathSegment::Negative(2)].into(),
                kind: MutationKind::Replace(json!(99))
            })
        );
    }

    #[test]
    fn index_by_usize_2() {
        let mut vec: Vec<i32> = vec![1, 2];
        let mut ob = vec.__observe();
        **ob[0] = 99;
        ob.reserve(64); // force reallocation
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![PathSegment::Negative(2)].into(),
                kind: MutationKind::Replace(json!(99))
            })
        );
    }

    #[test]
    fn append_and_index() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        **ob[0] = 11;
        ob.push(2);
        **ob[1] = 12;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!([12])),
                    },
                    Mutation {
                        path: vec![PathSegment::Negative(2)].into(),
                        kind: MutationKind::Replace(json!(11)),
                    },
                ])
            })
        );
    }

    #[test]
    fn index_by_range() {
        let mut vec: Vec<i32> = vec![1, 2, 3, 4];
        let mut ob = vec.__observe();
        {
            let slice = &mut ob[1..];
            **slice[0] = 222;
            **slice[1] = 333;
        }
        assert_eq!(ob, vec![1, 222, 333, 4]);
        assert_eq!(&ob[..], &[1, 222, 333, 4]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![PathSegment::Negative(3)].into(),
                        kind: MutationKind::Replace(json!(222)),
                    },
                    Mutation {
                        path: vec![PathSegment::Negative(2)].into(),
                        kind: MutationKind::Replace(json!(333)),
                    }
                ]),
            })
        )
    }
}
