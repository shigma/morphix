use std::ops::{Deref, DerefMut};

use crate::helper::Assignable;
use crate::observe::{DefaultSpec, DerefInductive, DerefMutInductive, Pointer, Succ, Unsigned};
use crate::{Observe, Observer};

pub struct BoxObserver<O> {
    inner: O,
}

impl<O> Deref for BoxObserver<O> {
    type Target = O;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<O> DerefMut for BoxObserver<O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<O> Assignable for BoxObserver<O> {}

impl<O, N> Observer for BoxObserver<O>
where
    O: Observer<UpperDepth = Succ<N>>,
    N: Unsigned,
    O::Head: DerefMutInductive<N, Target = Box<<O::Head as DerefInductive<O::UpperDepth>>::Target>>,
{
    type LowerDepth = Succ<O::LowerDepth>;
    type UpperDepth = N;
    type Head = O::Head;

    #[inline]
    fn observe(value: &mut Self::Head) -> Self {
        Self {
            inner: O::observe(value),
        }
    }

    #[inline]
    fn as_ptr(this: &Self) -> &Pointer<Self::Head> {
        O::as_ptr(&this.inner)
    }

    #[inline]
    unsafe fn collect_unchecked<A: crate::Adapter>(this: &mut Self) -> Result<Option<crate::Mutation<A>>, A::Error> {
        unsafe { O::collect_unchecked(&mut this.inner) }
    }
}

impl<T> Observe for Box<T>
where
    T: Observe + BoxObserveImpl<T, T::Spec>,
{
    type Observer<'i, S, N>
        = <T as BoxObserveImpl<T, T::Spec>>::Observer<'i, S, N>
    where
        Self: 'i,
        N: Unsigned,
        S: DerefMutInductive<N, Target = Self> + ?Sized + 'i;

    type Spec = T::Spec;
}

#[doc(hidden)]
pub trait BoxObserveImpl<T: Observe, Spec> {
    type Observer<'i, S, N>: Observer<Head = S, UpperDepth = N>
    where
        T: 'i,
        N: Unsigned,
        S: DerefMutInductive<N, Target = Box<T>> + ?Sized + 'i;
}

impl<T> BoxObserveImpl<T, DefaultSpec> for T
where
    T: Observe<Spec = DefaultSpec>,
{
    type Observer<'i, S, N>
        = BoxObserver<T::Observer<'i, S, Succ<N>>>
    where
        T: 'i,
        N: Unsigned,
        S: DerefMutInductive<N, Target = Box<T>> + ?Sized + 'i;
}
