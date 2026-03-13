//! Observer implementation for [`VecDeque<T>`].

use std::collections::vec_deque::{Drain, IterMut};
use std::collections::{TryReserveError, VecDeque};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};

use serde::Serialize;

use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
use crate::helper::{AsDeref, AsDerefMut, ObserverState, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{MutationKind, Mutations, Observe};

/// Observer state for [`VecDeque<T>`], tracking back-end
/// [`Append`](MutationKind::Append) / [`Truncate`](MutationKind::Truncate) boundaries.
///
/// Front-end mutations (`push_front`, `pop_front`) trigger a full
/// [`Replace`](MutationKind::Replace) because the current [`MutationKind`] set has no `Prepend`
/// variant.
struct VecDequeObserverState {
    /// Number of elements truncated from the back since the last flush.
    back_truncate_len: usize,
    /// Logical index dividing "existing" elements from "appended" elements at the back.
    /// Elements at indices `[0, back_append_index)` are existing; `[back_append_index, len)` are
    /// appended.
    back_append_index: usize,
    /// Whether a front-end mutation (push_front / pop_front) occurred, forcing full Replace.
    front_mutated: bool,
}

impl VecDequeObserverState {
    fn mark_back_truncate(&mut self, new_len: usize) {
        self.back_truncate_len += self.back_append_index - new_len;
        self.back_append_index = new_len;
    }

    /// Full invalidation: all existing content is lost, emit Replace on next flush.
    /// Does NOT set front_mutated — that's only for explicit front-end operations.
    fn mark_replace(&mut self) {
        self.back_truncate_len += self.back_append_index;
        self.back_append_index = 0;
    }
}

impl ObserverState for VecDequeObserverState {
    type Target = VecDeque<u8>; // placeholder; we never use Target generically

    fn invalidate(this: &mut Self, _: &VecDeque<u8>) {
        this.mark_replace();
    }
}

/// Observer implementation for [`VecDeque<T>`].
///
/// Precisely tracks back-end `push_back` / `pop_back` as [`Append`](MutationKind::Append) /
/// [`Truncate`](MutationKind::Truncate). Front-end mutations and arbitrary modifications
/// fall back to [`Replace`](MutationKind::Replace).
pub struct VecDequeObserver<'ob, T, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    state: VecDequeObserverState,
    phantom: PhantomData<&'ob mut (T, D)>,
}

impl<'ob, T, S: ?Sized, D> Deref for VecDequeObserver<'ob, T, S, D> {
    type Target = Pointer<S>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, T, S: ?Sized, D> DerefMut for VecDequeObserver<'ob, T, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, T, S: ?Sized, D> QuasiObserver for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = VecDeque<T>>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    fn invalidate(this: &mut Self) {
        this.state.mark_replace();
    }
}

impl<'ob, T, S: ?Sized, D> Observer for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
{
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            state: VecDequeObserverState {
                back_truncate_len: 0,
                back_append_index: 0,
                front_mutated: false,
            },
            phantom: PhantomData,
        }
    }

    fn observe(head: &mut Self::Head) -> Self {
        let len = head.as_deref_mut().len();
        Self {
            state: VecDequeObserverState {
                back_truncate_len: 0,
                back_append_index: len,
                front_mutated: false,
            },
            ptr: Pointer::new(head),
            phantom: PhantomData,
        }
    }

    unsafe fn refresh(this: &mut Self, head: &mut Self::Head) {
        Pointer::set(this, head);
    }
}

impl<'ob, T, S: ?Sized, D> SerializeObserver for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    T: Serialize + 'static,
    S: AsDerefMut<D, Target = VecDeque<T>>,
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
                return Mutations::replace(slice);
            }
            return Mutations::new();
        }

        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if back_truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(back_truncate_len));
        }
        #[cfg(feature = "append")]
        if len > back_append_index {
            mutations.extend(Mutations::append(&slice[back_append_index..]));
        }
        mutations
    }
}

// ── Inherent methods ─────────────────────────────────────────

impl<'ob, T, S: ?Sized, D> VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
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
    pub fn make_contiguous(&mut self) -> &mut [T] {
        self.untracked_mut().make_contiguous()
    }

    /// See [`VecDeque::get_mut`].
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.tracked_mut().get_mut(index)
    }

    /// See [`VecDeque::front_mut`].
    pub fn front_mut(&mut self) -> Option<&mut T> {
        self.tracked_mut().front_mut()
    }

    /// See [`VecDeque::back_mut`].
    pub fn back_mut(&mut self) -> Option<&mut T> {
        self.tracked_mut().back_mut()
    }

    /// See [`VecDeque::iter_mut`].
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        self.tracked_mut().iter_mut()
    }

    /// See [`VecDeque::as_mut_slices`].
    pub fn as_mut_slices(&mut self) -> (&mut [T], &mut [T]) {
        self.tracked_mut().as_mut_slices()
    }

    /// See [`VecDeque::range_mut`].
    pub fn range_mut<R>(&mut self, range: R) -> IterMut<'_, T>
    where
        R: RangeBounds<usize>,
    {
        self.tracked_mut().range_mut(range)
    }

    /// See [`VecDeque::swap`].
    pub fn swap(&mut self, i: usize, j: usize) {
        self.tracked_mut().swap(i, j);
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
    pub fn push_front(&mut self, value: T) {
        self.state.front_mutated = true;
        self.untracked_mut().push_front(value);
    }

    /// See [`VecDeque::pop_front`].
    pub fn pop_front(&mut self) -> Option<T> {
        let value = self.untracked_mut().pop_front()?;
        self.state.front_mutated = true;
        Some(value)
    }

    /// See [`VecDeque::pop_front_if`].
    pub fn pop_front_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let front = self.untracked_mut().front_mut()?;
        if predicate(front) { self.pop_front() } else { None }
    }

    /// See [`VecDeque::swap_remove_front`].
    pub fn swap_remove_front(&mut self, index: usize) -> Option<T> {
        let value = self.untracked_mut().swap_remove_front(index)?;
        self.state.front_mutated = true;
        Some(value)
    }
}

// ── Back-end append tracking (feature = "append") ─────────────

#[cfg(feature = "append")]
impl<'ob, T, S: ?Sized, D> VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
{
    /// See [`VecDeque::push_back`].
    pub fn push_back(&mut self, value: T) {
        self.untracked_mut().push_back(value);
    }

    /// See [`VecDeque::append`].
    pub fn append(&mut self, other: &mut VecDeque<T>) {
        self.untracked_mut().append(other);
    }

    /// See [`VecDeque::insert`].
    pub fn insert(&mut self, index: usize, value: T) {
        if index >= self.state.back_append_index {
            self.untracked_mut().insert(index, value);
        } else {
            self.tracked_mut().insert(index, value);
        }
    }
}

// ── Back-end truncate + append tracking ─────────────

#[cfg(any(feature = "append", feature = "truncate"))]
impl<'ob, T, S: ?Sized, D> VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
{
    /// See [`VecDeque::pop_back`].
    pub fn pop_back(&mut self) -> Option<T> {
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
    pub fn pop_back_if(&mut self, predicate: impl FnOnce(&mut T) -> bool) -> Option<T> {
        let back = self.untracked_mut().back_mut()?;
        if predicate(back) { self.pop_back() } else { None }
    }

    /// See [`VecDeque::clear`].
    pub fn clear(&mut self) {
        if self.state.back_append_index == 0 && !self.state.front_mutated {
            self.untracked_mut().clear();
        } else {
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
            self.tracked_mut().truncate(len);
            return;
        }
        self.state.mark_back_truncate(len);
        self.untracked_mut().truncate(len);
    }

    /// See [`VecDeque::remove`].
    pub fn remove(&mut self, index: usize) -> Option<T> {
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
    pub fn swap_remove_back(&mut self, index: usize) -> Option<T> {
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
    pub fn split_off(&mut self, at: usize) -> VecDeque<T> {
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
        F: FnMut(&T) -> bool,
    {
        self.tracked_mut().retain(f);
    }

    /// See [`VecDeque::retain_mut`].
    pub fn retain_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut T) -> bool,
    {
        self.tracked_mut().retain_mut(f);
    }

    /// See [`VecDeque::resize_with`].
    pub fn resize_with(&mut self, new_len: usize, generator: impl FnMut() -> T) {
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
    pub fn drain<R>(&mut self, range: R) -> Drain<'_, T>
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
            return self.tracked_mut().drain(range);
        }
        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).untracked_ref().len(),
        };
        if end < bai {
            return self.tracked_mut().drain(range);
        }
        self.state.mark_back_truncate(start);
        self.tracked_mut().drain(range)
    }
}

#[cfg(any(feature = "append", feature = "truncate"))]
impl<'ob, T: Clone, S: ?Sized, D> VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
{
    /// See [`VecDeque::resize`].
    pub fn resize(&mut self, new_len: usize, value: T) {
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

// ── Extend ──────────────────────────────────────────

#[cfg(feature = "append")]
impl<'ob, T, S: ?Sized, D, U> Extend<U> for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
    VecDeque<T>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        self.untracked_mut().extend(other);
    }
}

// ── IndexMut ────────────────────────────────────────

impl<'ob, T, S: ?Sized, D> Index<usize> for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = VecDeque<T>>,
{
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.untracked_ref()[index]
    }
}

impl<'ob, T, S: ?Sized, D> IndexMut<usize> for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = VecDeque<T>>,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}

// ── Trait impls ─────────────────────────────────────

impl<'ob, T, S: ?Sized, D> Debug for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    T: Debug,
    S: AsDeref<D, Target = VecDeque<T>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("VecDequeObserver").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, T, S: ?Sized, D, U> PartialEq<VecDeque<U>> for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = VecDeque<T>>,
    VecDeque<T>: PartialEq<VecDeque<U>>,
{
    fn eq(&self, other: &VecDeque<U>) -> bool {
        self.untracked_ref().eq(other)
    }
}

impl<'ob, T, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<VecDequeObserver<'ob, T, S2, D2>>
    for VecDequeObserver<'ob, T, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    T: PartialEq,
    S1: AsDeref<D1, Target = VecDeque<T>>,
    S2: AsDeref<D2, Target = VecDeque<T>>,
{
    fn eq(&self, other: &VecDequeObserver<'ob, T, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, T, S: ?Sized, D> Eq for VecDequeObserver<'ob, T, S, D>
where
    D: Unsigned,
    T: Eq,
    S: AsDeref<D, Target = VecDeque<T>>,
{
}

impl<T: Serialize + 'static> Observe for VecDeque<T> {
    type Observer<'ob, S, D>
        = VecDequeObserver<'ob, T, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [T: Serialize + 'static] RefObserve for VecDeque<T>;
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
        let mut deque = VecDeque::from([1, 2, 3]);
        let mut ob = deque.__observe();
        ob[1] = 99;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([1, 99, 3]))));
    }
}
