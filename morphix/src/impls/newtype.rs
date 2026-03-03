use std::cmp::Reverse;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::num::{Saturating, Wrapping};
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{Observer, SerializeObserver};
use crate::{Adapter, Mutations};

/// Observer implementation for transparent newtype wrappers such as
/// [`Wrapping<T>`], [`Saturating<T>`], and [`Reverse<T>`].
pub struct NewtypeObserver<O, S: ?Sized, D = Zero>(pub O, Pointer<S>, PhantomData<D>);

impl<O, S: ?Sized, D> Deref for NewtypeObserver<O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<O, S: ?Sized, D> DerefMut for NewtypeObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_deref_mut_coinductive();
        &mut self.1
    }
}

impl<O, S: ?Sized, D> QuasiObserver for NewtypeObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<O, S: ?Sized, D> Observer for NewtypeObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target: NewtypeInner<Inner = O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self(O::uninit(), Pointer::uninit(), PhantomData)
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let ptr = Pointer::new(head);
        let value = head.as_deref();
        Self(O::observe(value.as_inner()), ptr, PhantomData)
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(&this.1, head);
        let value = head.as_deref();
        unsafe { O::refresh(&mut this.0, value.as_inner()) }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for NewtypeObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target: NewtypeInner<Inner = O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        SerializeObserver::flush::<A>(&mut this.0)
    }
}

impl<O, S: ?Sized, D> Debug for NewtypeObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NewtypeObserver").field(&self.observed_ref()).finish()
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<NewtypeObserver<O2, S2, D2>> for NewtypeObserver<O1, S1, D1>
where
    O1: QuasiObserver<Target: Deref<Target: AsDeref<O1::InnerDepth>>>,
    O2: QuasiObserver<Target: Deref<Target: AsDeref<O2::InnerDepth>>>,
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &NewtypeObserver<O2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<O, S: ?Sized, D> Eq for NewtypeObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
{
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<NewtypeObserver<O2, S2, D2>> for NewtypeObserver<O1, S1, D1>
where
    O1: QuasiObserver<Target: Deref<Target: AsDeref<O1::InnerDepth>>>,
    O2: QuasiObserver<Target: Deref<Target: AsDeref<O2::InnerDepth>>>,
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &NewtypeObserver<O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<O, S: ?Sized, D> Ord for NewtypeObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &NewtypeObserver<O, S, D>) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
    }
}

/// Helper trait to access the inner field of a transparent newtype wrapper.
pub trait NewtypeInner {
    type Inner;

    fn as_inner(&self) -> &Self::Inner;
}

impl<T> NewtypeInner for Wrapping<T> {
    type Inner = T;

    #[inline]
    fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T> NewtypeInner for Saturating<T> {
    type Inner = T;

    #[inline]
    fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T> NewtypeInner for Reverse<T> {
    type Inner = T;

    #[inline]
    fn as_inner(&self) -> &T {
        &self.0
    }
}

macro_rules! impl_newtype {
    ($helper:ident, $helper_ref:ident, $wrapper:ident) => {
        spec_impl_observe!($helper, $wrapper<Self>, $wrapper<T>, NewtypeObserver);
        spec_impl_ref_observe!($helper_ref, $wrapper<Self>, $wrapper<T>, NewtypeObserver);

        impl<T: Snapshot> Snapshot for $wrapper<T> {
            type Snapshot = T::Snapshot;

            #[inline]
            fn to_snapshot(&self) -> Self::Snapshot {
                self.0.to_snapshot()
            }

            #[inline]
            fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
                self.0.eq_snapshot(snapshot)
            }
        }

        impl<O, S: ?Sized, D, U> PartialEq<$wrapper<U>> for NewtypeObserver<O, S, D>
        where
            O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: PartialEq<$wrapper<U>>,
        {
            #[inline]
            fn eq(&self, other: &$wrapper<U>) -> bool {
                self.observed_ref().eq(other)
            }
        }

        impl<O, S: ?Sized, D, U> PartialOrd<$wrapper<U>> for NewtypeObserver<O, S, D>
        where
            O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: PartialOrd<$wrapper<U>>,
        {
            #[inline]
            fn partial_cmp(&self, other: &$wrapper<U>) -> Option<std::cmp::Ordering> {
                self.observed_ref().partial_cmp(other)
            }
        }
    };
}

impl_newtype!(WrappingObserveImpl, WrappingRefObserveImpl, Wrapping);
impl_newtype!(SaturatingObserveImpl, SaturatingRefObserveImpl, Saturating);
impl_newtype!(ReverseObserveImpl, ReverseRefObserveImpl, Reverse);

#[cfg(test)]
mod tests {
    use std::cmp::Reverse;
    use std::num::{Saturating, Wrapping};

    use serde_json::json;

    use crate::MutationKind;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn wrapping_no_change() {
        let mut value = Wrapping(String::from("hello"));
        let mut ob = value.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn wrapping_replace() {
        let mut value = Wrapping(String::from("hello"));
        let mut ob = value.__observe();
        **ob = Wrapping(String::from("world"));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("world")));
    }

    #[test]
    fn saturating_no_change() {
        let mut value = Saturating(42u32);
        let mut ob = value.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn saturating_replace() {
        let mut value = Saturating(42u32);
        let mut ob = value.__observe();
        **ob = Saturating(100u32);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(100)));
    }

    #[test]
    fn reverse_no_change() {
        let mut value = Reverse(String::from("hello"));
        let mut ob = value.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn reverse_replace() {
        let mut value = Reverse(String::from("hello"));
        let mut ob = value.__observe();
        **ob = Reverse(String::from("world"));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("world")));
    }

    #[test]
    fn wrapping_granular_append() {
        let mut value = Wrapping(String::from("hello"));
        let mut ob = value.__observe();
        ob.0.push_str(" world");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
    }

    #[test]
    fn reverse_granular_append() {
        let mut value = Reverse(String::from("hello"));
        let mut ob = value.__observe();
        ob.0.push_str(" world");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
    }
}
