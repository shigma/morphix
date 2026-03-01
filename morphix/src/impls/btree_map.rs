use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::fmt::Debug;
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
    inner: UnsafeCell<BTreeMap<K, Box<O>>>,
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
        self.inner.get_mut().clear();
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
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            diff: None,
            inner: Default::default(),
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
            inner: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> SerializeObserver for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
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
                    this.inner.get_mut().remove(&key);
                    let value = (*this.ptr)
                        .as_deref()
                        .get(&key)
                        .expect("replaced key not found in observed map");
                    mutations.insert(key, MutationKind::Replace(A::serialize_value(value)?));
                }
            }
        }
        for (key, mut observer) in std::mem::take(this.inner.get_mut()) {
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
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone,
{
    /// See [`BTreeMap::clear`].
    #[inline]
    pub fn clear(&mut self) {
        self.inner.get_mut().clear();
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
        self.inner.get_mut().remove(&key);
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
        let Some(diff) = self.diff.as_mut() else {
            return self.observed_mut().remove(key);
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().remove_entry(key)?;
        self.inner.get_mut().remove::<K>(&key);
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
        Some(old_value)
    }

    /// See [`BTreeMap::get`].
    pub fn get<Q>(&self, key: &Q) -> Option<&O>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let (key, value) = self.observed_ref().get_key_value(key)?;
        let key_cloned = key.clone();
        match unsafe { (*self.inner.get()).entry(key_cloned) } {
            Entry::Occupied(occupied) => {
                let observer = occupied.into_mut().as_mut();
                unsafe { O::refresh(observer, value) }
                Some(observer)
            }
            Entry::Vacant(vacant) => Some(vacant.insert(Box::new(O::observe(value)))),
        }
    }

    /// See [`BTreeMap::get_mut`].
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut O>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let (key, value) = (*self.ptr).as_deref().get_key_value(key)?;
        let key_cloned = key.clone();
        match self.inner.get_mut().entry(key_cloned) {
            Entry::Occupied(occupied) => {
                let observer = occupied.into_mut().as_mut();
                unsafe { O::refresh(observer, value) }
                Some(observer)
            }
            Entry::Vacant(vacant) => Some(vacant.insert(Box::new(O::observe(value)))),
        }
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
    S: AsDerefMut<D, Target = BTreeMap<K, V>> + 'ob,
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
    S: AsDerefMut<D, Target = BTreeMap<K, V>> + 'ob,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Clone + Ord,
    Q: Ord,
{
    #[inline]
    fn index_mut(&mut self, index: &'q Q) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
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
        // Create observer for key 0 in the inner BTreeMap
        assert_eq!(ob.get(&0).unwrap().observed_ref(), "value 0");
        // Create many more observers, triggering node splits in the inner BTreeMap.
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
        // First get_mut: modify the value through the inner observer
        ob.get_mut("0").unwrap().push_str(" world");
        assert_eq!(ob.observed_ref().get("0").unwrap(), "hello world");
        // Insert many keys via untracked_mut to trigger node splits in the
        // observed BTreeMap without adding to diff.replaced
        for i in 1..100 {
            ob.untracked_mut()
                .insert(Box::leak(i.to_string().into_boxed_str()), format!("value {i}"));
        }
        // Second get_mut: refresh updates the inner observer's stale pointer
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
}
