use std::ops::{Deref, DerefMut};

use crate::helper::unsigned::{Succ, Unsigned, Zero};

pub trait AsDeref<N: Unsigned> {
    type Target: ?Sized;

    fn as_deref(&self) -> &Self::Target;
}

pub trait AsDerefMut<N: Unsigned>: AsDeref<N> {
    fn as_deref_mut(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> AsDeref<Zero> for T {
    type Target = T;

    #[inline]
    fn as_deref(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsDerefMut<Zero> for T {
    #[inline]
    fn as_deref_mut(&mut self) -> &mut T {
        self
    }
}

impl<T: AsDeref<N, Target: Deref> + ?Sized, N: Unsigned> AsDeref<Succ<N>> for T {
    type Target = <T::Target as Deref>::Target;

    #[inline]
    fn as_deref(&self) -> &Self::Target {
        self.as_deref().deref()
    }
}

impl<T: AsDerefMut<N, Target: DerefMut> + ?Sized, N: Unsigned> AsDerefMut<Succ<N>> for T {
    #[inline]
    fn as_deref_mut(&mut self) -> &mut Self::Target {
        self.as_deref_mut().deref_mut()
    }
}

pub trait AsDerefCoinductive<N: Unsigned> {
    type Target: ?Sized;

    fn as_deref_coinductive(&self) -> &Self::Target;
}

pub trait AsDerefMutCoinductive<N: Unsigned>: AsDerefCoinductive<N> {
    fn as_deref_mut_coinductive(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> AsDerefCoinductive<Zero> for T {
    type Target = T;

    #[inline]
    fn as_deref_coinductive(&self) -> &T {
        self
    }
}

impl<T: ?Sized> AsDerefMutCoinductive<Zero> for T {
    #[inline]
    fn as_deref_mut_coinductive(&mut self) -> &mut T {
        self
    }
}

impl<T: Deref<Target: AsDerefCoinductive<N>> + ?Sized, N: Unsigned> AsDerefCoinductive<Succ<N>> for T {
    type Target = <T::Target as AsDerefCoinductive<N>>::Target;

    #[inline]
    fn as_deref_coinductive(&self) -> &Self::Target {
        self.deref().as_deref_coinductive()
    }
}

impl<T: DerefMut<Target: AsDerefMutCoinductive<N>> + ?Sized, N: Unsigned> AsDerefMutCoinductive<Succ<N>> for T {
    #[inline]
    fn as_deref_mut_coinductive(&mut self) -> &mut Self::Target {
        self.deref_mut().as_deref_mut_coinductive()
    }
}
