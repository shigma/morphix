use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::default_impl_ref_observe;
use crate::helper::{AsDerefMut, AsNormalized, Pointer, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, RefObserve, SerializeObserver};
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

pub struct BTreeMapObserver<'ob, K, O, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    diff: Option<Diff<K>>,
    inner: BTreeMap<K, O>,
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
        self.inner.clear();
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
            inner: BTreeMap::new(),
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
            inner: BTreeMap::new(),
            phantom: PhantomData,
        }
    }
}

impl<'ob, K, O, S: ?Sized, D> SerializeObserver<'ob> for BTreeMapObserver<'ob, K, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BTreeMap<K, O::Head>> + 'ob,
    K: Serialize + Ord + Into<PathSegment>,
    O: SerializeObserver<'ob, InnerDepth = Zero>,
    O::Head: Serialize + Sized,
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
                .get_mut(&key)
                .expect("replaced key not found in inner observers");
            mutations.insert(key, unsafe { O::flush_unchecked::<A>(observer)? });
        }
        Ok(mutations)
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
    impl [K, V: RefObserve] RefObserve for BTreeMap<K, V>;
}

impl<K, V> Snapshot for BTreeMap<K, V>
where
    K: Snapshot,
    K::Value: Ord,
    V: Snapshot,
{
    type Value = BTreeMap<K::Value, V::Value>;

    #[inline]
    fn to_snapshot(&self) -> Self::Value {
        self.iter()
            .map(|(key, value)| (key.to_snapshot(), value.to_snapshot()))
            .collect()
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Value) -> bool {
        self.len() == snapshot.len()
            && self
                .iter()
                .zip(snapshot.iter())
                .all(|((key_a, value_a), (key_b, value_b))| key_a.eq_snapshot(key_b) && value_a.eq_snapshot(value_b))
    }
}
