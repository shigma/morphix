//! Observer implementation for [`VecDeque<T>`].

use std::cell::UnsafeCell;
use std::collections::vec_deque::Drain;
use std::collections::{TryReserveError, VecDeque};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};

use serde::Serialize;

use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
use crate::helper::{AsDeref, AsDerefMut, ObserverState, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{MutationKind, Mutations, Observe, PathSegment};

/// Observer state for [`VecDeque<T>`], tracking back-end
/// [`Append`](MutationKind::Append) / [`Truncate`](MutationKind::Truncate) boundaries.
///
/// Front-end mutations (`push_front`, `pop_front`) trigger a full
/// [`Replace`](MutationKind::Replace) because the current [`MutationKind`] set has no `Prepend`
/// variant.
struct VecDequeObserverState<O> {
    /// Number of elements truncated from the back since the last flush.
    back_truncate_len: usize,
    /// Logical index dividing "existing" elements from "appended" elements at the back.
    /// Elements at indices `[0, back_append_index)` are existing; `[back_append_index, len)` are
    /// appended.
    back_append_index: usize,
    /// Whether a front-end mutation (push_front / pop_front) occurred, forcing full Replace.
    front_mutated: bool,
    /// Lazily-initialized element observer storage.
    ///
    /// Mirrors the logical layout of the observed `VecDeque<T>`. Observers are created lazily
    /// via [`force_range`](VecDequeObserverState::force_range) on first mutable access.
    inner: UnsafeCell<VecDeque<O>>,
}

impl<O> VecDequeObserverState<O> {
    fn mark_back_truncate(&mut self, new_len: usize) {
        self.back_truncate_len += self.back_append_index - new_len;
        self.back_append_index = new_len;
    }

    /// Full invalidation: all existing content is lost, emit Replace on next flush.
    /// Does NOT set front_mutated — that's only for explicit front-end operations.
    fn mark_replace(&mut self) {
        self.inner.get_mut().clear();
        self.back_truncate_len += self.back_append_index;
        self.back_append_index = 0;
    }
}

impl<O> ObserverState for VecDequeObserverState<O>
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type Target = VecDeque<O::Head>;

    fn invalidate(this: &mut Self, _: &VecDeque<O::Head>) {
        this.mark_replace();
    }
}

/// Observer implementation for [`VecDeque<T>`].
///
/// Precisely tracks back-end `push_back` / `pop_back` as [`Append`](MutationKind::Append) /
/// [`Truncate`](MutationKind::Truncate). Front-end mutations and arbitrary modifications
/// fall back to [`Replace`](MutationKind::Replace).
///
/// Inner element observers are stored in a parallel `VecDeque<O>`, enabling fine-grained
/// mutation tracking for individual elements (e.g., modifying a field of a struct inside the
/// deque produces a path like `[-2].field` instead of a whole-deque Replace).
pub struct VecDequeObserver<'ob, O, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    state: VecDequeObserverState<O>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, O, S: ?Sized, D> Deref for VecDequeObserver<'ob, O, S, D> {
    type Target = Pointer<S>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, O, S: ?Sized, D> DerefMut for VecDequeObserver<'ob, O, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, O, S: ?Sized, D> QuasiObserver for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDeref<D, Target = VecDeque<O::Head>>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    fn invalidate(this: &mut Self) {
        this.state.mark_replace();
    }
}

impl<'ob, O, S: ?Sized, D> Observer for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    fn observe(head: &mut Self::Head) -> Self {
        let len = head.as_deref_mut().len();
        Self {
            state: VecDequeObserverState {
                back_truncate_len: 0,
                back_append_index: len,
                front_mutated: false,
                inner: UnsafeCell::new(VecDeque::new()),
            },
            ptr: Pointer::new(head),
            phantom: PhantomData,
        }
    }

    unsafe fn refresh(this: &mut Self, head: &mut Self::Head) {
        Pointer::set(this, head);
    }
}

impl<'ob, O, S: ?Sized, D> SerializeObserver for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized> + SerializeObserver,
    O::Head: Serialize + 'static,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let deque = (*this.ptr).as_deref_mut();
        let len = deque.len();
        let back_append_index = core::mem::replace(&mut this.state.back_append_index, len);
        let back_truncate_len = core::mem::replace(&mut this.state.back_truncate_len, 0);
        let front_mutated = core::mem::replace(&mut this.state.front_mutated, false);

        // Make contiguous so we can take slices for serialization.
        let slice = deque.make_contiguous();

        // If front was mutated, or if all existing content was replaced, emit whole Replace.
        if front_mutated || (back_append_index == 0 && back_truncate_len > 0) {
            if len > 0 || back_truncate_len > 0 {
                // Clear stale inner observers.
                this.state.inner.get_mut().clear();
                return Mutations::replace(slice);
            }
            return Mutations::new();
        }

        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if back_truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(back_truncate_len));
        }

        // Force-init inner observers for existing elements so we can flush them.
        unsafe { force_range(&this.state.inner, slice, 0, back_append_index) };

        #[cfg(feature = "append")]
        if len > back_append_index {
            mutations.extend(Mutations::append(&slice[back_append_index..]));
        }

        // Flush inner observers for existing elements.
        let inner = this.state.inner.get_mut();
        // inner might be shorter than back_append_index if not all elements were accessed;
        // force_range above ensures it's at least back_append_index long.
        let existing_len = back_append_index.min(inner.len());
        let existing = inner.make_contiguous();
        let mut is_replace = true;
        for (index, ob) in existing[..existing_len].iter_mut().enumerate().rev() {
            let mutations_i = unsafe { SerializeObserver::flush(ob) };
            is_replace &= mutations_i.is_replace();
            mutations.insert(PathSegment::Negative(len - index), mutations_i);
        }

        // Clear stale observers beyond the existing region.
        inner.truncate(existing_len);

        if is_replace && (back_append_index > 0 || back_truncate_len > 0) {
            return Mutations::replace(slice);
        }

        mutations
    }
}

/// Force-initializes element observers for the range `[start, end)`.
///
/// # Safety
///
/// The caller must ensure no references from previous `force_range` calls are alive.
unsafe fn force_range<O>(inner: &UnsafeCell<VecDeque<O>>, deque_slice: &mut [O::Head], start: usize, end: usize)
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    let observers = unsafe { &mut *inner.get() };
    let current_len = observers.len();
    if current_len < deque_slice.len() {
        for value in deque_slice[current_len..].iter_mut() {
            observers.push_back(O::observe(value));
        }
    }
    let ob_contiguous = observers.make_contiguous();
    for i in start..end {
        unsafe { Observer::refresh(&mut ob_contiguous[i], &mut deque_slice[i]) };
    }
}

impl<'ob, O, S: ?Sized, D> VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    delegate_methods! { untracked_mut() as VecDeque =>
        pub fn reserve_exact(&mut self, additional: usize);
        pub fn reserve(&mut self, additional: usize);
        pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn shrink_to_fit(&mut self);
        pub fn shrink_to(&mut self, min_capacity: usize);
    }

    /// See [`VecDeque::make_contiguous`].
    ///
    /// This only rearranges internal memory layout without changing logical order.
    /// Returns a mutable slice of inner element observers.
    pub fn make_contiguous(&mut self) -> &mut [O] {
        let deque = (*self.ptr).as_deref_mut();
        let deque_slice = deque.make_contiguous();
        let len = deque_slice.len();
        unsafe { force_range(&self.state.inner, deque_slice, 0, len) };
        self.state.inner.get_mut().make_contiguous()
    }

    /// Force-initializes and returns a mutable reference to the element observer at `index`.
    #[expect(clippy::mut_from_ref)]
    fn force_index(&self, index: usize) -> Option<&mut O> {
        let deque = unsafe { Pointer::as_mut(&self.ptr).as_deref_mut() };
        let len = deque.len();
        if index >= len {
            return None;
        }
        // Make contiguous so force_range can work with a slice.
        let slice = deque.make_contiguous();
        unsafe { force_range(&self.state.inner, slice, index, index + 1) };
        let observers = unsafe { &mut *self.state.inner.get() };
        observers.get_mut(index)
    }

    /// Force-initializes and returns mutable references to all element observers.
    fn force_all(&mut self) -> &mut VecDeque<O> {
        let deque = (*self.ptr).as_deref_mut();
        let slice = deque.make_contiguous();
        let len = slice.len();
        unsafe { force_range(&self.state.inner, slice, 0, len) };
        self.state.inner.get_mut()
    }

    /// See [`VecDeque::get_mut`].
    pub fn get_mut(&mut self, index: usize) -> Option<&mut O> {
        let deque = (*self.ptr).as_deref_mut();
        let len = deque.len();
        if index >= len {
            return None;
        }
        let slice = deque.make_contiguous();
        unsafe { force_range(&self.state.inner, slice, index, index + 1) };
        self.state.inner.get_mut().get_mut(index)
    }

    /// See [`VecDeque::front_mut`].
    pub fn front_mut(&mut self) -> Option<&mut O> {
        if (*self).untracked_ref().is_empty() {
            return None;
        }
        self.get_mut(0)
    }

    /// See [`VecDeque::back_mut`].
    pub fn back_mut(&mut self) -> Option<&mut O> {
        let len = (*self).untracked_ref().len();
        if len == 0 {
            return None;
        }
        self.get_mut(len - 1)
    }

    /// See [`VecDeque::iter_mut`].
    ///
    /// Returns an iterator over mutable references to inner element observers.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut O> {
        let observers = self.force_all();
        observers.iter_mut()
    }

    /// See [`VecDeque::as_mut_slices`].
    ///
    /// Returns observer slices. Since `force_all` makes the observer deque contiguous,
    /// the second slice will be empty.
    pub fn as_mut_slices(&mut self) -> (&mut [O], &mut [O]) {
        self.force_all();
        self.state.inner.get_mut().as_mut_slices()
    }

    /// See [`VecDeque::range_mut`].
    pub fn range_mut<R>(&mut self, range: R) -> impl Iterator<Item = &mut O>
    where
        R: RangeBounds<usize> + Clone,
    {
        let deque = (*self.ptr).as_deref_mut();
        let len = deque.len();
        let start = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => len,
        };
        let slice = deque.make_contiguous();
        unsafe { force_range(&self.state.inner, slice, start, end) };
        self.state.inner.get_mut().range_mut(range)
    }

    /// See [`VecDeque::swap`].
    pub fn swap(&mut self, i: usize, j: usize) {
        if i != j {
            // Invalidate observers for swapped elements.
            let observers = self.state.inner.get_mut();
            if let Some(ob) = observers.get_mut(i) {
                QuasiObserver::invalidate(ob);
            }
            if let Some(ob) = observers.get_mut(j) {
                QuasiObserver::invalidate(ob);
            }
            self.untracked_mut().swap(i, j);
        }
    }

    /// See [`VecDeque::rotate_left`].
    pub fn rotate_left(&mut self, n: usize) {
        if n != 0 && (*self).untracked_ref().len() > 1 {
            self.tracked_mut().rotate_left(n);
        }
    }

    /// See [`VecDeque::rotate_right`].
    pub fn rotate_right(&mut self, n: usize) {
        if n != 0 && (*self).untracked_ref().len() > 1 {
            self.tracked_mut().rotate_right(n);
        }
    }

    /// See [`VecDeque::push_front`].
    pub fn push_front(&mut self, value: O::Head) {
        self.state.front_mutated = true;
        self.state.inner.get_mut().clear();
        self.untracked_mut().push_front(value);
    }

    /// See [`VecDeque::pop_front`].
    pub fn pop_front(&mut self) -> Option<O::Head> {
        let value = self.untracked_mut().pop_front()?;
        self.state.front_mutated = true;
        self.state.inner.get_mut().clear();
        Some(value)
    }

    /// See [`VecDeque::pop_front_if`].
    pub fn pop_front_if(&mut self, predicate: impl FnOnce(&mut O::Head) -> bool) -> Option<O::Head> {
        // We need to check predicate without committing to front_mutated.
        let front = self.untracked_mut().front_mut()?;
        if predicate(front) { self.pop_front() } else { None }
    }

    /// See [`VecDeque::swap_remove_front`].
    pub fn swap_remove_front(&mut self, index: usize) -> Option<O::Head> {
        let value = self.untracked_mut().swap_remove_front(index)?;
        self.state.front_mutated = true;
        self.state.inner.get_mut().clear();
        Some(value)
    }
}

#[cfg(feature = "append")]
impl<'ob, O, S: ?Sized, D> VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    /// See [`VecDeque::push_back`].
    pub fn push_back(&mut self, value: O::Head) {
        self.untracked_mut().push_back(value);
    }

    /// See [`VecDeque::append`].
    pub fn append(&mut self, other: &mut VecDeque<O::Head>) {
        self.untracked_mut().append(other);
    }

    /// See [`VecDeque::insert`].
    pub fn insert(&mut self, index: usize, value: O::Head) {
        if index >= self.state.back_append_index {
            self.untracked_mut().insert(index, value);
        } else {
            self.state.inner.get_mut().clear();
            self.tracked_mut().insert(index, value);
        }
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<'ob, O, S: ?Sized, D> VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    /// See [`VecDeque::pop_back`].
    pub fn pop_back(&mut self) -> Option<O::Head> {
        let value = self.untracked_mut().pop_back()?;
        let len = (*self).untracked_ref().len();
        if len >= self.state.back_append_index {
            // popped from appended region, no-op
        } else if cfg!(feature = "truncate") && len + 1 == self.state.back_append_index {
            self.state.mark_back_truncate(len);
        } else {
            self.state.mark_replace();
        }
        Some(value)
    }

    /// See [`VecDeque::pop_back_if`].
    pub fn pop_back_if(&mut self, predicate: impl FnOnce(&mut O::Head) -> bool) -> Option<O::Head> {
        let back = self.untracked_mut().back_mut()?;
        if predicate(back) { self.pop_back() } else { None }
    }

    /// See [`VecDeque::clear`].
    pub fn clear(&mut self) {
        if self.state.back_append_index == 0 && !self.state.front_mutated {
            self.untracked_mut().clear();
        } else {
            self.state.inner.get_mut().clear();
            self.tracked_mut().clear();
        }
    }

    /// See [`VecDeque::truncate`].
    pub fn truncate(&mut self, len: usize) {
        let bai = self.state.back_append_index;
        if len >= bai {
            self.untracked_mut().truncate(len);
            return;
        }
        if cfg!(not(feature = "truncate")) || len == 0 {
            self.state.inner.get_mut().clear();
            self.tracked_mut().truncate(len);
            return;
        }
        self.state.mark_back_truncate(len);
        self.untracked_mut().truncate(len);
    }

    /// See [`VecDeque::remove`].
    pub fn remove(&mut self, index: usize) -> Option<O::Head> {
        let bai = self.state.back_append_index;
        let value = self.untracked_mut().remove(index)?;
        if index >= bai {
            // removed from appended region, no-op
        } else if cfg!(feature = "truncate") && index + 1 == bai {
            self.state.mark_back_truncate(index);
        } else {
            self.state.mark_replace();
        }
        Some(value)
    }

    /// See [`VecDeque::swap_remove_back`].
    pub fn swap_remove_back(&mut self, index: usize) -> Option<O::Head> {
        let bai = self.state.back_append_index;
        let value = self.untracked_mut().swap_remove_back(index)?;
        if index >= bai {
            // removed from appended region
        } else if cfg!(feature = "truncate") && index + 1 == bai {
            self.state.mark_back_truncate(index);
        } else {
            self.state.mark_replace();
        }
        Some(value)
    }

    /// See [`VecDeque::split_off`].
    pub fn split_off(&mut self, at: usize) -> VecDeque<O::Head> {
        let bai = self.state.back_append_index;
        let split = self.untracked_mut().split_off(at);
        if at >= bai {
            // split from appended region, no-op
        } else if cfg!(feature = "truncate") && at > 0 {
            self.state.mark_back_truncate(at);
        } else {
            self.state.mark_replace();
        }
        split
    }

    /// See [`VecDeque::retain`].
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&O::Head) -> bool,
    {
        self.state.inner.get_mut().clear();
        self.tracked_mut().retain(f);
    }

    /// See [`VecDeque::retain_mut`].
    pub fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut O::Head) -> bool,
    {
        self.state.inner.get_mut().clear();
        self.tracked_mut().retain_mut(f);
    }

    /// See [`VecDeque::resize_with`].
    pub fn resize_with(&mut self, new_len: usize, generator: impl FnMut() -> O::Head) {
        let bai = self.state.back_append_index;
        self.untracked_mut().resize_with(new_len, generator);
        if new_len >= bai {
            // grew or stayed same, no-op
        } else if cfg!(feature = "truncate") && new_len > 0 {
            self.state.mark_back_truncate(new_len);
        } else {
            self.state.mark_replace();
        }
    }

    /// See [`VecDeque::drain`].
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, O::Head>
    where
        R: RangeBounds<usize>,
    {
        let bai = self.state.back_append_index;
        let start = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start >= bai {
            return self.untracked_mut().drain(range);
        }
        if cfg!(not(feature = "truncate")) || start == 0 {
            self.state.inner.get_mut().clear();
            return self.tracked_mut().drain(range);
        }
        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).untracked_ref().len(),
        };
        if end < bai {
            self.state.inner.get_mut().clear();
            return self.tracked_mut().drain(range);
        }
        self.state.mark_back_truncate(start);
        self.state.inner.get_mut().clear();
        self.tracked_mut().drain(range)
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<'ob, O, S: ?Sized, D> VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    O::Head: Clone,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    /// See [`VecDeque::resize`].
    pub fn resize(&mut self, new_len: usize, value: O::Head) {
        let bai = self.state.back_append_index;
        self.untracked_mut().resize(new_len, value);
        if new_len >= bai {
            // grew or stayed same
        } else if cfg!(feature = "truncate") && new_len > 0 {
            self.state.mark_back_truncate(new_len);
        } else {
            self.state.mark_replace();
        }
    }
}

#[cfg(feature = "append")]
impl<'ob, O, S: ?Sized, D, U> Extend<U> for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
    VecDeque<O::Head>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        self.untracked_mut().extend(other);
    }
}

impl<'ob, O, S: ?Sized, D> Index<usize> for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    type Output = O;

    fn index(&self, index: usize) -> &Self::Output {
        self.force_index(index).expect("index out of bounds")
    }
}

impl<'ob, O, S: ?Sized, D> IndexMut<usize> for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDerefMut<D, Target = VecDeque<O::Head>>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).expect("index out of bounds")
    }
}

impl<'ob, O, S: ?Sized, D> Debug for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    O::Head: Debug,
    S: AsDeref<D, Target = VecDeque<O::Head>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecDequeObserver").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, O, S: ?Sized, D, U> PartialEq<VecDeque<U>> for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    S: AsDeref<D, Target = VecDeque<O::Head>>,
    VecDeque<O::Head>: PartialEq<VecDeque<U>>,
{
    fn eq(&self, other: &VecDeque<U>) -> bool {
        self.untracked_ref().eq(other)
    }
}

impl<'ob, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<VecDequeObserver<'ob, O2, S2, D2>>
    for VecDequeObserver<'ob, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
    S1: AsDeref<D1, Target = VecDeque<O1::Head>>,
    S2: AsDeref<D2, Target = VecDeque<O2::Head>>,
    VecDeque<O1::Head>: PartialEq<VecDeque<O2::Head>>,
{
    fn eq(&self, other: &VecDequeObserver<'ob, O2, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, O, S: ?Sized, D> Eq for VecDequeObserver<'ob, O, S, D>
where
    D: Unsigned,
    O: Observer<InnerDepth = Zero, Head: Sized>,
    O::Head: Eq,
    S: AsDeref<D, Target = VecDeque<O::Head>>,
{
}

impl<T: Observe> Observe for VecDeque<T> {
    type Observer<'ob, S, D>
        = VecDequeObserver<'ob, T::Observer<'ob, T, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [T: Observe] RefObserve for VecDeque<T>;
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_change_returns_none() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn reserve_returns_none() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.reserve(100);
        ob.shrink_to_fit();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn make_contiguous_returns_none() {
        let mut deque = VecDeque::new();
        deque.push_back(1);
        deque.push_back(2);
        deque.push_front(0);
        let mut ob = deque.__observe();
        ob.make_contiguous();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn push_back_triggers_append() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.push_back(2);
        ob.push_back(3);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([2, 3]))));
    }

    #[test]
    fn extend_triggers_append() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.extend([2, 3]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([2, 3]))));
    }

    #[test]
    fn append_other_deque() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        let mut other = VecDeque::from([4, 5]);
        ob.append(&mut other);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([4, 5]))));
    }

    #[test]
    fn pop_back_triggers_truncate() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.pop_back();
        ob.pop_back();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 2)));
    }

    #[test]
    fn truncate_method() {
        let mut deque = VecDeque::from([1, 2, 3, 4, 5]);
        let mut ob = deque.__observe();
        ob.truncate(2);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 3)));
    }

    #[test]
    fn pop_back_then_push_back() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.pop_back();
        ob.push_back(4);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 1), append!(_, json!([4])))));
    }

    #[test]
    fn pop_back_from_appended_region() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.push_back(2);
        ob.push_back(3);
        ob.pop_back(); // popping from appended region
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([2]))));
    }

    #[test]
    fn push_front_triggers_replace() {
        let mut deque = VecDeque::from([1, 2]);
        let mut ob = deque.__observe();
        ob.push_front(0);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([0, 1, 2]))));
    }

    #[test]
    fn pop_front_triggers_replace() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.pop_front();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([2, 3]))));
    }

    #[test]
    fn pop_front_if_true_triggers_replace() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let result = ob.pop_front_if(|x| *x == 1);
        assert_eq!(result, Some(1));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([2, 3]))));
    }

    #[test]
    fn pop_front_if_false_returns_none() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let result = ob.pop_front_if(|x| *x == 99);
        assert_eq!(result, None);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn push_front_overrides_back_append() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.push_back(2);
        ob.push_front(0); // front mutation overrides back tracking
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([0, 1, 2]))));
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut deque = VecDeque::from([1, 2]);
        let mut ob = deque.__observe();
        ob.retain(|x| *x > 1);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([2]))));
    }

    #[test]
    fn empty_deque_no_mutation() {
        let mut deque: VecDeque<i32> = VecDeque::new();
        let mut ob = deque.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn empty_deque_push_back() {
        let mut deque: VecDeque<i32> = VecDeque::new();
        let mut ob = deque.__observe();
        ob.push_back(1);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([1]))));
    }

    #[test]
    fn empty_deque_push_front() {
        let mut deque: VecDeque<i32> = VecDeque::new();
        let mut ob = deque.__observe();
        ob.push_front(1);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([1]))));
    }

    #[test]
    fn clear_empty_deque() {
        let mut deque: VecDeque<i32> = VecDeque::new();
        let mut ob = deque.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn clear_nonempty_deque() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([]))));
    }

    #[test]
    fn split_off_from_existing() {
        let mut deque = VecDeque::from([1, 2, 3, 4]);
        let mut ob = deque.__observe();
        let split = ob.split_off(2);
        assert_eq!(split, VecDeque::from([3, 4]));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 2)));
    }

    #[test]
    fn split_off_from_appended() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.push_back(2);
        ob.push_back(3);
        let split = ob.split_off(2);
        assert_eq!(split, VecDeque::from([3]));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([2]))));
    }

    #[test]
    fn remove_at_back_append_boundary() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let val = ob.remove(2);
        assert_eq!(val, Some(3));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 1)));
    }

    #[test]
    fn remove_from_middle_triggers_replace() {
        let mut deque = VecDeque::from([1, 2, 3, 4]);
        let mut ob = deque.__observe();
        let val = ob.remove(1);
        assert_eq!(val, Some(2));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([1, 3, 4]))));
    }

    #[test]
    fn insert_in_appended_region() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.push_back(2);
        ob.insert(2, 3); // insert at index 2, which is in appended region
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([2, 3]))));
    }

    #[test]
    fn insert_in_existing_region() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.insert(1, 99);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([1, 99, 2, 3]))));
    }

    #[test]
    fn swap_remove_front_triggers_replace() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let val = ob.swap_remove_front(1);
        assert_eq!(val, Some(2));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([1, 3]))));
    }

    #[test]
    fn multiple_flushes() {
        let mut deque = VecDeque::from([1, 2]);
        let mut ob = deque.__observe();
        ob.push_back(3);
        let Json(m1) = ob.flush().unwrap();
        assert_eq!(m1, Some(append!(_, json!([3]))));
        // Second flush with no changes
        let Json(m2) = ob.flush().unwrap();
        assert_eq!(m2, None);
        // Third flush with more changes
        ob.pop_back();
        let Json(m3) = ob.flush().unwrap();
        assert_eq!(m3, Some(truncate!(_, 1)));
    }

    #[test]
    fn resize_shrink() {
        let mut deque = VecDeque::from([1, 2, 3, 4, 5]);
        let mut ob = deque.__observe();
        ob.resize(2, 0);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 3)));
    }

    #[test]
    fn resize_grow() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.resize(3, 0);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([0, 0]))));
    }

    #[test]
    fn pop_back_if_true() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let result = ob.pop_back_if(|x| *x == 3);
        assert_eq!(result, Some(3));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 1)));
    }

    #[test]
    fn pop_back_if_false() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        let result = ob.pop_back_if(|x| *x == 99);
        assert_eq!(result, None);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn drain_from_appended() {
        let mut deque = VecDeque::from([1]);
        let mut ob = deque.__observe();
        ob.push_back(2);
        ob.push_back(3);
        let drained: Vec<_> = ob.drain(1..).collect();
        assert_eq!(drained, vec![2, 3]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None); // drained only appended elements
    }

    #[test]
    fn drain_straddles_boundary() {
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob.push_back(4);
        let drained: Vec<_> = ob.drain(1..).collect();
        assert_eq!(drained, vec![2, 3, 4]);
        let Json(mutation) = ob.flush().unwrap();
        // Drain straddles the append boundary, so granular tracking is lost → Replace.
        assert_eq!(mutation, Some(replace!(_, json!([1]))));
    }

    #[test]
    fn index_mut_triggers_replace() {
        let mut deque = VecDeque::from([1i32, 2, 3]);
        let mut ob = deque.__observe();
        **ob[1] = 99;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(-2, json!(99))));
    }

    #[test]
    fn index_returns_inner_observer() {
        let mut deque = VecDeque::from(["hello".to_string(), "world".to_string()]);
        let mut ob = deque.__observe();
        // Access through index should return an observer, not raw T.
        assert_eq!(ob[0], "hello");
        assert_eq!(ob[1], "world");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn modify_element_via_inner_observer() {
        let mut deque = VecDeque::from(["hello".to_string(), "world".to_string()]);
        let mut ob = deque.__observe();
        // Modify element through observer — should produce fine-grained mutation.
        // String's push_str produces Append at the element level.
        ob[0].push_str("!");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(-2, json!("!"))));
    }

    #[test]
    fn modify_multiple_elements_via_inner_observer() {
        let mut deque = VecDeque::from(["a".to_string(), "b".to_string(), "c".to_string()]);
        let mut ob = deque.__observe();
        ob[0].push_str("1");
        ob[2].push_str("3");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(batch!(_, append!(-1, json!("3")), append!(-3, json!("1"))))
        );
    }

    #[test]
    fn get_mut_returns_observer() {
        let mut deque = VecDeque::from(["foo".to_string(), "bar".to_string()]);
        let mut ob = deque.__observe();
        let elem = ob.get_mut(1).unwrap();
        elem.push_str("baz");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(-1, json!("baz"))));
    }

    #[test]
    fn front_mut_returns_observer() {
        let mut deque = VecDeque::from(["first".to_string(), "second".to_string()]);
        let mut ob = deque.__observe();
        let front = ob.front_mut().unwrap();
        front.push_str("!");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(-2, json!("!"))));
    }

    #[test]
    fn back_mut_returns_observer() {
        let mut deque = VecDeque::from(["first".to_string(), "second".to_string()]);
        let mut ob = deque.__observe();
        let back = ob.back_mut().unwrap();
        back.push_str("!");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(-1, json!("!"))));
    }

    #[test]
    fn iter_mut_returns_observers() {
        let mut deque = VecDeque::from(["a".to_string(), "b".to_string(), "c".to_string()]);
        let mut ob = deque.__observe();
        for elem in ob.iter_mut() {
            elem.push_str("!");
        }
        let Json(mutation) = ob.flush().unwrap();
        // All elements have fine-grained Append mutations.
        // Since not all report Replace, they are emitted individually.
        assert_eq!(
            mutation,
            Some(batch!(
                _,
                append!(-1, json!("!")),
                append!(-2, json!("!")),
                append!(-3, json!("!"))
            ))
        );
    }

    #[test]
    fn make_contiguous_returns_observer_slice() {
        let mut deque = VecDeque::from(["x".to_string(), "y".to_string()]);
        let mut ob = deque.__observe();
        let slice = ob.make_contiguous();
        slice[0].push_str("1");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(-2, json!("1"))));
    }

    #[test]
    fn modify_then_append() {
        let mut deque = VecDeque::from(["a".to_string()]);
        let mut ob = deque.__observe();
        ob[0].push_str("!");
        ob.push_back("b".to_string());
        let Json(mutation) = ob.flush().unwrap();
        // Element 0 has Append (not Replace), so no collapse.
        assert_eq!(
            mutation,
            Some(batch!(_, append!(_, json!(["b"])), append!(-2, json!("!"))))
        );
    }

    #[test]
    fn no_modify_then_append() {
        let mut deque = VecDeque::from(["a".to_string()]);
        let mut ob = deque.__observe();
        // Access without modification.
        let _ = &ob[0];
        ob.push_back("b".to_string());
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!(["b"]))));
    }

    #[test]
    fn modify_element_then_pop_back() {
        let mut deque = VecDeque::from(["a".to_string(), "b".to_string(), "c".to_string()]);
        let mut ob = deque.__observe();
        ob[0].push_str("!");
        ob.pop_back();
        let Json(mutation) = ob.flush().unwrap();
        // Element 0 has Append (not Replace), so fine-grained: truncate + element append.
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 1), append!(-2, json!("!")))));
    }

    #[test]
    fn index_read_only_no_mutation() {
        let mut deque = VecDeque::from(["hello".to_string(), "world".to_string()]);
        let mut ob = deque.__observe();
        // Read-only access through index should NOT produce mutations.
        let _val = &ob[0];
        let _val2 = &ob[1];
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn pop_push_clears_stale_observer_state() {
        let mut deque = VecDeque::from(["a".to_string(), "b".to_string(), "ab".to_string()]);
        let mut ob = deque.__observe();

        // Modify element 2, then pop and push back in the same cycle.
        ob[2].truncate(1);
        ob.pop_back();
        ob.push_back("cd".to_string());
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some()); // Truncate(1) + Append(["cd"])

        // Next cycle: element 2 should have a fresh observer.
        assert_eq!(ob[2], "cd");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }
}
