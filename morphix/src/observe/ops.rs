use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

pub trait Unsigned: 'static {}

pub struct Zero;
pub struct Succ<N: Unsigned>(PhantomData<N>);

impl Unsigned for Zero {}
impl<N: Unsigned> Unsigned for Succ<N> {}

pub trait DerefCoinductive<N: Unsigned> {
    type Target: ?Sized;

    fn deref_coinductive(&self) -> &Self::Target;
}

pub trait DerefMutCoinductive<N: Unsigned>: DerefCoinductive<N> {
    fn deref_mut_coinductive(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> DerefCoinductive<Zero> for T {
    type Target = T;

    #[inline]
    fn deref_coinductive(&self) -> &T {
        self
    }
}

impl<T: ?Sized> DerefMutCoinductive<Zero> for T {
    #[inline]
    fn deref_mut_coinductive(&mut self) -> &mut T {
        self
    }
}

impl<T: Deref<Target: DerefCoinductive<N>> + ?Sized, N: Unsigned> DerefCoinductive<Succ<N>> for T {
    type Target = <T::Target as DerefCoinductive<N>>::Target;

    #[inline]
    fn deref_coinductive(&self) -> &Self::Target {
        self.deref().deref_coinductive()
    }
}

impl<T: DerefMut<Target: DerefMutCoinductive<N>> + ?Sized, N: Unsigned> DerefMutCoinductive<Succ<N>> for T {
    #[inline]
    fn deref_mut_coinductive(&mut self) -> &mut Self::Target {
        self.deref_mut().deref_mut_coinductive()
    }
}

pub trait DerefInductive<N: Unsigned> {
    type Target: ?Sized;

    fn deref_inductive(&self) -> &Self::Target;
}

pub trait DerefMutInductive<N: Unsigned>: DerefInductive<N> {
    fn deref_mut_inductive(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> DerefInductive<Zero> for T {
    type Target = T;

    #[inline]
    fn deref_inductive(&self) -> &T {
        self
    }
}

impl<T: ?Sized> DerefMutInductive<Zero> for T {
    #[inline]
    fn deref_mut_inductive(&mut self) -> &mut T {
        self
    }
}

impl<T: DerefInductive<N, Target: Deref> + ?Sized, N: Unsigned> DerefInductive<Succ<N>> for T {
    type Target = <T::Target as Deref>::Target;

    #[inline]
    fn deref_inductive(&self) -> &Self::Target {
        self.deref_inductive().deref()
    }
}

impl<T: DerefMutInductive<N, Target: DerefMut> + ?Sized, N: Unsigned> DerefMutInductive<Succ<N>> for T {
    #[inline]
    fn deref_mut_inductive(&mut self) -> &mut Self::Target {
        self.deref_mut_inductive().deref_mut()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer<T: ?Sized>(Option<*mut T>);

impl<T: ?Sized> Pointer<T> {
    #[inline]
    pub fn new(value: &mut T) -> Self {
        Self(Some(value))
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }
}

impl<T: ?Sized> Default for Pointer<T> {
    #[inline]
    fn default() -> Self {
        Self(None)
    }
}

impl<T: ?Sized> Deref for Pointer<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &*ptr }
    }
}

impl<T: ?Sized> DerefMut for Pointer<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &mut *ptr }
    }
}
