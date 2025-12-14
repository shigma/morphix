use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;
use std::vec::{Drain, ExtractIf, Splice};

use serde::Serialize;

use crate::helper::macros::{default_impl_ref_observe, untracked_methods};
use crate::helper::{AsDerefMut, AsNormalized, Succ, Unsigned, Zero};
use crate::impls::slice::{ObserverSlice, SliceIndexImpl, SliceObserver, TruncateAppend};
use crate::observe::{DefaultSpec, Observer, RefObserve, SerializeObserver};
use crate::{Adapter, Mutation, Observe};

pub struct VecObserver<'ob, O, S: ?Sized, D = Zero> {
    inner: SliceObserver<'ob, UnsafeCell<Vec<O>>, TruncateAppend, S, Succ<D>>,
}

impl<'ob, O, S: ?Sized, D> Deref for VecObserver<'ob, O, S, D> {
    type Target = SliceObserver<'ob, UnsafeCell<Vec<O>>, TruncateAppend, S, Succ<D>>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ob, O, S: ?Sized, D> DerefMut for VecObserver<'ob, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'ob, O, S: ?Sized, D> AsNormalized for VecObserver<'ob, O, S, D> {
    type OuterDepth = Succ<Succ<Zero>>;
}

impl<'ob, O, S: ?Sized, D, T> Observer<'ob> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            inner: SliceObserver::uninit(),
        }
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            inner: SliceObserver::<UnsafeCell<Vec<O>>, TruncateAppend, S, Succ<D>>::observe(value),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        unsafe { SliceObserver::refresh(&mut this.inner, value) }
    }
}

impl<'ob, O, S: ?Sized, D, T> SerializeObserver<'ob> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: SerializeObserver<'ob, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        unsafe { SliceObserver::flush_unchecked::<A>(&mut this.inner) }
    }
}

impl<'ob, O, S: ?Sized, D, T> VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
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
        self.__force();
        self.inner.obs.as_slice()
    }

    /// See [`Vec::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [O] {
        self.__force();
        self.inner.obs.as_mut_slice()
    }
}

#[cfg(feature = "append")]
impl<'ob, O, S: ?Sized, D, T> VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
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
            Observer::as_inner(self).insert(index, element)
        } else {
            Observer::track_inner(self).insert(index, element)
        }
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<'ob, O, S: ?Sized, D, T> VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
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
            Observer::as_inner(self).clear()
        } else {
            Observer::track_inner(self).clear()
        }
    }

    /// See [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        let value = Observer::as_inner(self).remove(index);
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
        let value = Observer::as_inner(self).remove(index);
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
        let value = Observer::as_inner(self).pop()?;
        let append_index = self.__append_index();
        let len = self.as_deref().len();
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
        Observer::as_inner(self).truncate(len);
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
        let vec = Observer::as_inner(self).split_off(at);
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
        Observer::as_inner(self).resize_with(new_len, f);
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
            return Observer::as_inner(self).drain(range);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return Observer::track_inner(self).drain(range);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.as_deref().len(),
        };
        if end_index < append_index {
            return Observer::track_inner(self).drain(range);
        }
        self.__mark_truncate(start_index);
        Observer::track_inner(self).drain(range)
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
            return Observer::as_inner(self).splice(range, replace_with);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return Observer::track_inner(self).splice(range, replace_with);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.as_deref().len(),
        };
        if end_index < append_index {
            return Observer::track_inner(self).splice(range, replace_with);
        }
        self.__mark_truncate(start_index);
        Observer::as_inner(self).splice(range, replace_with)
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
            return Observer::as_inner(self).extract_if(range, filter);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return Observer::track_inner(self).extract_if(range, filter);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.as_deref().len(),
        };
        if end_index < append_index {
            return Observer::track_inner(self).extract_if(range, filter);
        }
        self.__mark_truncate(start_index);
        Observer::as_inner(self).extract_if(range, filter)
    }
}

#[cfg(feature = "append")]
impl<'ob, O, S: ?Sized, D, T> VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: Clone + 'ob,
{
    untracked_methods! { Vec =>
        pub fn extend_from_slice(&mut self, other: &[T]);
        pub fn extend_from_within<R>(&mut self, src: R)
        where { R: RangeBounds<usize> };
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<'ob, O, S: ?Sized, D, T> VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: Clone + 'ob,
{
    /// See [`Vec::resize`].
    #[inline]
    pub fn resize(&mut self, new_len: usize, value: T) {
        Observer::as_inner(self).resize(new_len, value);
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
impl<'ob, O, S: ?Sized, D, T, U> Extend<U> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    T: 'ob,
    Vec<T>: Extend<U>,
{
    #[inline]
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        Observer::as_inner(self).extend(other);
    }
}

impl<'ob, O, S: ?Sized, D, T> Debug for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecObserver").field(self.as_deref()).finish()
    }
}

impl<'ob, O, S: ?Sized, D, T, U> PartialEq<U> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    Vec<T>: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'ob, O, S: ?Sized, D, T, U> PartialOrd<U> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    Vec<T>: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<'ob, O, S: ?Sized, D, T, I> Index<I> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<'ob, O, S: ?Sized, D, T, I> IndexMut<I> for VecObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'ob, S, D>
        = VecObserver<'ob, T::Observer<'ob, T, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [T: RefObserve] RefObserve for Vec<T> ;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::MutationKind;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt, ShallowObserver};

    #[derive(Debug, Serialize, Clone, PartialEq, Eq)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'ob, S, D>
            = ShallowObserver<'ob, S, D>
        where
            Self: 'ob,
            D: Unsigned,
            S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

        type Spec = DefaultSpec;
    }

    #[test]
    fn no_change_returns_none() {
        let mut vec: Vec<Number> = vec![];
        let mut ob = vec.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!([])));
    }

    #[test]
    fn push_triggers_append() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.push(Number(2));
        ob.push(Number(3));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!([2, 3])));
    }

    #[test]
    fn append_vec() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        let mut extra = vec![Number(4), Number(5)];
        ob.append(&mut extra);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!([4, 5])));
    }

    #[test]
    fn extend_from_slice() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob.extend_from_slice(&[Number(6), Number(7)]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!([6, 7])));
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
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![(-2).into()].into(),
                kind: MutationKind::Replace(json!(99))
            })
        );
    }

    #[test]
    fn append_and_index() {
        let mut vec: Vec<Number> = vec![Number(1)];
        let mut ob = vec.__observe();
        ob[0].0 = 11;
        ob.push(Number(2));
        ob[1].0 = 12;
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
                        path: vec![(-2).into()].into(),
                        kind: MutationKind::Replace(json!(11)),
                    },
                ])
            })
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
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![(-3).into()].into(),
                        kind: MutationKind::Replace(json!(222)),
                    },
                    Mutation {
                        path: vec![(-2).into()].into(),
                        kind: MutationKind::Replace(json!(333)),
                    }
                ]),
            })
        )
    }
}
