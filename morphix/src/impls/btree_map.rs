use std::borrow::Borrow;
use std::cell::UnsafeCell;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr::NonNull;

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::default_impl_ref_observe;
use crate::helper::{AsDerefMut, AsNormalized, Pointer, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe, PathSegment};

struct Diff<K> {
    replaced: BTreeSet<K>,
    deleted: BTreeSet<K>,
}

impl<K> Default for Diff<K> {
    #[inline]
    fn default() -> Self {
        Self {
            replaced: BTreeSet::new(),
            deleted: BTreeSet::new(),
        }
    }
}

/// Observer implementation for [`BTreeMap<K, V>`].
pub struct BTreeMapObserver<'ob, K, O, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    diff: Option<Diff<&'ob K>>,
    inner: UnsafeCell<BTreeMap<&'ob K, O>>,
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

impl<'ob, K, O, S: ?Sized, D> AsNormalized for BTreeMapObserver<'ob, K, O, S, D> {
    type OuterDepth = Succ<Zero>;
}

impl<'ob, K, O, S: ?Sized, D> Observer<'ob> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = D;
    type Head = S;

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
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        Pointer::set(this, value);
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            diff: Some(Diff::default()),
            inner: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> SerializeObserver<'ob> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
    O: SerializeObserver<'ob, InnerDepth = Zero>,
    O::Head: Serialize + Sized,
    K: Serialize + Ord,
    &'ob K: Into<PathSegment>,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let Some(diff) = this.diff.take() else {
            return Ok(MutationKind::Replace(A::serialize_value(this.as_deref())?).into());
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
                .expect("replaced key not found in inner observers");
            mutations.insert(key, unsafe { O::flush_unchecked::<A>(observer)? });
        }
        Ok(mutations)
    }
}

impl<'ob, K, O, S: ?Sized, D> BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
    O::Head: Sized,
    K: 'ob,
{
    /// See [`BTreeMap::get`].
    pub fn get<Q>(&self, key: &Q) -> Option<&O>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let inner = Observer::as_inner(self);
        let key_ptr = NonNull::from_ref(inner.get_key_value(key)?.0);
        let value = inner.get_mut(key)?;
        // SAFETY: key_ptr is valid as it comes from inner.get_key_value
        match unsafe { (*self.inner.get()).entry(key_ptr.as_ref()) } {
            Entry::Occupied(occupied) => {
                let observer = occupied.into_mut();
                unsafe { O::refresh(observer, value) }
                Some(observer)
            }
            Entry::Vacant(vacant) => Some(vacant.insert(O::observe(value))),
        }
    }

    /// See [`BTreeMap::get_mut`].
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut O>
    where
        K: Borrow<Q> + Ord,
        Q: Ord + ?Sized,
    {
        let inner = Observer::as_inner(self);
        let key_ptr = NonNull::from_ref(inner.get_key_value(key)?.0);
        let value = inner.get_mut(key)?;
        // SAFETY: key_ptr is valid as it comes from inner.get_key_value
        match self.inner.get_mut().entry(unsafe { key_ptr.as_ref() }) {
            Entry::Occupied(occupied) => {
                let observer = occupied.into_mut();
                unsafe { O::refresh(observer, value) }
                Some(observer)
            }
            Entry::Vacant(vacant) => Some(vacant.insert(O::observe(value))),
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> Debug for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BTreeMapObserver").field(&self.as_deref()).finish()
    }
}

impl<'ob, K, O, S: ?Sized, D, V> PartialEq<BTreeMap<K, V>> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: PartialEq<BTreeMap<K, V>>,
{
    #[inline]
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'ob, K1, K2, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<BTreeMapObserver<'ob, K2, O2, S2, D2>>
    for BTreeMapObserver<'ob, K1, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDerefMut<D1>,
    S2: AsDerefMut<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &BTreeMapObserver<'ob, K2, O2, S2, D2>) -> bool {
        self.as_deref().eq(other.as_deref())
    }
}

impl<'ob, K, O, S: ?Sized, D> Eq for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Eq,
{
}

impl<'ob, K, O, S: ?Sized, D, V> PartialOrd<BTreeMap<K, V>> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: PartialOrd<BTreeMap<K, V>>,
{
    #[inline]
    fn partial_cmp(&self, other: &BTreeMap<K, V>) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<'ob, K1, K2, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<BTreeMapObserver<'ob, K2, O2, S2, D2>>
    for BTreeMapObserver<'ob, K1, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDerefMut<D1>,
    S2: AsDerefMut<D2>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &BTreeMapObserver<'ob, K2, O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other.as_deref())
    }
}

impl<'ob, K, O, S: ?Sized, D> Ord for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_deref().cmp(other.as_deref())
    }
}

impl<'ob, 'q, K, O, S: ?Sized, D, V, Q: ?Sized> Index<&'q Q> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, V>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Ord,
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
    O: Observer<'ob, InnerDepth = Zero, Head = V>,
    K: Borrow<Q> + Ord,
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
