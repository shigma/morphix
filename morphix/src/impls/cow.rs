use std::borrow::Cow;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::builtin::Snapshot;
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::DerefObserver;
use crate::observe::{DefaultSpec, Observer, ObserverExt, RefObserve, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

/// Observer implementation for [`Cow<'a, T>`].
pub struct CowObserver<B, O> {
    borrowed: B,
    owned: Option<O>,
}

impl<B, O> Deref for CowObserver<B, O> {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.borrowed
    }
}

impl<B, O> DerefMut for CowObserver<B, O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.owned = None;
        &mut self.borrowed
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
            borrowed: B::uninit(),
            owned: None,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let borrowed = B::observe(head);
        let owned = match AsDeref::<D>::as_deref(head) {
            Cow::Borrowed(_) => None,
            Cow::Owned(value) => Some(O::observe(value)),
        };
        Self { borrowed, owned }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        unsafe { B::refresh(&mut this.borrowed, head) }
        let value = AsDeref::<D>::as_deref(head);
        if let Some(owned) = &mut this.owned {
            match value {
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
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        if let Some(mut owned) = this.owned.take() {
            let head = &**this.borrowed.as_deref_coinductive();
            this.borrowed = B::observe(head);
            unsafe { O::flush_unchecked::<A>(&mut owned) }
        } else {
            unsafe { B::flush_unchecked::<A>(&mut this.borrowed) }
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
        let head = unsafe { Pointer::as_mut(self.borrowed.as_deref_coinductive()) };
        let cow: &mut Cow<'a, T> = AsDerefMut::<D>::as_deref_mut(head);
        let owned: &T::Owned = cow.to_mut();
        match &mut self.owned {
            Some(ob) => unsafe { Observer::force(ob, owned) },
            None => self.owned = Some(O::observe(owned)),
        }
        self.owned.as_mut().unwrap()
    }
}

impl<B, O> Debug for CowObserver<B, O>
where
    B: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("CowObserver").field(&self.borrowed).finish()
    }
}

impl<B1, B2, O1, O2> PartialEq<CowObserver<B2, O2>> for CowObserver<B1, O1>
where
    B1: PartialEq<B2>,
{
    #[inline]
    fn eq(&self, other: &CowObserver<B2, O2>) -> bool {
        self.borrowed.eq(&other.borrowed)
    }
}

impl<B, O> Eq for CowObserver<B, O> where B: Eq {}

impl<B1, B2, O1, O2> PartialOrd<CowObserver<B2, O2>> for CowObserver<B1, O1>
where
    B1: PartialOrd<B2>,
{
    #[inline]
    fn partial_cmp(&self, other: &CowObserver<B2, O2>) -> Option<std::cmp::Ordering> {
        self.borrowed.partial_cmp(&other.borrowed)
    }
}

impl<B, O> Ord for CowObserver<B, O>
where
    B: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.borrowed.cmp(&other.borrowed)
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
    fn to_mut_granular_tracking() {
        let mut cow = Cow::Borrowed("hello");
        let mut ob = cow.__observe();
        ob.to_mut().push_str(" world");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!(" world")));
    }

    #[test]
    fn to_mut_no_change() {
        let mut cow: Cow<'_, str> = Cow::Owned(String::from("hello"));
        let mut ob = cow.__observe();
        let _ = ob.to_mut(); // access but don't modify
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn to_mut_then_flush_then_no_change() {
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
        assert_eq!(ob, Cow::<str>::Borrowed("hello"));
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
}
