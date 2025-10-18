use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer<'i, S: ?Sized>(Option<*mut S>, PhantomData<&'i mut S>);

impl<'i, S: ?Sized> Pointer<'i, S> {
    #[inline]
    pub fn new(value: &mut S) -> Self {
        Self(Some(value), PhantomData)
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }
}

impl<'i, S: ?Sized> Default for Pointer<'i, S> {
    #[inline]
    fn default() -> Self {
        Self(None, PhantomData)
    }
}

impl<'i, S: ?Sized> Deref for Pointer<'i, S> {
    type Target = S;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &*ptr }
    }
}

impl<'i, S: ?Sized> DerefMut for Pointer<'i, S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut S {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &mut *ptr }
    }
}
