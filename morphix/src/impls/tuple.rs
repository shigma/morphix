use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{
    DefaultSpec, HashObserver, HashSpec, Observer, ObserverPointer, SerializeObserver, SnapshotObserver, SnapshotSpec,
};
use crate::{Adapter, Mutation, MutationKind, Observe};

pub struct TupleObserver<'ob, O, S: ?Sized, N = Zero> {
    ptr: ObserverPointer<S>,
    inner: (O,),
    mutated: bool,
    phantom: PhantomData<&'ob mut N>,
}

impl<'ob, O, S: ?Sized, N> Deref for TupleObserver<'ob, O, S, N> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, O, S: ?Sized, N> DerefMut for TupleObserver<'ob, O, S, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        &mut self.ptr
    }
}

impl<'ob, O, S> Assignable for TupleObserver<'ob, O, S> {
    type Depth = Succ<Zero>;
}

impl<'ob, O, S: ?Sized, N> Observer<'ob> for TupleObserver<'ob, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = (O::Head,)> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = N;
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

impl<'ob, O, S: ?Sized, N> SerializeObserver<'ob> for TupleObserver<'ob, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = (O::Head,)> + 'ob,
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

impl<T> Observe for (T,)
where
    T: Observe + TupleObserveImpl<T::Spec>,
{
    type Observer<'ob, S, N>
        = <T as TupleObserveImpl<T::Spec>>::Observer<'ob, S, N>
    where
        Self: 'ob,
        N: Unsigned,
        S: AsDerefMut<N, Target = Self> + ?Sized + 'ob;

    type Spec = T::Spec;
}

/// Helper trait for selecting appropriate observer implementations for [`Option<T>`].
pub trait TupleObserveImpl<Spec> {
    /// The observer type for [`Option<T>`] with the given specification.
    type Observer<'ob, S, N>: Observer<'ob, Head = S, InnerDepth = N>
    where
        Self: 'ob,
        N: Unsigned,
        S: AsDerefMut<N, Target = (Self,)> + ?Sized + 'ob;
}

impl<T> TupleObserveImpl<DefaultSpec> for T
where
    T: Observe<Spec = DefaultSpec>,
{
    type Observer<'ob, S, N>
        = TupleObserver<'ob, T::Observer<'ob, T, Zero>, S, N>
    where
        T: 'ob,
        N: Unsigned,
        S: AsDerefMut<N, Target = (Self,)> + ?Sized + 'ob;
}

impl<T> TupleObserveImpl<HashSpec> for T
where
    T: Hash + Observe<Spec = HashSpec>,
{
    type Observer<'ob, S, N>
        = HashObserver<'ob, S, N>
    where
        Self: 'ob,
        N: Unsigned,
        S: AsDerefMut<N, Target = (Self,)> + ?Sized + 'ob;
}

impl<T> TupleObserveImpl<SnapshotSpec> for T
where
    T: Clone + PartialEq + Observe<Spec = SnapshotSpec>,
{
    type Observer<'ob, S, N>
        = SnapshotObserver<'ob, S, N>
    where
        Self: 'ob,
        N: Unsigned,
        S: AsDerefMut<N, Target = (Self,)> + ?Sized + 'ob;
}

macro_rules! tuple_observer {
    ($ty:ident, $len:literal; $($o:ident, $t:ident, $n:tt);*) => {
        pub struct $ty<'ob, $($o,)* S: ?Sized, N = Zero> {
            ptr: ObserverPointer<S>,
            inner: ($($o,)*),
            mutated: bool,
            phantom: PhantomData<&'ob mut N>,
        }

        impl<'ob, $($o,)* S: ?Sized, N> Deref for $ty<'ob, $($o,)* S, N> {
            type Target = ObserverPointer<S>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.ptr
            }
        }

        impl<'ob, $($o,)* S: ?Sized, N> DerefMut for $ty<'ob, $($o,)* S, N> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                self.mutated = true;
                &mut self.ptr
            }
        }

        impl<'ob, $($o,)* S> Assignable for $ty<'ob, $($o,)* S> {
            type Depth = Succ<Zero>;
        }

        impl<'ob, $($o,)* S: ?Sized, N> Observer<'ob> for $ty<'ob, $($o,)* S, N>
        where
            N: Unsigned,
            S: AsDerefMut<N, Target = ($($o::Head,)*)> + 'ob,
            $($o: Observer<'ob, InnerDepth = Zero, Head: Sized>,)*
        {
            type InnerDepth = N;
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

        impl<'ob, $($o,)* S: ?Sized, N> SerializeObserver<'ob> for $ty<'ob, $($o,)* S, N>
        where
            N: Unsigned,
            S: AsDerefMut<N, Target = ($($o::Head,)*)> + 'ob,
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
            type Observer<'ob, S, N>
                = $ty<'ob, $($t::Observer<'ob, $t, Zero>,)* S, N>
            where
                Self: 'ob,
                N: Unsigned,
                S: AsDerefMut<N, Target = Self> + ?Sized + 'ob;

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
