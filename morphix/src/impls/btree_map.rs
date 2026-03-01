//! Observer implementation for [`BTreeMap<K, V>`].

use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::default_impl_ref_observe;
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe, PathSegment};

enum ValueState {
    /// Key existed in the original map and was overwritten via
    /// [`insert`](BTreeMapObserver::insert).
    Replaced,
    /// Key is new (did not exist in the original map), added via
    /// [`insert`](BTreeMapObserver::insert).
    Inserted,
    /// Key existed in the original map and was removed.
    Deleted,
}

fn mark_deleted<K, O>(diff: &mut BTreeMap<K, ValueState>, children: &mut BTreeMap<K, Box<O>>, key: K)
where
    K: Ord,
{
    children.remove(&key);
    match diff.entry(key) {
        Entry::Occupied(mut e) => {
            if matches!(e.get(), ValueState::Inserted) {
                e.remove();
            } else {
                e.insert(ValueState::Deleted);
            }
        }
        Entry::Vacant(e) => {
            e.insert(ValueState::Deleted);
        }
    }
}

/// Iterator produced by [`BTreeMapObserver::extract_if`].
#[allow(clippy::type_complexity)]
pub struct ExtractIf<'a, K, V, O, R, F>
where
    R: RangeBounds<K>,
    F: FnMut(&K, &mut V) -> bool,
{
    inner: std::collections::btree_map::ExtractIf<'a, K, V, R, F>,
    diff_children: Option<(&'a mut BTreeMap<K, ValueState>, &'a mut BTreeMap<K, Box<O>>)>,
}

impl<K, V, O, R, F> Iterator for ExtractIf<'_, K, V, O, R, F>
where
    K: Clone + Ord,
    R: RangeBounds<K>,
    F: FnMut(&K, &mut V) -> bool,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = self.inner.next()?;
        if let Some((diff, children)) = &mut self.diff_children {
            mark_deleted(diff, children, key.clone());
        }
        Some((key, value))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V, O, R, F> FusedIterator for ExtractIf<'_, K, V, O, R, F>
where
    K: Clone + Ord,
    R: RangeBounds<K>,
    F: FnMut(&K, &mut V) -> bool,
{
}

impl<K, V, O, R, F> Debug for ExtractIf<'_, K, V, O, R, F>
where
    K: Debug,
    V: Debug,
    R: RangeBounds<K>,
    F: FnMut(&K, &mut V) -> bool,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// Observer implementation for [`BTreeMap<K, V>`].
///
/// ## Limitations
///
/// Most methods (e.g. [`insert`](Self::insert), [`remove`](Self::remove),
/// [`get_mut`](Self::get_mut)) require `K: Clone` because the observer maintains its own
/// [`BTreeMap`] of cloned keys to track per-key observers independently of the observed map's
/// internal storage.
pub struct BTreeMapObserver<'ob, K, O, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    diff: Option<BTreeMap<K, ValueState>>,
    /// Boxed to ensure pointer stability: [`BTreeMap`] node splits move entries between nodes
    /// via `memcpy`, which would invalidate references to inline values. [`Box`] adds a layer
    /// of indirection so that only the pointer is moved, not the observer itself.
    children: UnsafeCell<BTreeMap<K, Box<O>>>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, K, O, S: ?Sized, D> Deref for BTreeMapObserver<'ob, K, O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, K, O, S: ?Sized, D> DerefMut for BTreeMapObserver<'ob, K, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.diff = None;
        self.children.get_mut().clear();
        &mut self.ptr
    }
}

impl<'ob, K, O, S: ?Sized, D> QuasiObserver for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<'ob, K, O, S: ?Sized, D> Observer for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = BTreeMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            diff: None,
            children: Default::default(),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        Pointer::set(this, value);
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            diff: Some(Default::default()),
            children: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> SerializeObserver for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = BTreeMap<K, O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized,
    K: Serialize + Ord + Into<PathSegment>,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let Some(diff) = this.diff.take() else {
            return Ok(MutationKind::Replace(A::serialize_value((*this).observed_ref())?).into());
        };
        let mut mutations = Mutations::new();
        for (key, state) in diff {
            match state {
                ValueState::Deleted => {
                    #[cfg(feature = "delete")]
                    mutations.insert(key, MutationKind::Delete);
                    #[cfg(not(feature = "delete"))]
                    unreachable!("delete feature is not enabled");
                }
                ValueState::Replaced | ValueState::Inserted => {
                    this.children.get_mut().remove(&key);
                    let value = (*this.ptr)
                        .as_deref()
                        .get(&key)
                        .expect("replaced key not found in observed map");
                    mutations.insert(key, MutationKind::Replace(A::serialize_value(value)?));
                }
            }
        }
        for (key, mut observer) in std::mem::take(this.children.get_mut()) {
            let value = (*this.ptr)
                .as_deref()
                .get(&key)
                .expect("observer key not found in observed map");
            unsafe { O::refresh(&mut observer, value) }
            mutations.insert(key, unsafe { O::flush_unchecked::<A>(&mut observer)? });
        }
        Ok(mutations)
    }
}

impl<'ob, K, O, S: ?Sized, D> BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = BTreeMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone,
{
    /// See [`BTreeMap::get`].
    pub fn get<Q>(&self, key: &Q) -> Option<&O>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let (key, value) = self.observed_ref().get_key_value(key)?;
        let key_cloned = key.clone();
        match unsafe { (*self.children.get()).entry(key_cloned) } {
            Entry::Occupied(occupied) => {
                let observer = occupied.into_mut().as_mut();
                unsafe { O::refresh(observer, value) }
                Some(observer)
            }
            Entry::Vacant(vacant) => Some(vacant.insert(Box::new(O::observe(value)))),
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone,
{
    fn __force_all(&mut self) -> &mut BTreeMap<K, Box<O>>
    where
        K: Ord,
    {
        let map = (*self.ptr).as_deref();
        let children = self.children.get_mut();
        for (key, value) in map.iter() {
            match children.entry(key.clone()) {
                Entry::Occupied(occupied) => {
                    let observer = occupied.into_mut().as_mut();
                    unsafe { O::refresh(observer, value) }
                }
                Entry::Vacant(vacant) => {
                    vacant.insert(Box::new(O::observe(value)));
                }
            }
        }
        children
    }

    /// See [`BTreeMap::get_mut`].
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut O>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let (key, value) = (*self.ptr).as_deref().get_key_value(key)?;
        let key_cloned = key.clone();
        match self.children.get_mut().entry(key_cloned) {
            Entry::Occupied(occupied) => {
                let observer = occupied.into_mut().as_mut();
                unsafe { O::refresh(observer, value) }
                Some(observer)
            }
            Entry::Vacant(vacant) => Some(vacant.insert(Box::new(O::observe(value)))),
        }
    }

    /// See [`BTreeMap::clear`].
    #[inline]
    pub fn clear(&mut self) {
        self.children.get_mut().clear();
        if (*self).observed_ref().is_empty() {
            self.untracked_mut().clear()
        } else {
            self.observed_mut().clear()
        }
    }

    /// See [`BTreeMap::insert`].
    pub fn insert(&mut self, key: K, value: O::Head) -> Option<O::Head>
    where
        K: Ord,
    {
        let Some(diff) = self.diff.as_mut() else {
            return self.observed_mut().insert(key, value);
        };
        let key_cloned = key.clone();
        let old_value = (*self.ptr).as_deref_mut().insert(key_cloned, value);
        self.children.get_mut().remove(&key);
        match diff.entry(key) {
            Entry::Occupied(mut e) => {
                if matches!(e.get(), ValueState::Deleted) {
                    e.insert(ValueState::Replaced);
                }
            }
            Entry::Vacant(e) => {
                if old_value.is_some() {
                    e.insert(ValueState::Replaced);
                } else {
                    e.insert(ValueState::Inserted);
                }
            }
        }
        old_value
    }

    /// See [`BTreeMap::remove`].
    pub fn remove<Q>(&mut self, key: &Q) -> Option<O::Head>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let Some(diff) = &mut self.diff else {
            return self.observed_mut().remove(key);
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().remove_entry(key)?;
        mark_deleted(diff, self.children.get_mut(), key);
        Some(old_value)
    }

    /// See [`BTreeMap::remove_entry`].
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, O::Head)>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let Some(diff) = &mut self.diff else {
            return self.observed_mut().remove_entry(key);
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().remove_entry(key)?;
        mark_deleted(diff, self.children.get_mut(), key.clone());
        Some((key, old_value))
    }

    /// See [`BTreeMap::pop_first`].
    pub fn pop_first(&mut self) -> Option<(K, O::Head)>
    where
        K: Ord,
    {
        let Some(diff) = &mut self.diff else {
            return self.observed_mut().pop_first();
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().pop_first()?;
        mark_deleted(diff, self.children.get_mut(), key.clone());
        Some((key, old_value))
    }

    /// See [`BTreeMap::pop_last`].
    pub fn pop_last(&mut self) -> Option<(K, O::Head)>
    where
        K: Ord,
    {
        let Some(diff) = &mut self.diff else {
            return self.observed_mut().pop_last();
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().pop_last()?;
        mark_deleted(diff, self.children.get_mut(), key.clone());
        Some((key, old_value))
    }

    /// See [`BTreeMap::retain`].
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        K: Ord,
        F: FnMut(&K, &mut O::Head) -> bool,
    {
        self.extract_if(.., |k, v| !f(k, v)).for_each(drop);
    }

    /// See [`BTreeMap::append`].
    // TODO: this drains `other` into individual inserts, which is much slower than
    // `BTreeMap::append`. Consider a bulk-insert approach that updates `diff` in one pass.
    pub fn append(&mut self, other: &mut BTreeMap<K, O::Head>)
    where
        K: Ord,
    {
        if self.diff.is_none() {
            return self.observed_mut().append(other);
        }
        for (key, value) in std::mem::take(other) {
            self.insert(key, value);
        }
    }

    /// See [`BTreeMap::split_off`].
    pub fn split_off<Q>(&mut self, key: &Q) -> BTreeMap<K, O::Head>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let Some(diff) = &mut self.diff else {
            return self.observed_mut().split_off(key);
        };
        let split = (*self.ptr).as_deref_mut().split_off(key);
        let children = self.children.get_mut();
        for key in split.keys().cloned() {
            mark_deleted(diff, children, key);
        }
        split
    }

    /// See [`BTreeMap::extract_if`].
    pub fn extract_if<F, R>(&mut self, range: R, pred: F) -> ExtractIf<'_, K, O::Head, O, R, F>
    where
        K: Ord,
        R: RangeBounds<K>,
        F: FnMut(&K, &mut O::Head) -> bool,
    {
        let inner = (*self.ptr).as_deref_mut().extract_if(range, pred);
        let diff_children = match &mut self.diff {
            Some(diff) => Some((diff, self.children.get_mut())),
            None => None,
        };
        ExtractIf { inner, diff_children }
    }

    /// See [`BTreeMap::iter_mut`].
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut O)> + '_
    where
        K: Ord,
    {
        self.__force_all().iter_mut().map(|(k, v)| (k, v.as_mut()))
    }

    /// See [`BTreeMap::values_mut`].
    #[inline]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut O> + '_
    where
        K: Ord,
    {
        self.__force_all().values_mut().map(|v| v.as_mut())
    }
}

impl<'ob, K, O, S: ?Sized, D> Debug for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BTreeMapObserver").field(&self.observed_ref()).finish()
    }
}

impl<'ob, K, O, S: ?Sized, D, V> PartialEq<BTreeMap<K, V>> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialEq<BTreeMap<K, V>>,
{
    #[inline]
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        self.observed_ref().eq(other)
    }
}

impl<'ob, K1, K2, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<BTreeMapObserver<'ob, K2, O2, S2, D2>>
    for BTreeMapObserver<'ob, K1, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &BTreeMapObserver<'ob, K2, O2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<'ob, K, O, S: ?Sized, D> Eq for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
{
}

impl<'ob, K, O, S: ?Sized, D, V> PartialOrd<BTreeMap<K, V>> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialOrd<BTreeMap<K, V>>,
{
    #[inline]
    fn partial_cmp(&self, other: &BTreeMap<K, V>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other)
    }
}

impl<'ob, K1, K2, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<BTreeMapObserver<'ob, K2, O2, S2, D2>>
    for BTreeMapObserver<'ob, K1, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &BTreeMapObserver<'ob, K2, O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<'ob, K, O, S: ?Sized, D> Ord for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
    }
}

impl<'ob, 'q, K, O, S: ?Sized, D, V, Q: ?Sized> Index<&'q Q> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = BTreeMap<K, V>>,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Clone + Ord,
    Q: Ord,
{
    type Output = O;

    #[inline]
    fn index(&self, index: &'q Q) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

impl<'ob, 'q, K, O, S: ?Sized, D, V, Q: ?Sized> IndexMut<&'q Q> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, V>>,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Clone + Ord,
    Q: Ord,
{
    #[inline]
    fn index_mut(&mut self, index: &'q Q) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
    }
}

// TODO: this inserts elements one by one, which is much slower than `BTreeMap::extend`.
// Consider a bulk-insert approach that updates `diff` in one pass.
impl<'ob, K, O, S: ?Sized, D> Extend<(K, O::Head)> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone + Ord,
{
    fn extend<I: IntoIterator<Item = (K, O::Head)>>(&mut self, iter: I) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl<K, V: Observe> Observe for BTreeMap<K, V> {
    type Observer<'ob, S, D>
        = BTreeMapObserver<'ob, K, V::Observer<'ob, V, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [K, V] RefObserve for BTreeMap<K, V>;
}

impl<K, V> Snapshot for BTreeMap<K, V>
where
    K: Snapshot,
    K::Snapshot: Ord,
    V: Snapshot,
{
    type Snapshot = BTreeMap<K::Snapshot, V::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.iter()
            .map(|(key, value)| (key.to_snapshot(), value.to_snapshot()))
            .collect()
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.len() == snapshot.len()
            && self
                .iter()
                .zip(snapshot.iter())
                .all(|((key_a, value_a), (key_b, value_b))| key_a.eq_snapshot(key_b) && value_a.eq_snapshot(value_b))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};
    use crate::{Mutation, MutationKind};

    #[test]
    fn pointer_stability_across_inner_splits() {
        let mut map = BTreeMap::new();
        for i in 0..100 {
            map.insert(i, format!("value {i}"));
        }
        let ob = map.__observe();
        // Create observer for key 0
        assert_eq!(ob.get(&0).unwrap().observed_ref(), "value 0");
        // Create many more observers, triggering node splits
        // Box<O> ensures previously created observers remain valid.
        for i in 1..100 {
            assert_eq!(ob.get(&i).unwrap().observed_ref(), &format!("value {i}"));
        }
        // Key 0's observer is still valid thanks to Box pointer stability
        assert_eq!(ob.get(&0).unwrap().observed_ref(), "value 0");
    }

    #[test]
    fn remove_nonexistent_key() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        assert_eq!(ob.remove("nonexistent"), None);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn insert_then_remove() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        assert_eq!(ob.insert("b", "y".to_string()), None);
        assert_eq!(ob.remove("b"), Some("y".to_string()));
        assert_eq!(ob.observed_ref().len(), 1);
        assert_eq!(ob.observed_ref().get("a"), Some(&"x".to_string()));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn remove_then_insert() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        assert_eq!(ob.remove("a"), Some("x".to_string()));
        assert_eq!(ob.insert("a", "y".to_string()), None);
        assert_eq!(ob.observed_ref().get("a"), Some(&"y".to_string()));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Replace(json!("y")),
            })
        );
    }

    #[test]
    fn get_mut_refresh_across_splits() {
        let mut map = BTreeMap::new();
        map.insert("0", "hello".to_string());
        let mut ob = map.__observe();
        // First get_mut: modify the value through the child observer
        ob.get_mut("0").unwrap().push_str(" world");
        assert_eq!(ob.observed_ref().get("0").unwrap(), "hello world");
        // Insert many keys via untracked_mut to trigger node splits in the
        // observed BTreeMap without adding to diff.replaced
        for i in 1..100 {
            ob.untracked_mut()
                .insert(Box::leak(i.to_string().into_boxed_str()), format!("value {i}"));
        }
        // Second get_mut: refresh updates the child observer's stale pointer
        ob.get_mut("0").unwrap().push_str("!");
        assert_eq!(ob.observed_ref().get("0").unwrap(), "hello world!");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["0".into()].into(),
                kind: MutationKind::Append(json!(" world!")),
            })
        );
    }

    #[test]
    fn insert_then_get_mut() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        ob.insert("b", "hello".to_string());
        ob.get_mut("b").unwrap().push_str(" world");
        assert_eq!(ob.observed_ref().get("b"), Some(&"hello world".to_string()));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["b".into()].into(),
                kind: MutationKind::Replace(json!("hello world")),
            })
        );
    }

    #[test]
    fn get_mut_then_insert() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        ob.get_mut("a").unwrap().push_str(" world");
        ob.insert("a", "bye".to_string());
        assert_eq!(ob.observed_ref().get("a"), Some(&"bye".to_string()));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Replace(json!("bye")),
            })
        );
    }

    #[test]
    fn remove_entry() {
        let mut map = BTreeMap::from([("a", "x".to_string()), ("b", "y".to_string())]);
        let mut ob = map.__observe();
        assert_eq!(ob.remove_entry("a"), Some(("a", "x".to_string())));
        assert_eq!(ob.observed_ref().len(), 1);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["a".into()].into(),
                kind: MutationKind::Delete,
            })
        );
    }

    #[test]
    fn pop_first_and_last() {
        let mut map = BTreeMap::from([("a", 1i32), ("b", 2), ("c", 3)]);
        let mut ob = map.__observe();
        assert_eq!(ob.pop_first(), Some(("a", 1)));
        assert_eq!(ob.pop_last(), Some(("c", 3)));
        assert_eq!(ob.observed_ref(), &BTreeMap::from([("b", 2)]));
        let Json(mutation) = ob.flush().unwrap();
        // Two deletions: "a" and "c"
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
    }

    #[test]
    fn retain() {
        let mut map = BTreeMap::from([("a", 1i32), ("b", 2), ("c", 3)]);
        let mut ob = map.__observe();
        ob.retain(|_, v| *v % 2 != 0);
        assert_eq!(ob.observed_ref(), &BTreeMap::from([("a", 1), ("c", 3)]));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["b".into()].into(),
                kind: MutationKind::Delete,
            })
        );
    }

    #[test]
    fn append_from_other() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        let mut other = BTreeMap::from([("b", "y".to_string()), ("c", "z".to_string())]);
        ob.append(&mut other);
        assert!(other.is_empty());
        assert_eq!(ob.observed_ref().len(), 3);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
    }

    #[test]
    fn split_off() {
        let mut map = BTreeMap::from([("a", 1i32), ("b", 2), ("c", 3)]);
        let mut ob = map.__observe();
        let split = ob.split_off("b");
        assert_eq!(split, BTreeMap::from([("b", 2), ("c", 3)]));
        assert_eq!(ob.observed_ref(), &BTreeMap::from([("a", 1)]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
    }

    #[test]
    fn extend() {
        let mut map = BTreeMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        ob.extend([("b", "y".to_string()), ("c", "z".to_string())]);
        assert_eq!(ob.observed_ref().len(), 3);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
    }

    #[test]
    fn extract_if() {
        let mut map = BTreeMap::from([("a", 1i32), ("b", 2), ("c", 3), ("d", 4)]);
        let mut ob = map.__observe();
        let extracted: BTreeMap<_, _> = ob.extract_if(.., |_, v| *v % 2 == 0).collect();
        assert_eq!(extracted, BTreeMap::from([("b", 2), ("d", 4)]));
        assert_eq!(ob.observed_ref(), &BTreeMap::from([("a", 1), ("c", 3)]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
    }

    #[test]
    fn extract_if_partial_drain() {
        let mut map = BTreeMap::from([("a", 1i32), ("b", 2), ("c", 3), ("d", 4)]);
        let mut ob = map.__observe();
        // Only take the first matching element, then drop the iterator.
        let first = ob.extract_if(.., |_, v| *v % 2 == 0).next();
        assert_eq!(first, Some(("b", 2)));
        // "d" matched the predicate but was never yielded, so it must be retained.
        assert_eq!(ob.observed_ref(), &BTreeMap::from([("a", 1), ("c", 3), ("d", 4)]));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["b".into()].into(),
                kind: MutationKind::Delete,
            })
        );
    }

    #[test]
    fn extract_if_insert_then_extract() {
        let mut map = BTreeMap::from([("a", 1i32)]);
        let mut ob = map.__observe();
        ob.insert("b", 2);
        // extract "b" which was just inserted: net no-op
        let extracted: BTreeMap<_, _> = ob.extract_if(.., |k, _| *k == "b").collect();
        assert_eq!(extracted, BTreeMap::from([("b", 2)]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn iter_mut() {
        let mut map = BTreeMap::from([("a", "x".to_string()), ("b", "y".to_string())]);
        let mut ob = map.__observe();
        for (_, v) in ob.iter_mut() {
            v.push_str("!");
        }
        assert_eq!(ob.observed_ref().get("a"), Some(&"x!".to_string()));
        assert_eq!(ob.observed_ref().get("b"), Some(&"y!".to_string()));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
        if let MutationKind::Batch(batch) = mutation.kind {
            assert_eq!(batch.len(), 2);
            assert_eq!(batch[0].kind, MutationKind::Append(json!("!")));
            assert_eq!(batch[1].kind, MutationKind::Append(json!("!")));
        }
    }

    #[test]
    fn values_mut() {
        let mut map = BTreeMap::from([("a", "hello".to_string()), ("b", "world".to_string())]);
        let mut ob = map.__observe();
        for v in ob.values_mut() {
            v.push('~');
        }
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
        if let MutationKind::Batch(batch) = mutation.kind {
            assert_eq!(batch.len(), 2);
            assert_eq!(batch[0].kind, MutationKind::Append(json!("~")));
            assert_eq!(batch[1].kind, MutationKind::Append(json!("~")));
        }
    }

    #[test]
    fn insert_then_pop() {
        let mut map: BTreeMap<&str, i32> = BTreeMap::new();
        let mut ob = map.__observe();
        ob.insert("a", 1);
        ob.insert("b", 2);
        assert_eq!(ob.pop_first(), Some(("a", 1)));
        // "a" was inserted then popped: net no-op
        // "b" was inserted: Inserted
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec!["b".into()].into(),
                kind: MutationKind::Replace(json!(2)),
            })
        );
    }
}
