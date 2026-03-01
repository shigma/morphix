use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::{PointerObserver, Snapshot};
use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, RefObserve, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

/// Observer implementation for tuple `(T,)`.
pub struct TupleObserver<O, S: ?Sized, D = Zero>(pub O, Pointer<S>, PhantomData<D>);

impl<O, S: ?Sized, D> Deref for TupleObserver<O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}

impl<O, S: ?Sized, D> DerefMut for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_deref_mut_coinductive();
        &mut self.1
    }
}

impl<O, S: ?Sized, D> QuasiObserver for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<O, S: ?Sized, D> Observer for TupleObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = (O::Head,)>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self(O::uninit(), Pointer::uninit(), PhantomData)
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        let ptr = Pointer::new(value);
        let value = value.as_deref();
        Self(O::observe(&value.0), ptr, PhantomData)
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        Pointer::set(&this.1, value);
        let value = value.as_deref();
        unsafe { O::refresh(&mut this.0, &value.0) }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for TupleObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = (O::Head,)>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let mutations_0 = SerializeObserver::flush::<A>(&mut this.0)?;
        let mut mutations = Mutations::new();
        mutations.insert(0, mutations_0);
        Ok(mutations)
    }
}

impl<O, S: ?Sized, D> Debug for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("TupleObserver").field(&self.observed_ref()).finish()
    }
}

impl<O, S: ?Sized, D, U> PartialEq<(U,)> for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialEq<(U,)>,
{
    #[inline]
    fn eq(&self, other: &(U,)) -> bool {
        self.observed_ref().eq(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<TupleObserver<O2, S2, D2>> for TupleObserver<O1, S1, D1>
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
    fn eq(&self, other: &TupleObserver<O2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<O, S: ?Sized, D> Eq for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
{
}

impl<O, S: ?Sized, D, U> PartialOrd<(U,)> for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialOrd<(U,)>,
{
    #[inline]
    fn partial_cmp(&self, other: &(U,)) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<TupleObserver<O2, S2, D2>> for TupleObserver<O1, S1, D1>
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
    fn partial_cmp(&self, other: &TupleObserver<O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<O, S: ?Sized, D> Ord for TupleObserver<O, S, D>
where
    O: QuasiObserver<Target: Deref<Target: AsDeref<O::InnerDepth>>>,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &TupleObserver<O, S, D>) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
    }
}

spec_impl_observe! {
    #[cfg_attr(docsrs, doc(fake_variadic))]
    #[cfg_attr(docsrs, doc = "This trait is implemented for tuples up to 12 items long.")]
    TupleObserveImpl, (Self,), (T,), TupleObserver
}

spec_impl_ref_observe! {
    #[cfg_attr(docsrs, doc(fake_variadic))]
    #[cfg_attr(docsrs, doc = "This trait is implemented for tuples up to 12 items long.")]
    TupleRefObserveImpl, (Self,), (T,)
}

#[cfg_attr(docsrs, doc(fake_variadic))]
#[cfg_attr(docsrs, doc = "This trait is implemented for tuples up to 12 items long.")]
impl<T: Snapshot> Snapshot for (T,) {
    type Snapshot = (T::Snapshot,);

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        (self.0.to_snapshot(),)
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.0.eq_snapshot(&snapshot.0)
    }
}

macro_rules! tuple_observer {
    ($ty:ident; $ptr:tt; $($o:ident, $p:ident, $t:ident, $u:ident, $n:tt);*) => {
        #[doc = concat!("Observer implementation for tuple `(", $(stringify!($t), ", ",)* ")`.")]
        pub struct $ty<$($o,)* S: ?Sized, D = Zero>(
            $(pub $o,)*
            /* ptr */ Pointer<S>,
            /* phantom */ PhantomData<D>,
        );

        impl<$($o,)* S: ?Sized, D> Deref for $ty<$($o,)* S, D> {
            type Target = Pointer<S>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.$ptr
            }
        }

        impl<$($o,)* S: ?Sized, D> DerefMut for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
        {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                $(self.$n.as_deref_mut_coinductive();)*
                &mut self.$ptr
            }
        }

        impl<$($o,)* S: ?Sized, D> QuasiObserver for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            D: Unsigned,
            S: AsDeref<D>,
        {
            type OuterDepth = Succ<Zero>;
            type InnerDepth = D;
        }

        impl<$($o,)* S: ?Sized, D> Observer for $ty<$($o,)* S, D>
        where
            D: Unsigned,
            S: AsDerefMut<D, Target = ($($o::Head,)*)>,
            $($o: Observer<InnerDepth = Zero, Head: Sized>,)*
        {
            #[inline]
            fn uninit() -> Self {
                Self(
                    $($o::uninit(),)*
                    /* ptr */ Pointer::uninit(),
                    /* phantom */ PhantomData,
                )
            }

            #[inline]
            fn observe(value: &Self::Head) -> Self {
                let ptr = Pointer::new(value);
                let value = value.as_deref();
                Self(
                    $($o::observe(&value.$n),)*
                    /* ptr */ ptr,
                    /* phantom */ PhantomData,
                )
            }

            #[inline]
            unsafe fn refresh(this: &mut Self, value: &Self::Head) {
                Pointer::set(&this.$ptr, value);
                let value = value.as_deref();
                unsafe {
                    $($o::refresh(&mut this.$n, &value.$n);)*
                }
            }
        }

        impl<$($o,)* S: ?Sized, D> SerializeObserver for $ty<$($o,)* S, D>
        where
            D: Unsigned,
            S: AsDerefMut<D, Target = ($($o::Head,)*)>,
            $($o: SerializeObserver<InnerDepth = Zero, Head: Serialize + Sized>,)*
        {
            #[inline]
            unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
                let mutations_tuple = ($(SerializeObserver::flush::<A>(&mut this.$n)?,)*);
                let mut mutations = Mutations::with_capacity(
                    0 $(+ mutations_tuple.$n.len())*
                );
                $(
                    mutations.insert($n, mutations_tuple.$n);
                )*
                Ok(mutations)
            }
        }

        impl<$($o,)* S: ?Sized, D> Debug for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: Debug,
        {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($ty)).field(&self.observed_ref()).finish()
            }
        }

        impl<$($o,)* S: ?Sized, D, U> PartialEq<(U,)> for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: PartialEq<(U,)>,
        {
            #[inline]
            fn eq(&self, other: &(U,)) -> bool {
                self.observed_ref().eq(other)
            }
        }

        impl<$($o,)* $($p,)* S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<$ty<$($p,)* S2, D2>>
            for $ty<$($o,)* S1, D1>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            $($p: QuasiObserver<Target: Deref<Target: AsDeref<$p::InnerDepth>>>,)*
            D1: Unsigned,
            D2: Unsigned,
            S1: AsDeref<D1>,
            S2: AsDeref<D2>,
            S1::Target: PartialEq<S2::Target>,
        {
            #[inline]
            fn eq(&self, other: &$ty<$($p,)* S2, D2>) -> bool {
                self.observed_ref().eq(other.observed_ref())
            }
        }

        impl<$($o,)* S: ?Sized, D> Eq for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: Eq,
        {
        }

        impl<$($o,)* S: ?Sized, D, U> PartialOrd<(U,)> for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: PartialOrd<(U,)>,
        {
            #[inline]
            fn partial_cmp(&self, other: &(U,)) -> Option<std::cmp::Ordering> {
                self.observed_ref().partial_cmp(other)
            }
        }

        impl<$($o,)* $($p,)* S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<$ty<$($p,)* S2, D2>>
            for $ty<$($o,)* S1, D1>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            $($p: QuasiObserver<Target: Deref<Target: AsDeref<$p::InnerDepth>>>,)*
            D1: Unsigned,
            D2: Unsigned,
            S1: AsDeref<D1>,
            S2: AsDeref<D2>,
            S1::Target: PartialOrd<S2::Target>,
        {
            #[inline]
            fn partial_cmp(&self, other: &$ty<$($p,)* S2, D2>) -> Option<std::cmp::Ordering> {
                self.observed_ref().partial_cmp(other.observed_ref())
            }
        }

        impl<$($o,)* S: ?Sized, D> Ord for $ty<$($o,)* S, D>
        where
            $($o: QuasiObserver<Target: Deref<Target: AsDeref<$o::InnerDepth>>>,)*
            D: Unsigned,
            S: AsDeref<D>,
            S::Target: Ord,
        {
            #[inline]
            fn cmp(&self, other: &$ty<$($o,)* S, D>) -> std::cmp::Ordering {
                self.observed_ref().cmp(other.observed_ref())
            }
        }

        #[cfg_attr(docsrs, doc(hidden))]
        impl<$($t,)*> Observe for ($($t,)*)
        where
            $($t: Observe,)*
        {
            type Observer<'ob, S, D>
                = $ty<$($t::Observer<'ob, $t, Zero>,)* S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

            type Spec = DefaultSpec;
        }

        #[cfg_attr(docsrs, doc(hidden))]
        impl<$($t,)*> RefObserve for ($($t,)*)
        where
            $($t: RefObserve,)*
        {
            type Observer<'ob, S, D>
                = PointerObserver<'ob, S, D>
            where
                Self: 'ob,
                D: Unsigned,
                S: AsDeref<D, Target = Self> + ?Sized + 'ob;

            type Spec = DefaultSpec;
        }
    };
}

tuple_observer!(TupleObserver2; 2; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1);
tuple_observer!(TupleObserver3; 3; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2);
tuple_observer!(TupleObserver4; 4; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3);
tuple_observer!(TupleObserver5; 5; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4);
tuple_observer!(TupleObserver6; 6; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5);
tuple_observer!(TupleObserver7; 7; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5; O7, P7, T7, U7, 6);
tuple_observer!(TupleObserver8; 8; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5; O7, P7, T7, U7, 6; O8, P8, T8, U8, 7);
tuple_observer!(TupleObserver9; 9; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5; O7, P7, T7, U7, 6; O8, P8, T8, U8, 7; O9, P9, T9, U9, 8);
tuple_observer!(TupleObserver10; 10; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5; O7, P7, T7, U7, 6; O8, P8, T8, U8, 7; O9, P9, T9, U9, 8; O10, P10, T10, U10, 9);
tuple_observer!(TupleObserver11; 11; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5; O7, P7, T7, U7, 6; O8, P8, T8, U8, 7; O9, P9, T9, U9, 8; O10, P10, T10, U10, 9; O11, P11, T11, U11, 10);
tuple_observer!(TupleObserver12; 12; O1, P1, T1, U1, 0; O2, P2, T2, U2, 1; O3, P3, T3, U3, 2; O4, P4, T4, U4, 3; O5, P5, T5, U5, 4; O6, P6, T6, U6, 5; O7, P7, T7, U7, 6; O8, P8, T8, U8, 7; O9, P9, T9, U9, 8; O10, P10, T10, U10, 9; O11, P11, T11, U11, 10; O12, P12, T12, U12, 11);

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};
    use crate::{Mutation, MutationKind};

    #[test]
    fn no_change_returns_none() {
        let mut tuple = (String::from("hello"),);
        let mut ob = tuple.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn deref_triggers_replace() {
        // Same-length replacement: inner StringObserver cannot detect this
        // because it only tracks length-based changes (append/truncate).
        let mut tuple = (String::from("hello"),);
        let mut ob = tuple.__observe();
        **ob = (String::from("world"),);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation.unwrap(),
            Mutation {
                path: vec![0.into()].into(),
                kind: MutationKind::Replace(json!("world")),
            }
        );

        // Longer replacement: without `as_deref_mut_coinductive`, inner
        // StringObserver would incorrectly produce Append(" world").
        let mut tuple = (String::from("hello"),);
        let mut ob = tuple.__observe();
        **ob = (String::from("hello world"),);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation.unwrap(),
            Mutation {
                path: vec![0.into()].into(),
                kind: MutationKind::Replace(json!("hello world")),
            }
        );
    }
}
