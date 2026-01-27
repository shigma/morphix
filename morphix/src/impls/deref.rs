use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::builtin::Snapshot;
use crate::helper::{AsDeref, AsDerefMut, AsNormalized, Succ, Unsigned};
use crate::observe::{Observer, RefObserve, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

/// Observer implementation for pointer types such as [`Box<T>`] and `&mut T`.
///
/// This observer wraps the inner type's observer and forwards all operations to it, maintaining
/// proper dereference chains for pointer types.
pub struct DerefObserver<'ob, O> {
    inner: O,
    phantom: PhantomData<&'ob mut ()>,
}

impl<'ob, O> Deref for DerefObserver<'ob, O> {
    type Target = O;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ob, O> DerefMut for DerefObserver<'ob, O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'ob, O> AsNormalized for DerefObserver<'ob, O>
where
    O: AsNormalized,
{
    type OuterDepth = Succ<O::OuterDepth>;
}

impl<'ob, O, D> Observer<'ob> for DerefObserver<'ob, O>
where
    O: Observer<'ob, InnerDepth = Succ<D>>,
    O::Head: AsDeref<D>,
    D: Unsigned,
{
    type InnerDepth = D;
    type Head = O::Head;

    #[inline]
    fn uninit() -> Self {
        Self {
            inner: O::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            inner: O::observe(value),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        unsafe { O::refresh(&mut this.inner, value) }
    }
}

impl<'ob, O, D> SerializeObserver<'ob> for DerefObserver<'ob, O>
where
    O: SerializeObserver<'ob, InnerDepth = Succ<D>>,
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
            impl<'ob, O> std::fmt::$trait for DerefObserver<'ob, O>
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

impl<'ob, O> Debug for DerefObserver<'ob, O>
where
    O: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DerefObserver").field(&self.inner).finish()
    }
}

impl<'ob, O, D, T: ?Sized, U: ?Sized> PartialEq<U> for DerefObserver<'ob, O>
where
    O: Observer<'ob, InnerDepth = Succ<D>>,
    O::Head: AsDeref<D, Target = T>,
    D: Unsigned,
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        AsDeref::<D>::as_deref(&**self.as_normalized_ref()).eq(other)
    }
}

impl<'ob, O, D, T: ?Sized, U: ?Sized> PartialOrd<U> for DerefObserver<'ob, O>
where
    O: Observer<'ob, InnerDepth = Succ<D>>,
    O::Head: AsDeref<D, Target = T>,
    D: Unsigned,
    T: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        AsDeref::<D>::as_deref(&**self.as_normalized_ref()).partial_cmp(other)
    }
}

macro_rules! impl_deref_observe {
    ($(impl $([$($gen:tt)*])? Observe for $ty:ty $(where { $($where:tt)+ })?;)*) => {
        $(
            impl <$($($gen)*)?> Observe for $ty {
                type Observer<'ob, S, D>
                    = DerefObserver<'ob, T::Observer<'ob, S, Succ<D>>>
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
                    = DerefObserver<'ob, T::Observer<'ob, S, Succ<D>>>
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
                type Value = $value;

                #[inline]
                fn to_snapshot(&self) -> Self::Value {
                    (**self).to_snapshot()
                }

                #[inline]
                fn cmp_snapshot(&self, snapshot: &Self::Value) -> bool {
                    (**self).cmp_snapshot(snapshot)
                }
            }
        )*
    };
}

impl_snapshot! {
    impl [T: Snapshot + ?Sized] Snapshot for &T as T::Value;
    impl [T: Snapshot + ?Sized] Snapshot for &mut T as T::Value;
    impl [T: Snapshot + ?Sized] Snapshot for Box<T> as T::Value;
    impl [T: Snapshot + ?Sized] Snapshot for std::rc::Rc<T> as T::Value;
    impl [T: Snapshot + ?Sized] Snapshot for std::sync::Arc<T> as T::Value;
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
