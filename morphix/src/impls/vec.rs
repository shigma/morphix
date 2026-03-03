//! Observer implementation for [`Vec<T>`].

use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;
use std::vec::{Drain, Splice};

use crate::builtin::Snapshot;
use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
use crate::helper::{AsDeref, AsDerefMut, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::slice::{SliceIndexImpl, SliceObserver, SliceObserverState};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{MutationKind, Mutations, Observe, PathSegment};

/// Observer state for dynamically-sized slices ([`Vec<T>`], `Box<[T]>`), tracking append and
/// truncate boundaries.
///
/// The `append_index` divides the observed slice into two regions: elements before it are
/// "existing" (may have individual observer state), and elements from `append_index` onward are
/// "appended" (new since the last flush).
///
/// ## Replace Semantics
///
/// During [`flush`](SliceObserverState::flush), if all existing elements' inner observers report
/// [`Replace`](MutationKind::Replace) and there was at least some tracked content
/// (`append_index > 0` or `truncate_len > 0`), the granular mutations are collapsed into a single
/// whole-slice [`Replace`](MutationKind::Replace).
pub struct VecObserverState<O> {
    /// Number of elements truncated from the end since the last flush.
    pub truncate_len: usize,
    /// Starting index of appended elements. Elements before this index are "existing" and have
    /// their inner observers flushed individually.
    pub append_index: usize,
    /// Lazily-initialized element observer storage.
    ///
    /// Unlike map observers which use [`Box<O>`] for pointer stability across rehashing /
    /// node-splits, we store observers inline in a [`Vec<O>`]. This is sound because `init_range`
    /// always resizes to `values.len()` (the full observed slice length), not just `end`. Since
    /// `values.len()` can only change through `&mut self` operations (push, pop, etc.), it
    /// stays constant for the entire duration of any `&self` borrow. Therefore, the first
    /// `init_range` call sizes the Vec to its final length, and subsequent calls within the
    /// same `&self` borrow lifetime never trigger reallocation, keeping all previously returned
    /// references valid.
    pub inner: UnsafeCell<Vec<O>>,
}

impl<O> SliceObserverState for VecObserverState<O>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type Slice = [O::Head];
    type Item = O;

    #[inline]
    fn uninit() -> Self {
        Self {
            truncate_len: 0,
            append_index: 0,
            inner: UnsafeCell::new(Vec::new()),
        }
    }

    #[inline]
    fn observe(slice: &Self::Slice) -> Self {
        Self {
            truncate_len: 0,
            append_index: slice.as_ref().len(),
            inner: UnsafeCell::new(Vec::new()),
        }
    }

    #[inline]
    fn mark_replace(&mut self) {
        self.inner.get_mut().clear();
        self.mark_truncate(0);
    }

    #[inline]
    fn as_slice(&self) -> &[Self::Item] {
        unsafe { &*self.inner.get() }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        self.inner.get_mut()
    }

    unsafe fn init_range(&self, start: usize, end: usize, slice: &Self::Slice) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.resize_with(slice.len(), O::uninit);
        let ob_iter = inner[start..end].iter_mut();
        let value_iter = slice[start..end].iter();
        for (ob, value) in ob_iter.zip(value_iter) {
            unsafe { Observer::force(ob, value) }
        }
    }

    fn flush(&mut self, slice: &Self::Slice) -> Mutations
    where
        Self::Item: SerializeObserver,
        <Self::Item as ObserverExt>::Head: serde::Serialize + 'static,
    {
        let append_index = core::mem::replace(&mut self.append_index, slice.len());
        let truncate_len = core::mem::replace(&mut self.truncate_len, 0);
        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(truncate_len));
        }
        #[cfg(feature = "append")]
        if slice.len() > append_index {
            mutations.extend(Mutations::append(&slice[append_index..]));
        }
        unsafe { self.init_range(0, append_index, slice) }
        let (existing, stale) = self.inner.get_mut().split_at_mut(append_index);
        let mut is_replace = true;
        for (index, ob) in existing.iter_mut().enumerate().rev() {
            let inner_mutations = unsafe { SerializeObserver::flush(ob) };
            is_replace &= inner_mutations.is_replace();
            mutations.insert(PathSegment::Negative(slice.len() - index), inner_mutations);
        }
        for observer in stale {
            *observer = O::uninit();
        }
        if is_replace && (append_index > 0 || truncate_len > 0) {
            return Mutations::replace(slice);
        };
        mutations
    }
}

impl<O> VecObserverState<O> {
    /// Marks elements from `index` onward as truncated.
    ///
    /// Accumulates `append_index - index` into `truncate_len` and moves `append_index` down to
    /// `index`. When `index` is 0, this triggers Replace semantics on flush.
    pub fn mark_truncate(&mut self, index: usize) {
        self.truncate_len += self.append_index - index;
        self.append_index = index;
    }
}

/// Observer implementation for [`Vec<T>`].
pub struct VecObserver<O, S: ?Sized, D = Zero> {
    inner: SliceObserver<VecObserverState<O>, S, Succ<D>>,
}

impl<O, S: ?Sized, D> Deref for VecObserver<O, S, D> {
    type Target = SliceObserver<VecObserverState<O>, S, Succ<D>>;
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
    fn observe(head: &Self::Head) -> Self {
        Self {
            inner: SliceObserver::<VecObserverState<O>, S, Succ<D>>::observe(head),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        unsafe { SliceObserver::refresh(&mut this.inner, head) }
    }
}

impl<O, S: ?Sized, D, T> SerializeObserver for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Vec<T>>,
    O: SerializeObserver<InnerDepth = Zero, Head = T>,
    T: serde::Serialize + 'static,
{
    #[inline]
    unsafe fn flush(this: &mut Self) -> Mutations {
        unsafe { SliceObserver::flush(&mut this.inner) }
    }
}

impl<O, S: ?Sized, D, T> VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    delegate_methods! { untracked_mut as Vec =>
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
    delegate_methods! { untracked_mut as Vec =>
        pub fn push(&mut self, value: T);
        pub fn append(&mut self, other: &mut Vec<T>);
    }

    /// See [`Vec::insert`].
    #[inline]
    pub fn insert(&mut self, index: usize, element: T) {
        if index >= self.state.append_index {
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
    /// See [`Vec::clear`].
    #[inline]
    pub fn clear(&mut self) {
        if self.state.append_index == 0 {
            self.untracked_mut().clear()
        } else {
            self.observed_mut().clear()
        }
    }

    /// See [`Vec::remove`].
    pub fn remove(&mut self, index: usize) -> T {
        let value = self.untracked_mut().remove(index);
        if index >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && index + 1 == self.state.append_index {
            self.state.mark_truncate(index);
        } else {
            self.state.mark_replace();
        }
        value
    }

    /// See [`Vec::swap_remove`].
    pub fn swap_remove(&mut self, index: usize) -> T {
        let value = self.untracked_mut().remove(index);
        if index >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && index + 1 == self.state.append_index {
            self.state.mark_truncate(index);
        } else {
            self.state.mark_replace();
        }
        value
    }

    /// See [`Vec::pop`].
    pub fn pop(&mut self) -> Option<T> {
        let value = self.untracked_mut().pop()?;
        let len = (*self).observed_ref().len();
        if len >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && len + 1 == self.state.append_index {
            self.state.mark_truncate(len);
        } else {
            self.state.mark_replace();
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
        if len >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && len > 0 {
            self.state.mark_truncate(len);
        } else {
            self.state.mark_replace();
        }
    }

    /// See [`Vec::split_off`].
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        let vec = self.untracked_mut().split_off(at);
        if at >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && at > 0 {
            self.state.mark_truncate(at);
        } else {
            self.state.mark_replace();
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
        if new_len >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && new_len > 0 {
            self.state.mark_truncate(new_len);
        } else {
            self.state.mark_replace();
        }
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
        if start_index >= self.state.append_index {
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
        if end_index < self.state.append_index {
            return self.observed_mut().drain(range);
        }
        self.state.mark_truncate(start_index);
        self.observed_mut().drain(range)
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
        if start_index >= self.state.append_index {
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
        if end_index < self.state.append_index {
            return self.observed_mut().splice(range, replace_with);
        }
        self.state.mark_truncate(start_index);
        self.untracked_mut().splice(range, replace_with)
    }

    /// See [`Vec::retain`].
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.extract_if(.., |v| !f(v)).for_each(drop);
    }

    /// See [`Vec::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, filter: F) -> ExtractIf<'_, O, F>
    where
        F: FnMut(&mut T) -> bool,
        R: RangeBounds<usize>,
    {
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        let vec = (*self.inner.ptr).as_deref_mut();
        let inner = vec.extract_if(range, filter);
        let state = if start_index < self.inner.state.append_index {
            Some((&mut self.inner.state, start_index))
        } else {
            None
        };
        ExtractIf { inner, state }
    }
}

/// Iterator produced by [`VecObserver::extract_if`].
#[cfg(any(feature = "append", feature = "truncate"))]
pub struct ExtractIf<'a, O, F>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
    F: FnMut(&mut O::Head) -> bool,
{
    inner: std::vec::ExtractIf<'a, O::Head, F>,
    state: Option<(&'a mut VecObserverState<O>, usize)>,
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<O, F> Iterator for ExtractIf<'_, O, F>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
    F: FnMut(&mut O::Head) -> bool,
{
    type Item = O::Head;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.inner.next()?;
        // Update diff on first extraction. We use `start_index` (range start) instead of
        // the actual index of the extracted element because `std::vec::ExtractIf` only
        // yields `T` values without index information.
        if let Some((state, start_index)) = self.state.take() {
            if cfg!(feature = "truncate") && start_index > 0 {
                state.mark_truncate(start_index);
            } else {
                state.mark_replace();
            }
        }
        Some(value)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<O, F> FusedIterator for ExtractIf<'_, O, F>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
    F: FnMut(&mut O::Head) -> bool,
{
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<O, F> Debug for ExtractIf<'_, O, F>
where
    O: Observer<InnerDepth = Zero, Head: Debug + Sized>,
    F: FnMut(&mut O::Head) -> bool,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
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
    delegate_methods! { untracked_mut as Vec =>
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
        if new_len >= self.state.append_index {
            // no-op
        } else if cfg!(feature = "truncate") && new_len > 0 {
            self.state.mark_truncate(new_len);
        } else {
            self.state.mark_replace();
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
        // All existing elements (only ob[0]) report Replace, and there are appended elements.
        // The optimization collapses everything into a single whole-vec Replace.
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Replace(json!([11, 12])),
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
                        path: vec![PathSegment::Negative(2)].into(),
                        kind: MutationKind::Replace(json!(333)),
                    },
                    Mutation {
                        path: vec![PathSegment::Negative(3)].into(),
                        kind: MutationKind::Replace(json!(222)),
                    }
                ]),
            })
        )
    }

    #[test]
    fn pop_push_clears_stale_state() {
        let mut vec = vec!["a".to_string(), "b".to_string(), "ab".to_string()];
        let mut ob = vec.__observe();

        // Modify element 2, then pop and push back in the SAME cycle.
        // The inner observer Vec never sees a shorter length, so resize_with
        // alone cannot clear the stale state — flush must reset it.
        ob[2].truncate(1);
        ob.pop();
        ob.push("cd".to_string());
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some()); // Truncate(1) + Append(["cd"])

        // Next cycle: element 2 should have a fresh observer.
        assert_eq!(ob[2], "cd");
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn extract_if_no_match() {
        let mut vec = vec![1, 2, 3];
        let mut ob = vec.__observe();
        let extracted: Vec<_> = ob.extract_if(.., |x| *x > 10).collect();
        assert!(extracted.is_empty());
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn extract_if_after_append_index() {
        let mut vec = vec![1, 2];
        let mut ob = vec.__observe();
        ob.push(3);
        ob.push(4);
        // Range starts at append_index, so extraction is fully untracked.
        let extracted: Vec<_> = ob.extract_if(2.., |x| *x > 2).collect();
        assert_eq!(extracted, vec![3, 4]);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn extract_if_before_append_index() {
        let mut vec = vec![1, 2, 3, 4];
        let mut ob = vec.__observe();
        // start_index == 0 triggers Replace on first extraction.
        let extracted: Vec<_> = ob.extract_if(.., |x| *x % 2 == 0).collect();
        assert_eq!(extracted, vec![2, 4]);
        assert_eq!(ob, vec![1, 3]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!([1, 3])));
    }

    #[test]
    fn extract_if_straddles_append_index() {
        let mut vec = vec![1, 2, 3];
        let mut ob = vec.__observe();
        ob.push(4);
        ob.push(5);
        // Range 1.. straddles: start_index=1 < append_index=3.
        // First extraction triggers mark_truncate(1).
        let extracted: Vec<_> = ob.extract_if(1.., |x| *x % 2 == 0).collect();
        assert_eq!(extracted, vec![2, 4]);
        assert_eq!(ob, vec![1, 3, 5]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Truncate(2),
                    },
                    Mutation {
                        path: vec![].into(),
                        kind: MutationKind::Append(json!([3, 5])),
                    },
                ]),
            })
        );
    }
}
