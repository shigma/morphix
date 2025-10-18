use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer<S: ?Sized>(Option<*mut S>);

impl<S: ?Sized> Pointer<S> {
    #[inline]
    pub fn new(value: &mut S) -> Self {
        Self(Some(value))
    }

    #[inline]
    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }
}

impl<S: ?Sized> Default for Pointer<S> {
    #[inline]
    fn default() -> Self {
        Self(None)
    }
}

impl<S: ?Sized> Deref for Pointer<S> {
    type Target = S;

    #[inline]
    fn deref(&self) -> &Self::Target {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &*ptr }
    }
}

impl<S: ?Sized> DerefMut for Pointer<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut S {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &mut *ptr }
    }
}
