use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::helper::{AsDeref, AsDerefMut, Assignable, Succ, Unsigned};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, Mutation, Observe};

#[derive(Default)]
pub struct BoxObserver<'i, O> {
    inner: O,
    phantom: PhantomData<&'i mut ()>,
}

impl<'i, O> Deref for BoxObserver<'i, O> {
    type Target = O;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'i, O> DerefMut for BoxObserver<'i, O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'i, O> Assignable for BoxObserver<'i, O>
where
    O: Observer<'i>,
{
    type Depth = Succ<O::LowerDepth>;
}

impl<'i, O, N, T: ?Sized> Observer<'i> for BoxObserver<'i, O>
where
    O: Observer<'i, UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
{
    type LowerDepth = Succ<O::LowerDepth>;
    type UpperDepth = N;
    type Head = O::Head;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            inner: O::observe(value),
            phantom: PhantomData,
        }
    }
}

impl<'i, O, N, T: ?Sized> SerializeObserver<'i> for BoxObserver<'i, O>
where
    O: SerializeObserver<'i, UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
{
    #[inline]
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        unsafe { O::collect_unchecked(&mut this.inner) }
    }
}

macro_rules! impl_fmt {
    ($($trait:ident),* $(,)?) => {
        $(
            impl<'i, O: std::fmt::$trait> std::fmt::$trait for BoxObserver<'i, O> {
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

impl<'i, O: Debug> Debug for BoxObserver<'i, O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoxObserver").field(&self.inner).finish()
    }
}

impl<'i, O, N, T: ?Sized, U: ?Sized> PartialEq<U> for BoxObserver<'i, O>
where
    O: Observer<'i, UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
    Box<T>: PartialEq<U>,
{
    fn eq(&self, other: &U) -> bool {
        AsDeref::<N>::as_deref(&**O::as_ptr(self)).eq(other)
    }
}

impl<'i, O, N, T: ?Sized, U: ?Sized> PartialOrd<U> for BoxObserver<'i, O>
where
    O: Observer<'i, UpperDepth = Succ<N>>,
    O::Head: AsDerefMut<N, Target = Box<T>>,
    N: Unsigned,
    Box<T>: PartialOrd<U>,
{
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        AsDeref::<N>::as_deref(&**O::as_ptr(self)).partial_cmp(other)
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
    type Observer<'i, S, N>: Observer<'i, Head = S, UpperDepth = N>
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
        = BoxObserver<'i, T::Observer<'i, S, Succ<N>>>
    where
        T: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Box<T>> + ?Sized + 'i;
}
