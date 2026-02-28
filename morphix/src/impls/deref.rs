use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::builtin::Snapshot;
use crate::helper::{AsDeref, AsDerefMut, QuasiObserver, Succ, Unsigned};
use crate::observe::{Observer, ObserverExt, RefObserve, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

/// Observer implementation for pointer types such as [`Box<T>`] and `&mut T`.
///
/// This observer wraps the inner type's observer and forwards all operations to it, maintaining
/// proper dereference chains for pointer types.
pub struct DerefObserver<O> {
    inner: O,
}

impl<O> Deref for DerefObserver<O> {
    type Target = O;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<O> DerefMut for DerefObserver<O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<O, D> QuasiObserver for DerefObserver<O>
where
    D: Unsigned,
    O: QuasiObserver<InnerDepth = Succ<D>>,
    O::Target: Deref<Target: AsDeref<D> + AsDeref<Succ<D>>>,
{
    type OuterDepth = Succ<O::OuterDepth>;
    type InnerDepth = D;
}

impl<O, D> Observer for DerefObserver<O>
where
    O: Observer<InnerDepth = Succ<D>>,
    O::Head: AsDeref<D>,
    D: Unsigned,
{
    #[inline]
    fn uninit() -> Self {
        Self { inner: O::uninit() }
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        Self {
            inner: O::observe(value),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        unsafe { O::refresh(&mut this.inner, value) }
    }
}

impl<O, D> SerializeObserver for DerefObserver<O>
where
    O: SerializeObserver<InnerDepth = Succ<D>>,
    O::Head: AsDeref<D>,
    D: Unsigned,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        unsafe { O::flush_unchecked::<A>(&mut this.inner) }
    }
}

macro_rules! impl_fmt {
    ($($trait:ident),* $(,)?) => {
        $(
            impl<O> std::fmt::$trait for DerefObserver<O>
            where
                O: std::fmt::$trait,
            {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::$trait::fmt(&self.inner, f)
                }
            }
        )*
    };
}

impl_fmt! {
    Binary,
    Display,
    LowerExp,
    LowerHex,
    Octal,
    Pointer,
    UpperExp,
    UpperHex,
}

impl<O> Debug for DerefObserver<O>
where
    O: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DerefObserver").field(&self.inner).finish()
    }
}

impl<O1, O2> PartialEq<DerefObserver<O2>> for DerefObserver<O1>
where
    O1: PartialEq<O2>,
{
    #[inline]
    fn eq(&self, other: &DerefObserver<O2>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<O> Eq for DerefObserver<O> where O: Eq {}

impl<O1, O2> PartialOrd<DerefObserver<O2>> for DerefObserver<O1>
where
    O1: PartialOrd<O2>,
{
    #[inline]
    fn partial_cmp(&self, other: &DerefObserver<O2>) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<O> Ord for DerefObserver<O>
where
    O: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

macro_rules! impl_deref_observe {
    ($(impl $([$($gen:tt)*])? Observe for $ty:ty $(where { $($where:tt)+ })?;)*) => {
        $(
            impl <$($($gen)*)?> Observe for $ty {
                type Observer<'ob, S, D>
                    = DerefObserver<T::Observer<'ob, S, Succ<D>>>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = T::Spec;
            }
        )*
    };
}

impl_deref_observe! {
    impl [T: Observe + ?Sized] Observe for Box<T>;
    impl [T: Observe + ?Sized] Observe for &mut T;
    impl [T: RefObserve + ?Sized] Observe for &T;
    impl [T: RefObserve + ?Sized] Observe for std::rc::Rc<T>;
    impl [T: RefObserve + ?Sized] Observe for std::sync::Arc<T>;
}

macro_rules! impl_deref_ref_observe {
    ($(impl $([$($gen:tt)*])? RefObserve for $ty:ty $(where { $($where:tt)+ })?;)*) => {
        $(
            impl <$($($gen)*)?> RefObserve for $ty {
                type Observer<'ob, S, D>
                    = DerefObserver<T::Observer<'ob, S, Succ<D>>>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDeref<D, Target = Self> + ?Sized + 'ob;

                type Spec = T::Spec;
            }
        )*
    };
}

impl_deref_ref_observe! {
    impl [T: RefObserve + ?Sized] RefObserve for &T;
    impl [T: RefObserve + ?Sized] RefObserve for &mut T;
    impl [T: RefObserve + ?Sized] RefObserve for Box<T>;
    impl [T: RefObserve + ?Sized] RefObserve for std::rc::Rc<T>;
    impl [T: RefObserve + ?Sized] RefObserve for std::sync::Arc<T>;
}

macro_rules! impl_snapshot {
    ($(impl $([$($gen:tt)*])? Snapshot for $ty:ty as $value:ty $(where { $($where:tt)+ })?;)*) => {
        $(
            impl <$($($gen)*)?> Snapshot for $ty {
                type Snapshot = $value;

                #[inline]
                fn to_snapshot(&self) -> Self::Snapshot {
                    (**self).to_snapshot()
                }

                #[inline]
                fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
                    (**self).eq_snapshot(snapshot)
                }
            }
        )*
    };
}

impl_snapshot! {
    impl [T: Snapshot + ?Sized] Snapshot for &T as T::Snapshot;
    impl [T: Snapshot + ?Sized] Snapshot for &mut T as T::Snapshot;
    impl [T: Snapshot + ?Sized] Snapshot for Box<T> as T::Snapshot;
    impl [T: Snapshot + ?Sized] Snapshot for std::rc::Rc<T> as T::Snapshot;
    impl [T: Snapshot + ?Sized] Snapshot for std::sync::Arc<T> as T::Snapshot;
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<$($($gen)*,)? O> PartialEq<$ty> for DerefObserver<O>
            where
                Self: ObserverExt<Target: PartialEq<$ty>>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.observed_ref().eq(other)
                }
            }

            impl<$($($gen)*,)? O> PartialOrd<$ty> for DerefObserver<O>
            where
                Self: ObserverExt<Target: PartialOrd<$ty>>,
            {
                #[inline]
                fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                    self.observed_ref().partial_cmp(other)
                }
            }
        )*
    };
}

generic_impl_cmp! {
    impl [U: ?Sized] _ for Box<U>;
    impl ['a, U: ?Sized] _ for &'a U;
    impl ['a, U: ?Sized] _ for &'a mut U;
    impl [U: ?Sized] _ for std::rc::Rc<U>;
    impl [U: ?Sized] _ for std::sync::Arc<U>;
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::MutationKind;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn test_deref_method() {
        let mut value = Box::new(String::from("Hello, World!"));
        let mut ob = value.__observe();
        assert_eq!(*ob, "Hello, World!");

        ob.push_str("\n");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("\n")));
    }

    #[test]
    fn test_deref_replace() {
        let mut value = Box::new(String::from("Hello, World!"));
        let mut ob = value.__observe();
        assert_eq!(*ob, "Hello, World!");

        ****ob = String::from("42");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("42")));
    }

    #[test]
    fn test_deref_assign() {
        let mut value = Box::new(String::from("Hello, World!"));
        let mut ob = value.__observe();
        assert_eq!(*ob, "Hello, World!");

        ****ob = String::from("42");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("42")));
    }
}
