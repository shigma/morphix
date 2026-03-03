use std::borrow::Cow;
use std::fmt::Debug;
use std::ops::{AddAssign, Deref, DerefMut};

use crate::builtin::Snapshot;
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::{DerefObserver, StringObserver};
use crate::observe::{DefaultSpec, Observer, ObserverExt, RefObserve, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

/// Observer implementation for [`Cow<'a, T>`].
pub struct CowObserver<B, O> {
    inner: B,
    owned: Option<O>,
    mutated: bool,
}

impl<B, O> Deref for CowObserver<B, O> {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<B, O> DerefMut for CowObserver<B, O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.owned = None;
        self.mutated = true;
        &mut self.inner
    }
}

impl<B, O, D> QuasiObserver for CowObserver<B, O>
where
    D: Unsigned,
    B: QuasiObserver<InnerDepth = Succ<D>>,
    B::Target: Deref<Target: AsDeref<D> + AsDeref<Succ<D>>>,
{
    type OuterDepth = Succ<B::OuterDepth>;
    type InnerDepth = D;
}

impl<'a, B, O, D, T> Observer for CowObserver<B, O>
where
    B: Observer<InnerDepth = Succ<D>>,
    B::Head: AsDeref<D, Target = Cow<'a, T>>,
    O: Observer<InnerDepth = Zero, Head = T::Owned>,
    D: Unsigned,
    T: ToOwned + ?Sized + 'a,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            inner: B::uninit(),
            owned: None,
            mutated: false,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let inner = B::observe(head);
        let owned = match AsDeref::<D>::as_deref(head) {
            Cow::Borrowed(_) => None,
            Cow::Owned(value) => Some(O::observe(value)),
        };
        Self {
            inner,
            owned,
            mutated: false,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        unsafe { B::refresh(&mut this.inner, head) }
        if let Some(owned) = &mut this.owned {
            match AsDeref::<D>::as_deref(head) {
                Cow::Borrowed(_) => panic!("inconsistent state for CowObserver"),
                Cow::Owned(value) => unsafe { O::refresh(owned, value) },
            }
        }
    }
}

impl<'a, B, O, D, T> SerializeObserver for CowObserver<B, O>
where
    D: Unsigned,
    B: SerializeObserver<InnerDepth = Succ<D>>,
    B::Head: AsDeref<D, Target = Cow<'a, T>>,
    O: SerializeObserver<InnerDepth = Zero, Head = T::Owned>,
    T: ToOwned + ?Sized + 'a,
{
    unsafe fn flush<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        if let Some(mut owned) = this.owned.take()
            && !this.mutated
        {
            let head = &**this.inner.as_deref_coinductive();
            this.inner = B::observe(head);
            unsafe { O::flush::<A>(&mut owned) }
        } else {
            unsafe { B::flush::<A>(&mut this.inner) }
        }
    }
}

impl<'a, B, O, T, D> CowObserver<B, O>
where
    D: Unsigned,
    B: Observer<InnerDepth = Succ<D>>,
    B::Head: AsDerefMut<D, Target = Cow<'a, T>>,
    O: Observer<InnerDepth = Zero, Head = T::Owned>,
    T: ToOwned + ?Sized + 'a,
{
    /// See [`Cow::to_mut`].
    #[inline]
    pub fn to_mut(&mut self) -> &mut O {
        let head = unsafe { Pointer::as_mut(self.inner.as_deref_coinductive()) };
        let cow = AsDerefMut::<D>::as_deref_mut(head);
        let owned = cow.to_mut();
        self.owned.get_or_insert_with(|| O::observe(owned))
    }
}

impl<B, O> Debug for CowObserver<B, O>
where
    B: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CowObserver").field(&self.inner).finish()
    }
}

impl<B1, B2, O1, O2> PartialEq<CowObserver<B2, O2>> for CowObserver<B1, O1>
where
    B1: PartialEq<B2>,
{
    #[inline]
    fn eq(&self, other: &CowObserver<B2, O2>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<B, O> Eq for CowObserver<B, O> where B: Eq {}

impl<B1, B2, O1, O2> PartialOrd<CowObserver<B2, O2>> for CowObserver<B1, O1>
where
    B1: PartialOrd<B2>,
{
    #[inline]
    fn partial_cmp(&self, other: &CowObserver<B2, O2>) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<B, O> Ord for CowObserver<B, O>
where
    B: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<'ob, 'a, B, D, R> AddAssign<R> for CowObserver<B, StringObserver<'ob, String, Zero>>
where
    D: Unsigned,
    B: Observer<InnerDepth = Succ<D>>,
    B::Head: AsDerefMut<D, Target = Cow<'a, str>>,
    Cow<'a, str>: AddAssign<R>,
    R: Deref<Target = str> + Into<Cow<'a, str>>,
{
    #[inline]
    fn add_assign(&mut self, rhs: R) {
        let head = unsafe { Pointer::as_mut(self.inner.as_deref_coinductive()) };
        let cow = AsDerefMut::<D>::as_deref_mut(head);
        if cow.is_empty() {
            self.mutated = true;
            *cow = rhs.into();
        } else if !rhs.is_empty() {
            if let Cow::Borrowed(lhs) = cow {
                let mut s = String::with_capacity(lhs.len() + rhs.len());
                s.push_str(lhs);
                *cow = Cow::Owned(s);
            }
            self.to_mut().push_str(&rhs);
        }
    }
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<$($($gen)*,)? B, O> PartialEq<$ty> for CowObserver<B, O>
            where
                Self: ObserverExt<Target: PartialEq<$ty>>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.observed_ref().eq(other)
                }
            }

            impl<$($($gen)*,)? B, O> PartialOrd<$ty> for CowObserver<B, O>
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
    impl _ for str;
    impl _ for String;
    impl _ for std::ffi::CStr;
    impl _ for std::ffi::CString;
    impl _ for std::ffi::OsStr;
    impl _ for std::ffi::OsString;
    impl _ for std::path::Path;
    impl _ for std::path::PathBuf;
    impl [U] _ for Vec<U>;
    impl ['b, U: ?Sized] _ for &'b U;
    impl ['b, U: ?Sized] _ for &'b mut U;
    impl ['b, U: ToOwned + ?Sized] _ for Cow<'b, U>;
}

impl<'a, T> Observe for Cow<'a, T>
where
    T: ToOwned + RefObserve + ?Sized + 'a,
    T::Owned: Observe,
{
    type Observer<'ob, S, D>
        = CowObserver<T::Observer<'ob, S, Succ<D>>, <T::Owned as Observe>::Observer<'ob, T::Owned, Zero>>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl<'a, T> RefObserve for Cow<'a, T>
where
    T: RefObserve + ToOwned + ?Sized + 'a,
{
    type Observer<'ob, S, D>
        = DerefObserver<T::Observer<'ob, S, Succ<D>>>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = T::Spec;
}

impl<'a, T> Snapshot for Cow<'a, T>
where
    T: Snapshot + ToOwned + ?Sized,
{
    type Snapshot = T::Snapshot;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        (**self).to_snapshot()
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        (**self).eq_snapshot(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};
    use crate::{Mutation, MutationKind};

    #[test]
    fn no_change_returns_none() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn replace_via_deref_mut() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ***ob = Cow::Owned(String::from("world"));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("world")));
    }

    #[test]
    fn unsize_append() {
        const S: &str = "hello world";
        let mut cow = Cow::Borrowed(&S[..5]);
        let mut ob = cow.__observe();
        ***ob = Cow::Borrowed(S);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
    }

    #[test]
    fn unsize_truncate() {
        const S: &str = "hello world";
        let mut cow = Cow::Borrowed(S);
        let mut ob = cow.__observe();
        ***ob = Cow::Borrowed(&S[..5]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Truncate(6));
    }

    #[test]
    fn to_mut_no_change() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ob.to_mut();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn to_mut_granular_tracking() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ob.to_mut().push_str(" world");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn replace_after_to_mut() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ob.to_mut().push_str(" world");
        ***ob = Cow::Borrowed("replaced");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("replaced")));
    }

    #[test]
    fn to_mut_after_replace() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ***ob = Cow::Borrowed("replaced");
        ob.to_mut().push_str(" world");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("replaced world")));
    }

    #[test]
    fn owned_cow_no_change() {
        let mut cow: Cow<'_, str> = Cow::Owned(String::from("hello"));
        let mut ob = cow.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn owned_cow_replace() {
        let mut cow: Cow<'_, str> = Cow::Owned(String::from("hello"));
        let mut ob = cow.__observe();
        ***ob = Cow::Owned(String::from("world"));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("world")));
    }

    #[test]
    fn comparison_with_cow() {
        let mut cow = Cow::Borrowed("hello");
        let ob = cow.__observe();
        assert_eq!(ob, Cow::Borrowed("hello"));
        assert_eq!(ob, Cow::<str>::Owned(String::from("hello")));
    }

    #[test]
    fn to_mut_truncate_then_append() {
        let mut cow: Cow<'_, str> = Cow::Owned(String::from("hello world"));
        let mut ob = cow.__observe();
        let s = ob.to_mut();
        s.truncate(5);
        s.push_str("!");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation.unwrap().kind,
            MutationKind::Batch(vec![
                Mutation {
                    path: Default::default(),
                    kind: MutationKind::Truncate(6),
                },
                Mutation {
                    path: Default::default(),
                    kind: MutationKind::Append(json!("!")),
                },
            ])
        );
    }

    #[test]
    fn add_assign_borrowed() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ob += " world";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
    }

    #[test]
    fn add_assign_empty_lhs() {
        let mut cow = Cow::Borrowed("");
        let mut ob = cow.__observe();
        ob += "hello";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("hello")));
    }

    #[test]
    fn add_assign_empty_rhs() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ob += "";
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn add_assign_owned() {
        let mut cow: Cow<'_, str> = Cow::Owned(String::from("hello"));
        let mut ob = cow.__observe();
        ob += " world";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
    }

    #[test]
    fn add_assign_multiple() {
        let mut cow = Cow::Borrowed("a");
        let mut ob = cow.__observe();
        ob += "b";
        ob += "c";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("bc")));
    }
}
