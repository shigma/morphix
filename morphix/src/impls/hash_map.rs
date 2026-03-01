use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet, TryReserveError};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{default_impl_ref_observe, untracked_methods};
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe, PathSegment};

struct Diff<K> {
    replaced: HashSet<K>,
    deleted: HashSet<K>,
}

impl<K> Default for Diff<K> {
    #[inline]
    fn default() -> Self {
        Self {
            replaced: HashSet::new(),
            deleted: HashSet::new(),
        }
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
    diff: Option<Diff<K>>,
    /// Boxed to ensure pointer stability: [`HashMap`] rehashing moves all entries to a new
    /// allocation, which would invalidate references to inline values. [`Box`] adds a layer
    /// of indirection so that only the pointer is moved, not the observer itself.
    inner: UnsafeCell<HashMap<K, Box<O>>>,
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
        self.inner.get_mut().clear();
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
    S: AsDerefMut<D, Target = HashMap<K, O::Head>> + 'ob,
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
            diff: Some(Diff::default()),
            inner: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> SerializeObserver for HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashMap<K, O::Head>> + 'ob,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized,
    K: Serialize + Eq + Hash + Into<PathSegment>,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let Some(diff) = this.diff.take() else {
            return Ok(MutationKind::Replace(A::serialize_value((*this).observed_ref())?).into());
        };
        let mut mutations = Mutations::new();
        for key in diff.deleted {
            #[cfg(feature = "delete")]
            mutations.insert(key, MutationKind::Delete);
            #[cfg(not(feature = "delete"))]
            unreachable!("delete feature is not enabled");
        }
        for key in diff.replaced {
            let observer = this
                .inner
                .get_mut()
                .get_mut(&key)
                .expect("replaced key not found in inner observers")
                .as_mut();
            mutations.insert(key, unsafe { O::flush_unchecked::<A>(observer)? });
        }
        Ok(mutations)
    }
}

impl<'ob, K, O, S: ?Sized, D, V> HashMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashMap<K, V>> + 'ob,
    O: Observer<InnerDepth = Zero, Head = V> + 'ob,
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
    S: AsDerefMut<D, Target = HashMap<K, O::Head>> + 'ob,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
    K: Clone,
{
    /// See [`HashMap::clear`].
    #[inline]
    pub fn clear(&mut self) {
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
        diff.deleted.remove(&key);
        diff.replaced.insert(key);
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
        diff.replaced.remove::<K>(&key);
        diff.deleted.insert(key);
        Some(old_value)
    }

    /// See [`HashMap::get`].
    pub fn get<Q>(&self, key: &Q) -> Option<&O>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash + ?Sized,
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

    /// See [`HashMap::get_mut`].
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut O>
    where
        K: Borrow<Q> + Eq + Hash,
        Q: Eq + Hash + ?Sized,
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
    S: AsDerefMut<D, Target = HashMap<K, V>> + 'ob,
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
    S: AsDerefMut<D, Target = HashMap<K, V>> + 'ob,
    O: Observer<InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Clone + Eq + Hash,
    Q: Eq + Hash,
{
    #[inline]
    fn index_mut(&mut self, index: &'q Q) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
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
