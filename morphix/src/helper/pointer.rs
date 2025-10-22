use std::ops::{Deref, DerefMut};

pub struct Pointer<S: ?Sized>(Option<*mut S>);

impl<S: ?Sized> Pointer<S> {
    #[inline]
    pub fn new(value: &mut S) -> Self {
        Self(Some(value))
    }

    #[inline]
    pub(crate) fn is_null(&self) -> bool {
        self.0.is_none()
    }

    #[inline]
    pub(crate) fn as_ref<'i>(&self) -> &'i S {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &*ptr }
    }

    #[inline]
    #[allow(clippy::mut_from_ref)]
    pub(crate) fn as_mut<'i>(&self) -> &'i mut S {
        let ptr = self.0.expect("Observed pointer should not be null");
        unsafe { &mut *ptr }
    }
}

impl<S: ?Sized> Default for Pointer<S> {
    #[inline]
    fn default() -> Self {
        Self(None)
    }
}

impl<S: ?Sized> PartialEq for Pointer<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<S: ?Sized> Eq for Pointer<S> {}

impl<S: ?Sized> Deref for Pointer<S> {
    type Target = S;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<S: ?Sized> DerefMut for Pointer<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut S {
        self.as_mut()
    }
}
