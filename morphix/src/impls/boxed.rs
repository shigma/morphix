use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::helper::{AsDeref, AsDerefMut, Assignable, Succ, Unsigned};
use crate::observe::DefaultSpec;
use crate::{Adapter, Mutation, Observe, Observer};

#[derive(Default)]
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

impl<O, N, T: ?Sized> Observer for BoxObserver<O>
where
    O: Observer<UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
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
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        unsafe { O::collect_unchecked(&mut this.inner) }
    }
}

macro_rules! impl_fmt {
    ($($trait:ident),* $(,)?) => {
        $(
            impl<O: std::fmt::$trait> std::fmt::$trait for BoxObserver<O> {
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

impl<O: Debug> Debug for BoxObserver<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoxObserver").field(&self.inner).finish()
    }
}

impl<O, N, T: ?Sized, U: ?Sized> PartialEq<U> for BoxObserver<O>
where
    O: Observer<UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
    Box<T>: PartialEq<U>,
{
    fn eq(&self, other: &U) -> bool {
        (**O::as_ptr(self)).as_deref().eq(other)
    }
}

impl<O, N, T: ?Sized, U: ?Sized> PartialOrd<U> for BoxObserver<O>
where
    O: Observer<UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
    Box<T>: PartialOrd<U>,
{
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (**O::as_ptr(self)).as_deref().partial_cmp(other)
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
        S: AsDerefMut<N, Target = Self> + ?Sized + 'i;

    type Spec = T::Spec;
}

#[doc(hidden)]
pub trait BoxObserveImpl<T: Observe, Spec> {
    type Observer<'i, S, N>: Observer<Head = S, UpperDepth = N>
    where
        T: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Box<T>> + ?Sized + 'i;
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
        S: AsDerefMut<N, Target = Box<T>> + ?Sized + 'i;
}
