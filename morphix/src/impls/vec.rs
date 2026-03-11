//! Observer implementation for [`Vec<T>`].

use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;
use std::vec::{Drain, Splice};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
use crate::helper::{AsDeref, AsDerefMut, ObserverState, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::slice::{SliceIndexImpl, SliceObserver, SliceObserverState, SliceSerializeObserverState};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{MutationKind, Mutations, Observe, PathSegment};

/// Observer state for dynamically-sized slices ([`Vec<T>`], [`Box<[T]>`](Box)), tracking
/// [`Append`](MutationKind::Append) and [`Truncate`](MutationKind::Truncate) boundaries.
///
/// The `append_index` divides the observed slice into two regions: elements before it are
/// "existing" (may have individual observer state), and elements from `append_index` onward are
/// "appended" (new since the last flush).
///
/// ## Replace Semantics
///
/// During [`flush`](SliceSerializeObserverState::flush), if all existing elements' inner observers
/// report [`Replace`](MutationKind::Replace) and there was at least some tracked content
/// (`append_index > 0` or `truncate_len > 0`), the granular mutations are collapsed into a single
/// whole-slice [`Replace`](MutationKind::Replace).
pub struct VecObserverState<O> {
    /// Number of elements truncated from the end since the last flush.
    truncate_len: usize,
    /// Starting index of appended elements. Elements before this index are "existing" and have
    /// their inner observers flushed individually.
    append_index: usize,
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
    inner: UnsafeCell<Vec<O>>,
}

impl<O> VecObserverState<O> {
    #[inline]
    fn mark_truncate(&mut self, new_len: usize) {
        self.truncate_len += self.append_index - new_len;
        self.append_index = new_len;
    }

    #[inline]
    fn mark_replace(&mut self) {
        self.inner.get_mut().clear();
        self.mark_truncate(0);
    }
}

impl<O> ObserverState for VecObserverState<O>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type Target = [O::Head];

    #[inline]
    fn invalidate(this: &mut Self, _: &[O::Head]) {
        this.mark_replace();
    }
}

impl<O> SliceObserverState for VecObserverState<O>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
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
    fn observe(slice: &mut Self::Target) -> Self {
        Self {
            truncate_len: 0,
            append_index: slice.as_ref().len(),
            inner: UnsafeCell::new(Vec::new()),
        }
    }

    #[inline]
    fn as_slice(&self) -> &[Self::Item] {
        unsafe { &*self.inner.get() }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        self.inner.get_mut()
    }

    unsafe fn init_range(&self, start: usize, end: usize, slice: &mut Self::Target) {
        let inner = unsafe { &mut *self.inner.get() };
        inner.resize_with(slice.len(), O::uninit);
        let ob_iter = inner[start..end].iter_mut();
        let value_iter = slice[start..end].iter_mut();
        for (ob, value) in ob_iter.zip(value_iter) {
            unsafe { Observer::force(ob, value) }
        }
    }
}

impl<O, S, D> SliceSerializeObserverState<S, D> for VecObserverState<O>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + ?Sized,
    O: Observer<InnerDepth = Zero> + SerializeObserver,
    O::Head: Serialize + Sized + 'static,
{
    fn flush(&mut self, ptr: &mut Pointer<S>) -> Mutations {
        let slice = (**ptr).as_deref_mut();
        let append_index = core::mem::replace(&mut self.append_index, slice.len());
        let truncate_len = core::mem::replace(&mut self.truncate_len, 0);
        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(truncate_len));
        }
        // init_range must precede Mutations::append: init_range takes `&mut slice` (Unique function-entry
        // retag over the full slice), which would invalidate a SerializeRef's SRO tag if the append
        // mutation were created first.
        unsafe { self.init_range(0, append_index, slice) }
        #[cfg(feature = "append")]
        if slice.len() > append_index {
            mutations.extend(Mutations::append(&slice[append_index..]));
        }
        let (existing, stale) = self.inner.get_mut().split_at_mut(append_index);
        let mut is_replace = true;
        for (index, ob) in existing.iter_mut().enumerate().rev() {
            let mutations_i = unsafe { SerializeObserver::flush(ob) };
            is_replace &= mutations_i.is_replace();
            mutations.insert(PathSegment::Negative(slice.len() - index), mutations_i);
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
    S: AsDeref<D, Target = Vec<O::Head>>,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type Head = S;
    type OuterDepth = Succ<Succ<Zero>>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        // SliceObserver::invalidate(&mut this.inner);
        ObserverState::invalidate(&mut this.inner.state, (*this.inner.ptr).as_deref());
    }
}

impl<O, S: ?Sized, D, T> Observer for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            inner: Observer::uninit(),
        }
    }

    #[inline]
    fn observe(head: &mut Self::Head) -> Self {
        Self {
            inner: Observer::observe(head),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &mut Self::Head) {
        unsafe { Observer::refresh(&mut this.inner, head) }
    }
}

impl<O, S: ?Sized, D, T> SerializeObserver for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
    O: Observer<InnerDepth = Zero, Head = T> + SerializeObserver,
    T: Serialize + 'static,
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
            self.tracked_mut().insert(index, element)
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
            self.tracked_mut().clear()
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
        let len = (*self).untracked_ref().len();
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
            return self.tracked_mut().drain(range);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).untracked_ref().len(),
        };
        if end_index < self.state.append_index {
            return self.tracked_mut().drain(range);
        }
        self.state.mark_truncate(start_index);
        self.tracked_mut().drain(range)
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
            return self.tracked_mut().splice(range, replace_with);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).untracked_ref().len(),
        };
        if end_index < self.state.append_index {
            return self.tracked_mut().splice(range, replace_with);
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
    S: AsDeref<D, Target = Vec<O::Head>>,
    O: Observer<InnerDepth = Zero, Head: Sized + Debug>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecObserver").field(&self.untracked_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for Vec<_>);* $(;)?) => {
        $(
            impl<$($($gen)*,)? O, S: ?Sized, D> PartialEq<$ty> for VecObserver<O, S, D>
            where
                D: Unsigned,
                S: AsDeref<D, Target = Vec<O::Head>>,
                O: Observer<InnerDepth = Zero, Head: Sized>,
                Vec<O::Head>: PartialEq<$ty>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.untracked_ref().eq(other)
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
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
    S1: AsDeref<D1, Target = Vec<O1::Head>>,
    S2: AsDeref<D2, Target = Vec<O2::Head>>,
    Vec<O1::Head>: PartialEq<Vec<O2::Head>>,
{
    #[inline]
    fn eq(&self, other: &VecObserver<O2, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<O, S: ?Sized, D> Eq for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Vec<O::Head>>,
    O: Observer<InnerDepth = Zero, Head: Sized + Eq>,
{
}

impl<O, S: ?Sized, D, U> PartialOrd<Vec<U>> for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Vec<O::Head>>,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    Vec<O::Head>: PartialOrd<Vec<U>>,
{
    #[inline]
    fn partial_cmp(&self, other: &Vec<U>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<VecObserver<O2, S2, D2>> for VecObserver<O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
    S1: AsDeref<D1, Target = Vec<O1::Head>>,
    S2: AsDeref<D2, Target = Vec<O2::Head>>,
    Vec<O1::Head>: PartialOrd<Vec<O2::Head>>,
{
    #[inline]
    fn partial_cmp(&self, other: &VecObserver<O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<O, S: ?Sized, D> Ord for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Vec<O::Head>>,
    O: Observer<InnerDepth = Zero, Head: Sized + Ord>,
{
    #[inline]
    fn cmp(&self, other: &VecObserver<O, S, D>) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
    }
}

impl<O, S: ?Sized, D, T, I> Index<I> for VecObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>>,
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
    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_change_returns_none() {
        let mut vec: Vec<i32> = vec![];
        let mut ob = vec.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([]))));
    }

    #[test]
    fn push_triggers_append() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        ob.push(2);
        ob.push(3);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([2, 3]))));
    }

    #[test]
    fn append_vec() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        let mut extra = vec![4, 5];
        ob.append(&mut extra);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([4, 5]))));
    }

    #[test]
    fn extend_from_slice() {
        let mut vec: Vec<i32> = vec![1];
        let mut ob = vec.__observe();
        ob.extend_from_slice(&[6, 7]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([6, 7]))));
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
        assert_eq!(mutation, Some(replace!(-2, json!(99))));
    }

    #[test]
    fn index_by_usize_2() {
        let mut vec: Vec<i32> = vec![1, 2];
        let mut ob = vec.__observe();
        **ob[0] = 99;
        ob.reserve(64); // force reallocation
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(-2, json!(99))));
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
        assert_eq!(mutation, Some(replace!(_, json!([11, 12]))));
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
            Some(batch!(_, replace!(-2, json!(333)), replace!(-3, json!(222))))
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
        assert_eq!(mutation, None);
    }

    #[test]
    fn extract_if_no_match() {
        let mut vec = vec![1, 2, 3];
        let mut ob = vec.__observe();
        let extracted: Vec<_> = ob.extract_if(.., |x| *x > 10).collect();
        assert!(extracted.is_empty());
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
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
        assert_eq!(mutation, None);
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
        assert_eq!(mutation, Some(replace!(_, json!([1, 3]))));
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
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 2), append!(_, json!([3, 5])),)));
    }
}
