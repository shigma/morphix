use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, RefObserve, RefObserver, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe};

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

impl<'ob, O, S> Assignable for TupleObserver<'ob, O, S> {
    type Depth = Succ<Zero>;
}

impl<'ob, O, S: ?Sized, D> Observer<'ob> for TupleObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = (O::Head,)> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: ObserverPointer::default(),
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
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        if this.mutated {
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref())?),
            }));
        }
        let mut mutations = Vec::with_capacity(1);
        if let Some(mut mutation) = SerializeObserver::collect::<A>(&mut this.inner.0)? {
            mutation.path.push(0.into());
            mutations.push(mutation);
        }
        Ok(Mutation::coalesce(mutations))
    }
}

spec_impl_observe!(TupleObserveImpl, (Self,), (T,), TupleObserver);
spec_impl_ref_observe!(TupleRefObserveImpl, (Self,), (T,));

macro_rules! tuple_observer {
    ($ty:ident, $len:literal; $($o:ident, $t:ident, $n:tt);*) => {
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

        impl<'ob, $($o,)* S> Assignable for $ty<'ob, $($o,)* S> {
            type Depth = Succ<Zero>;
        }

        impl<'ob, $($o,)* S: ?Sized, D> Observer<'ob> for $ty<'ob, $($o,)* S, D>
        where
            D: Unsigned,
            S: AsDerefMut<D, Target = ($($o::Head,)*)> + 'ob,
            $($o: Observer<'ob, InnerDepth = Zero, Head: Sized>,)*
        {
            type InnerDepth = D;
            type OuterDepth = Zero;
            type Head = S;

            #[inline]
            fn uninit() -> Self {
                Self {
                    ptr: ObserverPointer::default(),
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
            unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
                if this.mutated {
                    return Ok(Some(Mutation {
                        path: Default::default(),
                        kind: MutationKind::Replace(A::serialize_value(this.as_deref())?),
                    }));
                }
                let mut mutations = Vec::with_capacity($len);
                $(
                    if let Some(mut mutation) = SerializeObserver::collect::<A>(&mut this.inner.$n)? {
                        mutation.path.push($n.into());
                        mutations.push(mutation);
                    }
                )*
                Ok(Mutation::coalesce(mutations))
            }
        }

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

        impl<$($t,)*> RefObserve for ($($t,)*)
        where
            $($t: RefObserve,)*
        {
            type Observer<'a, 'ob, S, D>
                = RefObserver<'a, 'ob, S, D>
            where
                Self: 'a + 'ob,
                D: Unsigned,
                S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

            type Spec = DefaultSpec;
        }
    };
}

tuple_observer!(TupleObserver2, 2; O1, T1, 0; O2, T2, 1);
tuple_observer!(TupleObserver3, 3; O1, T1, 0; O2, T2, 1; O3, T3, 2);
tuple_observer!(TupleObserver4, 4; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3);
tuple_observer!(TupleObserver5, 5; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4);
tuple_observer!(TupleObserver6, 6; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5);
tuple_observer!(TupleObserver7, 7; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6);
tuple_observer!(TupleObserver8, 8; O1, T1, 0; O2, T2, 1; O3, T3, 2; O4, T4, 3; O5, T5, 4; O6, T6, 5; O7, T7, 6; O8, T8, 7);
