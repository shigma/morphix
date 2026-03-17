//! Observer implementation for [`BTreeSet<T>`].

use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut, RangeBounds};

use serde::Serialize;

use crate::general::Snapshot;
use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
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
        let boundary = match &self.boundary {
            Some(boundary) if boundary >= v => boundary,
            _ => return,
        };
        self.truncate_len += set.range(v..=boundary).count();
        self.boundary = set.range(..v).next_back().cloned();
    }
}

impl<T: Ord> ObserverState for BTreeSetObserverState<T> {
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
    T: Ord,
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
    fn nonempty_mut(&mut self) -> &mut BTreeSet<T> {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut()
        } else {
            self.tracked_mut()
        }
    }

    delegate_methods! { nonempty_mut() as BTreeSet =>
        pub fn clear(&mut self);
        pub fn pop_first(&mut self) -> Option<T>;
    }

    /// See [`BTreeSet::pop_last`].
    pub fn pop_last(&mut self) -> Option<T> {
        let set = (*self.ptr).as_deref();
        if let Some(last) = set.last() {
            self.state.shrink_boundary(last, set);
        }
        self.untracked_mut().pop_last()
    }

    /// See [`BTreeSet::insert`].
    pub fn insert(&mut self, value: T) -> bool {
        let set = (*self.ptr).as_deref();
        self.state.shrink_boundary(&value, set);
        self.untracked_mut().insert(value)
    }

    /// See [`BTreeSet::replace`].
    pub fn replace(&mut self, value: T) -> Option<T> {
        let set = (*self.ptr).as_deref();
        self.state.shrink_boundary(&value, set);
        self.untracked_mut().replace(value)
    }

    /// See [`BTreeSet::remove`].
    pub fn remove<Q>(&mut self, value: &Q) -> bool
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let set = (*self.ptr).as_deref();
        if let Some(found) = set.get(value) {
            self.state.shrink_boundary(found, set);
        }
        self.untracked_mut().remove(value)
    }

    /// See [`BTreeSet::take`].
    pub fn take<Q>(&mut self, value: &Q) -> Option<T>
    where
        T: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        let set = (*self.ptr).as_deref();
        if let Some(found) = set.get(value) {
            self.state.shrink_boundary(found, set);
        }
        self.untracked_mut().take(value)
    }

    /// See [`BTreeSet::retain`].
    #[rustversion::since(1.91)]
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.extract_if(.., |v| !f(v)).for_each(drop);
    }

    /// See [`BTreeSet::append`].
    pub fn append(&mut self, other: &mut BTreeSet<T>) {
        let set = (*self.ptr).as_deref();
        if let Some(first) = other.first() {
            self.state.shrink_boundary(first, set);
        }
        self.untracked_mut().append(other);
    }

    /// See [`BTreeSet::split_off`].
    pub fn split_off(&mut self, value: &T) -> BTreeSet<T> {
        let set = (*self.ptr).as_deref();
        if let Some(found) = set.get(value) {
            self.state.shrink_boundary(found, set);
        }
        self.untracked_mut().split_off(value)
    }

    /// See [`BTreeSet::extract_if`].
    #[rustversion::since(1.91)]
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> ExtractIf<'_, 'ob, T, S, D, R, F>
    where
        R: RangeBounds<T>,
        F: FnMut(&T) -> bool,
    {
        let set = unsafe { Pointer::as_mut(&self.ptr).as_deref_mut() };
        let inner = MaybeUninit::new(set.extract_if(range, pred));
        ExtractIf {
            inner,
            ob: self,
            first_extracted: None,
            extracted_count: 0,
        }
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

/// Iterator produced by [`BTreeSetObserver::extract_if`].
#[rustversion::since(1.91)]
pub struct ExtractIf<'a, 'ob, T, S: ?Sized, D, R, F>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
{
    /// Wrapped in [`MaybeUninit`] (a union) to prevent SB from deep-retagging the internal mutable
    /// references inside stdlib's [`ExtractIf`](std::collections::btree_set::ExtractIf) when
    /// [`Drop::drop`] is entered. SB does not recurse into unions during retagging, so the strongly
    /// protected Unique tag from the [`drop_in_place`](std::ptr::drop_in_place) shim won't cover
    /// those inner references, allowing subsequent [`Pointer`]-based reads of the [`BTreeSet`]
    /// after the inner iterator is dropped.
    inner: MaybeUninit<std::collections::btree_set::ExtractIf<'a, T, R, F>>,
    ob: &'a mut BTreeSetObserver<'ob, T, S, D>,
    first_extracted: Option<T>,
    extracted_count: usize,
}

#[rustversion::since(1.91)]
impl<T, S: ?Sized, D, R, F> Drop for ExtractIf<'_, '_, T, S, D, R, F>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
{
    fn drop(&mut self) {
        unsafe { self.inner.assume_init_drop() }
        let Some(first) = &self.first_extracted else {
            return;
        };
        let set = (*self.ob.ptr).as_deref();
        self.ob.state.shrink_boundary(first, set);
        self.ob.state.truncate_len += self.extracted_count;
    }
}

#[rustversion::since(1.91)]
impl<T, S: ?Sized, D, R, F> Iterator for ExtractIf<'_, '_, T, S, D, R, F>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    R: RangeBounds<T>,
    F: FnMut(&T) -> bool,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = unsafe { self.inner.assume_init_mut() }.next()?;
        if let Some(boundary) = &mut self.ob.state.boundary
            && value <= *boundary
        {
            self.extracted_count += 1;
            if self.first_extracted.is_none() {
                self.first_extracted = Some(value.clone());
            }
        }
        Some(value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        unsafe { self.inner.assume_init_ref() }.size_hint()
    }
}

#[rustversion::since(1.91)]
impl<T, S: ?Sized, D, R, F> FusedIterator for ExtractIf<'_, '_, T, S, D, R, F>
where
    T: Clone + Ord,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    R: RangeBounds<T>,
    F: FnMut(&T) -> bool,
{
}

#[rustversion::since(1.91)]
impl<T, S: ?Sized, D, R, F> Debug for ExtractIf<'_, '_, T, S, D, R, F>
where
    T: Clone + Ord + Debug,
    D: Unsigned,
    S: AsDeref<D, Target = BTreeSet<T>>,
    R: RangeBounds<T>,
    F: FnMut(&T) -> bool,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe { self.inner.assume_init_ref() }.fmt(f)
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
        assert_eq!(ob.pop_last(), Some(2));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 2)));
    }

    #[test]
    fn retain_noop() {
        let mut set = BTreeSet::from([1, 2, 3, 4, 5]);
        let mut ob = set.__observe();
        ob.retain(|v| *v < 10);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn retain_truncate() {
        let mut set = BTreeSet::from([1, 2, 3, 4, 5]);
        let mut ob = set.__observe();
        ob.retain(|v| *v % 2 == 1);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 4), append!(_, json!([3, 5])))));
    }

    #[test]
    fn extract_if_drop() {
        let mut set = BTreeSet::from([1, 2, 3, 4, 5]);
        let mut ob = set.__observe();
        let mut iter = ob.extract_if(.., |v| *v % 2 == 0);
        assert_eq!(iter.next(), Some(2));
        drop(iter);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 4), append!(_, json!([3, 4, 5])))));
    }
}
