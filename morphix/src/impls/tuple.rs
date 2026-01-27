use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::PointerObserver;
use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDerefMut, AsNormalized, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, RefObserve, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe};

pub struct TupleObserver<'ob, O, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    inner: (O,),
    mutated: bool,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, O, S: ?Sized, D> Deref for TupleObserver<'ob, O, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, O, S: ?Sized, D> DerefMut for TupleObserver<'ob, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        &mut self.ptr
    }
}

impl<'ob, O, S: ?Sized, D> AsNormalized for TupleObserver<'ob, O, S, D> {
    type OuterDepth = Succ<Zero>;
}

impl<'ob, O, S: ?Sized, D> Observer<'ob> for TupleObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = (O::Head,)> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = D;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: ObserverPointer::uninit(),
            inner: (O::uninit(),),
            mutated: false,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        let ptr = ObserverPointer::new(value);
        let value = value.as_deref_mut();
        Self {
            ptr,
            inner: (O::observe(&mut value.0),),
            mutated: false,
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        ObserverPointer::set(&this.ptr, value);
        let value = value.as_deref_mut();
        unsafe {
            O::refresh(&mut this.inner.0, &mut value.0);
        }
    }
}

impl<'ob, O, S: ?Sized, D> SerializeObserver<'ob> for TupleObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = (O::Head,)> + 'ob,
    O: SerializeObserver<'ob, InnerDepth = Zero>,
    O::Head: Serialize + Sized,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        if this.mutated {
            return Ok(MutationKind::Replace(A::serialize_value(this.as_deref())?).into());
        }
        SerializeObserver::flush::<A>(&mut this.inner.0)
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

macro_rules! tuple_observer {
    ($ty:ident; $($o:ident, $t:ident, $n:tt);*) => {
        pub struct $ty<'ob, $($o,)* S: ?Sized, D = Zero> {
            ptr: ObserverPointer<S>,
            inner: ($($o,)*),
            mutated: bool,
            phantom: PhantomData<&'ob mut D>,
        }

        impl<'ob, $($o,)* S: ?Sized, D> Deref for $ty<'ob, $($o,)* S, D> {
            type Target = ObserverPointer<S>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.ptr
            }
        }

        impl<'ob, $($o,)* S: ?Sized, D> DerefMut for $ty<'ob, $($o,)* S, D> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.mutated = true;
                &mut self.ptr
            }
        }

        impl<'ob, $($o,)* S: ?Sized, D> AsNormalized for $ty<'ob, $($o,)* S, D> {
            type OuterDepth = Succ<Zero>;
        }

        impl<'ob, $($o,)* S: ?Sized, D> Observer<'ob> for $ty<'ob, $($o,)* S, D>
        where
            D: Unsigned,
            S: AsDerefMut<D, Target = ($($o::Head,)*)> + 'ob,
            $($o: Observer<'ob, InnerDepth = Zero, Head: Sized>,)*
        {
            type InnerDepth = D;
            type Head = S;

            #[inline]
            fn uninit() -> Self {
                Self {
                    ptr: ObserverPointer::uninit(),
                    inner: ($($o::uninit(),)*),
                    mutated: false,
                    phantom: PhantomData,
                }
            }

            #[inline]
            fn observe(value: &'ob mut Self::Head) -> Self {
                let ptr = ObserverPointer::new(value);
                let value = value.as_deref_mut();
                Self {
                    ptr,
                    inner: ($($o::observe(&mut value.$n),)*),
                    mutated: false,
                    phantom: PhantomData,
                }
            }

            #[inline]
            unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
                ObserverPointer::set(&this.ptr, value);
                let value = value.as_deref_mut();
                unsafe {
                    $($o::refresh(&mut this.inner.$n, &mut value.$n);)*
                }
            }
        }

        impl<'ob, $($o,)* S: ?Sized, D> SerializeObserver<'ob> for $ty<'ob, $($o,)* S, D>
        where
            D: Unsigned,
            S: AsDerefMut<D, Target = ($($o::Head,)*)> + 'ob,
            $($o: SerializeObserver<'ob, InnerDepth = Zero, Head: Serialize + Sized>,)*
        {
            #[inline]
            unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
                if this.mutated {
                    return Ok(MutationKind::Replace(A::serialize_value(this.as_deref())?).into());
                }
                let mutations_tuple = ($(SerializeObserver::flush::<A>(&mut this.inner.$n)?,)*);
                let mut mutations = Mutations::with_capacity(
                    0 $(+ mutations_tuple.$n.len())*
                );
                $(
                    mutations.insert($n, mutations_tuple.$n);
                )*
                Ok(mutations)
            }
        }

        #[cfg_attr(docsrs, doc(hidden))]
        impl<$($t,)*> Observe for ($($t,)*)
        where
            $($t: Observe,)*
        {
            type Observer<'ob, S, D>
                = $ty<'ob, $($t::Observer<'ob, $t, Zero>,)* S, D>
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
            type Observer<'ob, S, D, E>
                = PointerObserver<'ob, S, D, E>
            where
                Self: 'ob,
                D: Unsigned,
                E: Unsigned,
                S: $crate::helper::AsDeref<D> + ?Sized + 'ob,
                S::Target: $crate::helper::AsDeref<E, Target = Self>;

            type Spec = DefaultSpec;
        }
    };
}

tuple_observer!(TupleObserver2; O1, T1, 0; O2, T2, 1);
tuple_observer!(TupleObserver3; O1, T1, 0; O2, T2, 1; O3, T3, 2);
tuple_observer!(TupleObserver4; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3);
tuple_observer!(TupleObserver5; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4);
tuple_observer!(TupleObserver6; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5);
tuple_observer!(TupleObserver7; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6);
tuple_observer!(TupleObserver8; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6; O8, T8, 7);
tuple_observer!(TupleObserver9; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6; O8, T8, 7; O9, T9, 8);
tuple_observer!(TupleObserver10; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6; O8, T8, 7; O9, T9, 8; O10, T10, 9);
tuple_observer!(TupleObserver11; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6; O8, T8, 7; O9, T9, 8; O10, T10, 9; O11, T11, 10);
tuple_observer!(TupleObserver12; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6; O8, T8, 7; O9, T9, 8; O10, T10, 9; O11, T11, 10; O12, T12, 11);
