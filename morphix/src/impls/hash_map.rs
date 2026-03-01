//! Observer implementation for [`HashMap<K, V>`].

use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, TryReserveError};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{default_impl_ref_observe, untracked_methods};
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe, PathSegment};

enum ValueState {
    /// Key existed in the original map and was overwritten via [`insert`](HashMapObserver::insert).
    Replaced,
    /// Key is new (did not exist in the original map), added via
    /// [`insert`](HashMapObserver::insert).
    Inserted,
    /// Key existed in the original map and was removed.
    Deleted,
}

fn mark_deleted<K, O>(diff: &mut HashMap<K, ValueState>, children: &mut HashMap<K, Box<O>>, key: K)
where
    K: Eq + Hash,
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

/// Iterator produced by [`HashMapObserver::extract_if`].
#[allow(clippy::type_complexity)]
pub struct ExtractIf<'a, K, V, O, F>
where
    F: FnMut(&K, &mut V) -> bool,
{
    inner: std::collections::hash_map::ExtractIf<'a, K, V, F>,
    diff_children: Option<(&'a mut HashMap<K, ValueState>, &'a mut HashMap<K, Box<O>>)>,
}

impl<K, V, O, F> Iterator for ExtractIf<'_, K, V, O, F>
where
    K: Clone + Eq + Hash,
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

impl<K, V, O, F> FusedIterator for ExtractIf<'_, K, V, O, F>
where
    K: Clone + Eq + Hash,
    F: FnMut(&K, &mut V) -> bool,
{
}

impl<K, V, O, F> Debug for ExtractIf<'_, K, V, O, F>
where
    K: Debug,
    V: Debug,
    F: FnMut(&K, &mut V) -> bool,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// Observer implementation for [`HashMap<K, V>`].
///
/// ## Limitations
///
/// Most methods (e.g. [`insert`](Self::insert), [`remove`](Self::remove),
/// [`get_mut`](Self::get_mut)) require `K: Clone` because the observer maintains its own
/// [`HashMap`] of cloned keys to track per-key observers independently of the observed map's
/// internal storage.
pub struct HashMapObserver<'ob, K, O, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    diff: Option<HashMap<K, ValueState>>,
    /// Boxed to ensure pointer stability: [`HashMap`] rehashing moves all entries to a new
    /// allocation, which would invalidate references to inline values. [`Box`] adds a layer
    /// of indirection so that only the pointer is moved, not the observer itself.
    children: UnsafeCell<HashMap<K, Box<O>>>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, K, O, S: ?Sized, D> Deref for HashMapObserver<'ob, K, O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, K, O, S: ?Sized, D> DerefMut for HashMapObserver<'ob, K, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.diff = None;
        self.children.get_mut().clear();
        &mut self.ptr
    }
}

impl<'ob, K, O, S: ?Sized, D> QuasiObserver for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<'ob, K, O, S: ?Sized, D> Observer for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = HashMap<K, O::Head>>,
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

impl<'ob, K, O, S: ?Sized, D> SerializeObserver for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = HashMap<K, O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized,
    K: Serialize + Eq + Hash + Into<PathSegment>,
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

impl<'ob, K, O, S: ?Sized, D, V> HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashMap<K, V>>,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Eq + Hash,
    V: 'ob,
{
    untracked_methods! { HashMap =>
        pub fn reserve(&mut self, additional: usize);
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn shrink_to_fit(&mut self);
        pub fn shrink_to(&mut self, min_capacity: usize);
    }
}

impl<'ob, K, O, S: ?Sized, D> HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = HashMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone,
{
    /// See [`HashMap::get`].
    pub fn get<Q>(&self, key: &Q) -> Option<&O>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash + ?Sized,
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

impl<'ob, K, O, S: ?Sized, D> HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone,
{
    fn __force_all(&mut self) -> &mut HashMap<K, Box<O>>
    where
        K: Eq + Hash,
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

    /// See [`HashMap::get_mut`].
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut O>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash + ?Sized,
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

    /// See [`HashMap::clear`].
    #[inline]
    pub fn clear(&mut self) {
        self.children.get_mut().clear();
        if (*self).observed_ref().is_empty() {
            self.untracked_mut().clear()
        } else {
            self.observed_mut().clear()
        }
    }

    /// See [`HashMap::insert`].
    pub fn insert(&mut self, key: K, value: O::Head) -> Option<O::Head>
    where
        K: Eq + Hash,
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

    /// See [`HashMap::remove`].
    pub fn remove<Q>(&mut self, key: &Q) -> Option<O::Head>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash + ?Sized,
    {
        let Some(diff) = self.diff.as_mut() else {
            return self.observed_mut().remove(key);
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().remove_entry(key)?;
        mark_deleted(diff, self.children.get_mut(), key);
        Some(old_value)
    }

    /// See [`HashMap::remove_entry`].
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, O::Head)>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash + ?Sized,
    {
        let Some(diff) = &mut self.diff else {
            return self.observed_mut().remove_entry(key);
        };
        let (key, old_value) = (*self.ptr).as_deref_mut().remove_entry(key)?;
        mark_deleted(diff, self.children.get_mut(), key.clone());
        Some((key, old_value))
    }

    /// See [`HashMap::retain`].
    #[inline]
    pub fn retain<F>(&mut self, mut f: F)
    where
        K: Eq + Hash,
        F: FnMut(&K, &mut O::Head) -> bool,
    {
        self.extract_if(|k, v| !f(k, v)).for_each(drop);
    }

    /// See [`HashMap::extract_if`].
    pub fn extract_if<F>(&mut self, pred: F) -> ExtractIf<'_, K, O::Head, O, F>
    where
        K: Eq + Hash,
        F: FnMut(&K, &mut O::Head) -> bool,
    {
        let inner = (*self.ptr).as_deref_mut().extract_if(pred);
        let diff_children = match &mut self.diff {
            Some(diff) => Some((diff, self.children.get_mut())),
            None => None,
        };
        ExtractIf { inner, diff_children }
    }

    /// See [`HashMap::iter_mut`].
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut O)> + '_
    where
        K: Eq + Hash,
    {
        self.__force_all().iter_mut().map(|(k, v)| (k, v.as_mut()))
    }

    /// See [`HashMap::values_mut`].
    #[inline]
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut O> + '_
    where
        K: Eq + Hash,
    {
        self.__force_all().values_mut().map(|v| v.as_mut())
    }
}

impl<'ob, K, O, S: ?Sized, D> Debug for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("HashMapObserver").field(&self.observed_ref()).finish()
    }
}

impl<'ob, K, O, S: ?Sized, D, V> PartialEq<HashMap<K, V>> for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialEq<HashMap<K, V>>,
{
    #[inline]
    fn eq(&self, other: &HashMap<K, V>) -> bool {
        self.observed_ref().eq(other)
    }
}

impl<'ob, K1, K2, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<HashMapObserver<'ob, K2, O2, S2, D2>>
    for HashMapObserver<'ob, K1, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &HashMapObserver<'ob, K2, O2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<'ob, K, O, S: ?Sized, D> Eq for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
{
}

impl<'ob, 'q, K, O, S: ?Sized, D, V, Q: ?Sized> Index<&'q Q> for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = HashMap<K, V>>,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Clone + Eq + Hash,
    Q: Eq + Hash,
{
    type Output = O;

    #[inline]
    fn index(&self, index: &'q Q) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

impl<'ob, 'q, K, O, S: ?Sized, D, V, Q: ?Sized> IndexMut<&'q Q> for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashMap<K, V>>,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Clone + Eq + Hash,
    Q: Eq + Hash,
{
    #[inline]
    fn index_mut(&mut self, index: &'q Q) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
    }
}

// TODO: this inserts elements one by one, which is much slower than `HashMap::extend`.
// Consider a bulk-insert approach that updates `diff` in one pass.
impl<'ob, K, O, S: ?Sized, D> Extend<(K, O::Head)> for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashMap<K, O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone + Eq + Hash,
{
    fn extend<I: IntoIterator<Item = (K, O::Head)>>(&mut self, iter: I) {
        for (key, value) in iter {
            self.insert(key, value);
        }
    }
}

impl<K, V: Observe> Observe for HashMap<K, V> {
    type Observer<'ob, S, D>
        = HashMapObserver<'ob, K, V::Observer<'ob, V, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [K, V] RefObserve for HashMap<K, V>;
}

impl<K, V> Snapshot for HashMap<K, V>
where
    K: Snapshot,
    K::Snapshot: Eq + Hash,
    V: Snapshot,
{
    type Snapshot = HashMap<K::Snapshot, V::Snapshot>;

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
    use std::collections::HashMap;

    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};
    use crate::{Mutation, MutationKind};

    #[test]
    fn remove_nonexistent_key() {
        let mut map = HashMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        assert_eq!(ob.remove("nonexistent"), None);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn insert_then_remove() {
        let mut map = HashMap::from([("a", "x".to_string())]);
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
        let mut map = HashMap::from([("a", "x".to_string())]);
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
    fn remove_entry() {
        let mut map = HashMap::from([("a", "x".to_string()), ("b", "y".to_string())]);
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
    fn retain() {
        let mut map = HashMap::from([("a", 1i32), ("b", 2), ("c", 3)]);
        let mut ob = map.__observe();
        ob.retain(|_, v| *v % 2 != 0);
        assert_eq!(ob.observed_ref(), &HashMap::from([("a", 1), ("c", 3)]));
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
    fn extend() {
        let mut map = HashMap::from([("a", "x".to_string())]);
        let mut ob = map.__observe();
        ob.extend([("b", "y".to_string()), ("c", "z".to_string())]);
        assert_eq!(ob.observed_ref().len(), 3);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
    }

    #[test]
    fn extract_if() {
        let mut map = HashMap::from([("a", 1i32), ("b", 2), ("c", 3), ("d", 4)]);
        let mut ob = map.__observe();
        let extracted: HashMap<_, _> = ob.extract_if(|_, v| *v % 2 == 0).collect();
        assert_eq!(extracted, HashMap::from([("b", 2), ("d", 4)]));
        assert_eq!(ob.observed_ref(), &HashMap::from([("a", 1), ("c", 3)]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        let mutation = mutation.unwrap();
        assert!(matches!(mutation.kind, MutationKind::Batch(_)));
    }

    #[test]
    fn extract_if_insert_then_extract() {
        let mut map = HashMap::from([("a", 1i32)]);
        let mut ob = map.__observe();
        ob.insert("b", 2);
        // extract "b" which was just inserted: net no-op
        let extracted: HashMap<_, _> = ob.extract_if(|k, _| *k == "b").collect();
        assert_eq!(extracted, HashMap::from([("b", 2)]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn get_mut_then_insert() {
        let mut map = HashMap::from([("a", "x".to_string())]);
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
    fn insert_then_get_mut() {
        let mut map = HashMap::from([("a", "x".to_string())]);
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
    fn iter_mut() {
        let mut map = HashMap::from([("a", "x".to_string()), ("b", "y".to_string())]);
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
            for m in &batch {
                assert_eq!(m.kind, MutationKind::Append(json!("!")));
            }
        }
    }

    #[test]
    fn values_mut() {
        let mut map = HashMap::from([("a", "hello".to_string()), ("b", "world".to_string())]);
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
            for m in &batch {
                assert_eq!(m.kind, MutationKind::Append(json!("~")));
            }
        }
    }
}
