//! Observer implementation for [`BTreeSet<T>`].

use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::default_impl_ref_observe;
use crate::helper::{AsDeref, AsDerefMut, ObserverState, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Mutations, Observe};

struct BTreeSetObserverState<T> {
    /// The last element (inclusive) of the common prefix between the old and current set.
    /// `None` means the prefix is empty (either the set was initially empty, or all original
    /// elements have been moved to the tail).
    boundary: Option<T>,
    /// Number of elements in the *original* set (at last flush/observe) that are strictly
    /// after `boundary`. This becomes `truncate_len` on flush.
    truncate_len: usize,
}

impl<T> Default for BTreeSetObserverState<T> {
    fn default() -> Self {
        Self {
            boundary: None,
            truncate_len: 0,
        }
    }
}

impl<T: Clone + Ord> BTreeSetObserverState<T> {
    /// Move all original elements in `[v, boundary]` from the prefix into the tail,
    /// then shrink `boundary` to the predecessor of `v`.
    fn shrink_boundary(&mut self, v: &T, set: &BTreeSet<T>) {
        let boundary = self.boundary.as_ref().unwrap();
        self.truncate_len += set.range(v..=boundary).count();
        self.boundary = set.range(..v).next_back().cloned();
    }
}

impl<T: Clone + Ord> ObserverState for BTreeSetObserverState<T> {
    type Target = BTreeSet<T>;

    fn invalidate(this: &mut Self, set: &BTreeSet<T>) {
        if let Some(boundary) = this.boundary.take() {
            this.truncate_len += set.range(..=boundary).count();
        }
    }
}

/// Observer implementation for [`BTreeSet<T>`].
///
/// Tracks granular mutations by maintaining a prefix boundary. Elements up to and including
/// the boundary are unchanged from the last flush; elements beyond the boundary form the
/// "tail" region that will be emitted as [`Truncate`](crate::MutationKind::Truncate) /
/// [`Append`](crate::MutationKind::Append) mutations.
///
/// ## Limitations
///
/// Most methods require `T: Clone` because the observer stores the boundary element.
pub struct BTreeSetObserver<'ob, T, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    state: BTreeSetObserverState<T>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, T, S: ?Sized, D> Deref for BTreeSetObserver<'ob, T, S, D> {
    type Target = Pointer<S>;

    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, T, S: ?Sized, D> DerefMut for BTreeSetObserver<'ob, T, S, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, T, S: ?Sized, D> QuasiObserver for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    fn invalidate(this: &mut Self) {
        ObserverState::invalidate(&mut this.state, (*this.ptr).as_deref());
    }
}

impl<'ob, T, S: ?Sized, D> Observer for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeSet<T>>,
{
    fn observe(head: &mut Self::Head) -> Self {
        let this = Self {
            state: BTreeSetObserverState {
                boundary: head.as_deref_mut().last().cloned(),
                truncate_len: 0,
            },
            ptr: Pointer::new(head),
            phantom: PhantomData,
        };
        Pointer::register_state::<_, D>(&this.ptr, &this.state);
        this
    }

    unsafe fn relocate(this: &mut Self, head: &mut Self::Head) {
        Pointer::set(this, head);
    }
}

impl<'ob, T, S: ?Sized, D> SerializeObserver for BTreeSetObserver<'ob, T, S, D>
where
    T: Serialize + Clone + Ord + 'static,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let set = (*this.ptr).as_deref();
        let truncate_len = std::mem::replace(&mut this.state.truncate_len, 0);
        let boundary = std::mem::replace(&mut this.state.boundary, set.last().cloned());

        let prefix_len = match &boundary {
            Some(b) => set.range(..=b).count(),
            None => 0,
        };
        if prefix_len == 0 && truncate_len > 0 {
            return Mutations::replace(set);
        }

        let mut mutations = Mutations::new();

        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            mutations.extend(crate::MutationKind::Truncate(truncate_len));
        }

        #[cfg(feature = "append")]
        {
            // BTreeSet has no contiguous slice representation, so we collect the appended
            // elements into an owned Vec and box it as the Append value.
            let appended: Vec<T> = set.iter().skip(prefix_len).cloned().collect();
            if !appended.is_empty() {
                mutations.extend(crate::MutationKind::Append(
                    Box::new(appended) as Box<dyn erased_serde::Serialize>
                ));
            }
        }

        mutations
    }
}

impl<'ob, T, S: ?Sized, D> BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeSet<T>>,
{
    /// See [`BTreeSet::clear`].
    pub fn clear(&mut self) {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut().clear()
        } else {
            self.tracked_mut().clear()
        }
    }

    /// See [`BTreeSet::insert`].
    pub fn insert(&mut self, value: T) -> bool {
        if let Some(boundary) = &self.state.boundary
            && value <= *boundary
        {
            let set = (*self.ptr).as_deref();
            self.state.shrink_boundary(&value, set);
        }
        (*self.ptr).as_deref_mut().insert(value)
    }

    /// See [`BTreeSet::replace`].
    pub fn replace(&mut self, value: T) -> Option<T> {
        if let Some(boundary) = &self.state.boundary
            && value <= *boundary
        {
            let set = (*self.ptr).as_deref();
            self.state.shrink_boundary(&value, set);
        }
        (*self.ptr).as_deref_mut().replace(value)
    }

    /// See [`BTreeSet::remove`].
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if let Some(boundary) = &self.state.boundary
            && let Some(found) = (*self.ptr).as_deref().get(value)
            && found <= boundary
        {
            self.state.shrink_boundary(found, (*self.ptr).as_deref());
        }
        (*self.ptr).as_deref_mut().remove(value)
    }

    /// See [`BTreeSet::take`].
    pub fn take<Q>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        if let Some(boundary) = &self.state.boundary
            && let Some(found) = (*self.ptr).as_deref().get(value)
            && found <= boundary
        {
            self.state.shrink_boundary(found, (*self.ptr).as_deref());
        }
        (*self.ptr).as_deref_mut().take(value)
    }

    /// See [`BTreeSet::pop_first`].
    pub fn pop_first(&mut self) -> Option<T> {
        let set = (*self.ptr).as_deref();
        if let Some(first) = set.first()
            && let Some(boundary) = &self.state.boundary
            && first <= boundary
        {
            self.state.shrink_boundary(first, set);
        }
        (*self.ptr).as_deref_mut().pop_first()
    }

    /// See [`BTreeSet::pop_last`].
    pub fn pop_last(&mut self) -> Option<T> {
        let set = (*self.ptr).as_deref();
        if let Some(last) = set.last()
            && let Some(boundary) = &self.state.boundary
            && last <= boundary
        {
            self.state.shrink_boundary(last, set);
        }
        (*self.ptr).as_deref_mut().pop_last()
    }

    /// See [`BTreeSet::retain`].
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut().retain(f);
        } else {
            self.tracked_mut().retain(f);
        }
    }

    /// See [`BTreeSet::append`].
    pub fn append(&mut self, other: &mut BTreeSet<T>) {
        for value in std::mem::take(other) {
            self.insert(value);
        }
    }

    /// See [`BTreeSet::split_off`].
    pub fn split_off(&mut self, value: &T) -> BTreeSet<T> {
        if let Some(boundary) = &self.state.boundary
            && let Some(first_split) = (*self.ptr).as_deref().range(value..).next()
            && first_split <= boundary
        {
            self.state.shrink_boundary(first_split, (*self.ptr).as_deref());
        }
        (*self.ptr).as_deref_mut().split_off(value)
    }

    /// See [`BTreeSet::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> std::collections::btree_set::ExtractIf<'_, T, R, F>
    where
        R: std::ops::RangeBounds<T>,
        F: FnMut(&T) -> bool,
    {
        ObserverState::invalidate(&mut self.state, (*self.ptr).as_deref());
        (*self.ptr).as_deref_mut().extract_if(range, pred)
    }
}

impl<'ob, T, S: ?Sized, D> Debug for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    BTreeSet<T>: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BTreeSetObserver").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, T, S: ?Sized, D> PartialEq<BTreeSet<T>> for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    BTreeSet<T>: PartialEq,
{
    fn eq(&self, other: &BTreeSet<T>) -> bool {
        self.untracked_ref().eq(other)
    }
}

impl<'ob, T1, T2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<BTreeSetObserver<'ob, T2, S2, D2>>
    for BTreeSetObserver<'ob, T1, S1, D1>
where
    T1: Clone + Ord,
    T2: Clone + Ord,
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1, Target = BTreeSet<T1>>,
    S2: AsDeref<D2, Target = BTreeSet<T2>>,
    BTreeSet<T1>: PartialEq<BTreeSet<T2>>,
{
    fn eq(&self, other: &BTreeSetObserver<'ob, T2, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, T, S, D> Eq for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    BTreeSet<T>: Eq,
{
}

impl<'ob, T, S: ?Sized, D> PartialOrd<BTreeSet<T>> for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    BTreeSet<T>: PartialOrd,
{
    fn partial_cmp(&self, other: &BTreeSet<T>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other)
    }
}

impl<'ob, T1, T2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<BTreeSetObserver<'ob, T2, S2, D2>>
    for BTreeSetObserver<'ob, T1, S1, D1>
where
    T1: Clone + Ord,
    T2: Clone + Ord,
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1, Target = BTreeSet<T1>>,
    S2: AsDeref<D2, Target = BTreeSet<T2>>,
    BTreeSet<T1>: PartialOrd<BTreeSet<T2>>,
{
    fn partial_cmp(&self, other: &BTreeSetObserver<'ob, T2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<'ob, T, S, D> Ord for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    BTreeSet<T>: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
    }
}

impl<'ob, T, S: ?Sized, D, U> Extend<U> for BTreeSetObserver<'ob, T, S, D>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeSet<T>>,
    BTreeSet<T>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, iter: I) {
        self.tracked_mut().extend(iter);
    }
}

impl<T: Clone + Ord> Observe for BTreeSet<T> {
    type Observer<'ob, S, D>
        = BTreeSetObserver<'ob, T, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [T] RefObserve for BTreeSet<T>;
}

impl<T> Snapshot for BTreeSet<T>
where
    T: Snapshot,
    T::Snapshot: Ord,
{
    type Snapshot = BTreeSet<T::Snapshot>;

    fn to_snapshot(&self) -> Self::Snapshot {
        self.iter().map(|item| item.to_snapshot()).collect()
    }

    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.len() == snapshot.len() && self.iter().zip(snapshot.iter()).all(|(a, b)| a.eq_snapshot(b))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_change() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn insert_append() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.insert(4);
        ob.insert(5);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!([4, 5]))));
    }

    #[test]
    fn remove_last_as_truncate() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.remove(&3);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 1)));
    }

    #[test]
    fn remove_middle() {
        let mut set = BTreeSet::from([1, 2, 3, 4, 5]);
        let mut ob = set.__observe();
        ob.remove(&3);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 3), append!(_, json!([4, 5])))));
    }

    #[test]
    fn insert_middle_then_append() {
        let mut set = BTreeSet::from([1, 3, 5]);
        let mut ob = set.__observe();
        ob.insert(2);
        ob.insert(6);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(batch!(_, truncate!(_, 2), append!(_, json!([2, 3, 5, 6]))))
        );
    }

    #[test]
    fn clear_non_empty() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([]))));
    }

    #[test]
    fn clear_empty() {
        let mut set: BTreeSet<i32> = BTreeSet::new();
        let mut ob = set.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        **ob = BTreeSet::from([4, 5]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([4, 5]))));
    }

    #[test]
    fn double_flush() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.insert(4);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn pop_first() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        assert_eq!(ob.pop_first(), Some(1));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!([2, 3]))));
    }

    #[test]
    fn pop_last_as_truncate() {
        let mut set = BTreeSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        assert_eq!(ob.pop_last(), Some(3));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 1)));
    }
}
