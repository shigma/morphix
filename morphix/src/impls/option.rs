use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{Observer, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations};

/// Observer implementation for [`Option<T>`].
///
/// This observer tracks changes to optional values, including transitions between [`Some`] and
/// [`None`] states. It provides specialized methods for working with options while maintaining
/// change tracking.
pub struct OptionObserver<O, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    mutated: bool,
    initial: bool,
    inner: Option<O>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> Deref for OptionObserver<O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<O, S: ?Sized, D> DerefMut for OptionObserver<O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        self.inner = None;
        &mut self.ptr
    }
}

impl<O, S: ?Sized, D> QuasiObserver for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: crate::helper::AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<O, S: ?Sized, D> Observer for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            mutated: false,
            initial: false,
            inner: None,
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        Pointer::set(this, value);
        match (&mut this.inner, value.as_deref()) {
            (Some(inner), Some(value)) => unsafe { Observer::refresh(inner, value) },
            (None, _) => {}
            _ => unreachable!("inconsistent option observer state"),
        }
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            mutated: false,
            initial: value.as_deref().is_some(),
            inner: value.as_deref().as_ref().map(O::observe),
            phantom: PhantomData,
        }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let value = this.ptr.as_deref();
        let initial = this.initial;
        this.initial = value.is_some();
        if !this.mutated {
            if let Some(ob) = &mut this.inner {
                return SerializeObserver::flush::<A>(ob);
            } else {
                return Ok(Mutations::new());
            }
        }
        this.mutated = false;
        if initial || value.is_some() {
            Ok(MutationKind::Replace(A::serialize_value(value)?).into())
        } else {
            Ok(Mutations::new())
        }
    }
}

impl<O, S: ?Sized, D> OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn __insert(&mut self, value: O::Head) {
        self.mutated = true;
        let inserted = Observer::as_inner(self).insert(value);
        self.inner = Some(O::observe(inserted));
    }

    /// See [`Option::as_mut`].
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut O> {
        if self.inner.is_none() {
            self.inner = Observer::as_inner(self).as_ref().map(O::observe);
        }
        self.inner.as_mut()
    }

    /// See [`Option::insert`].
    #[inline]
    pub fn insert(&mut self, value: O::Head) -> &mut O {
        self.__insert(value);
        // SAFETY: `__insert` ensures that `self.inner` is `Some`.
        self.inner.as_mut().unwrap()
    }

    /// See [`Option::get_or_insert`].
    #[inline]
    pub fn get_or_insert(&mut self, value: O::Head) -> &mut O {
        self.get_or_insert_with(|| value)
    }

    /// See [`Option::get_or_insert_default`].
    #[inline]
    pub fn get_or_insert_default(&mut self) -> &mut O
    where
        O::Head: Default,
    {
        self.get_or_insert_with(Default::default)
    }

    /// See [`Option::get_or_insert_with`].
    #[inline]
    pub fn get_or_insert_with<F>(&mut self, f: F) -> &mut O
    where
        F: FnOnce() -> O::Head,
    {
        if self.as_deref().is_none() {
            self.__insert(f());
        }
        // SAFETY: We just ensured that value is `Some`.
        self.as_mut().unwrap()
    }
}

impl<O, S: ?Sized, D> Debug for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OptionObserver").field(&self.as_deref()).finish()
    }
}

impl<O, S: ?Sized, D, U> PartialEq<Option<U>> for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: PartialEq<Option<U>>,
{
    #[inline]
    fn eq(&self, other: &Option<U>) -> bool {
        self.as_deref().eq(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<OptionObserver<O2, S2, D2>> for OptionObserver<O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDerefMut<D1>,
    S2: AsDerefMut<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &OptionObserver<O2, S2, D2>) -> bool {
        self.as_deref().eq(other.as_deref())
    }
}

impl<O, S: ?Sized, D> Eq for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Eq,
{
}

impl<O, S: ?Sized, D, U> PartialOrd<Option<U>> for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: PartialOrd<Option<U>>,
{
    #[inline]
    fn partial_cmp(&self, other: &Option<U>) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<OptionObserver<O2, S2, D2>> for OptionObserver<O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDerefMut<D1>,
    S2: AsDerefMut<D2>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &OptionObserver<O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other.as_deref())
    }
}

impl<O, S: ?Sized, D> Ord for OptionObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &OptionObserver<O, S, D>) -> std::cmp::Ordering {
        self.as_deref().cmp(other.as_deref())
    }
}

spec_impl_observe!(OptionObserveImpl, Option<Self>, Option<T>, OptionObserver);
spec_impl_ref_observe!(OptionRefObserveImpl, Option<Self>, Option<T>);

impl<T: Snapshot> Snapshot for Option<T> {
    type Snapshot = Option<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.as_ref().map(|v| v.to_snapshot())
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        match (self, snapshot) {
            (Some(v), Some(snapshot)) => v.eq_snapshot(snapshot),
            (None, None) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::builtin::GeneralObserver;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_change_returns_none() {
        let mut opt: Option<i32> = None;
        let mut ob = opt.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());

        let mut opt: Option<i32> = Some(1);
        let mut ob = opt.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn deref_triggers_replace() {
        let mut opt: Option<i32> = Some(42);
        let mut ob = opt.__observe();
        **ob = None;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(null)));

        let mut opt: Option<i32> = None;
        let mut ob = opt.__observe();
        **ob = Some(42);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(42)));

        let mut opt: Option<i32> = None;
        let mut ob = opt.__observe();
        **ob = Some(42);
        **ob = None;
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());

        let mut opt: Option<&str> = Some("42");
        let mut ob = opt.__observe();
        **ob = None;
        **ob = Some("42");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("42")));
    }

    #[test]
    fn insert_returns_observer() {
        let mut opt: Option<String> = None;
        let mut ob = opt.__observe();
        let s = ob.insert(String::from("99"));
        assert_eq!(format!("{s:?}"), r#"StringObserver("99")"#);
        *s += "9";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("999")));
    }

    #[test]
    fn as_mut_tracks_inner() {
        let mut opt = Some(String::from("foo"));
        let mut ob = opt.__observe();
        *ob.as_mut().unwrap() += "bar";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn get_or_insert() {
        // get_or_insert
        let mut opt: Option<i32> = None;
        let mut ob = opt.__observe();
        *ob.get_or_insert(5) = 6;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(6)));

        // get_or_insert_default
        let mut opt: Option<i32> = None;
        let mut ob = opt.__observe();
        *ob.get_or_insert_default() = 77;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(77)));

        // get_or_insert_with
        let mut opt: Option<i32> = None;
        let mut ob = opt.__observe();
        *ob.get_or_insert_with(|| 88) = 99;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(99)));
    }

    #[test]
    fn specialization() {
        let mut opt: Option<i32> = Some(0i32);
        let ob: GeneralObserver<_, _, _> = opt.__observe();
        assert_eq!(format!("{ob:?}"), r#"SnapshotObserver(Some(0))"#);

        let mut opt: Option<&str> = Some("");
        let ob: OptionObserver<_, _, _> = opt.__observe();
        assert_eq!(format!("{ob:?}"), r#"OptionObserver(Some(""))"#);
    }

    #[test]
    fn ref_specialization() {
        let mut opt = &Some(0i32);
        let ob = opt.__observe();
        assert_eq!(format!("{ob:?}"), r#"DerefObserver(SnapshotObserver(Some(0)))"#);

        let mut opt = &Some("");
        let ob = opt.__observe();
        assert_eq!(format!("{ob:?}"), r#"DerefObserver(PointerObserver(Some("")))"#);
    }

    #[test]
    fn refresh() {
        let mut vec = vec![None::<i32>];
        let mut ob = vec.__observe();
        **ob[0] = Some(1);
        ob.reserve(10); // force reallocation
        assert_eq!(**ob[0], Some(1));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(1)));
    }
}
