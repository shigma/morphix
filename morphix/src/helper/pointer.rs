use std::ops::{Deref, DerefMut};

pub struct Pointer<S: ?Sized>(Option<*mut S>);

impl<S: ?Sized> Pointer<S> {
    #[inline]
    pub fn new(value: &mut S) -> Self {
        Self(Some(value))
    }

    #[inline]
    pub fn is_null(this: &Self) -> bool {
        this.0.is_none()
    }

    #[inline]
    #[expect(clippy::should_implement_trait)]
    pub fn as_ref<'i>(this: &Self) -> &'i S {
        let ptr = this.0.expect("Observed pointer should not be null");
        unsafe { &*ptr }
    }

    #[inline]
    pub fn as_mut<'i>(this: &Self) -> &'i mut S {
        let ptr = this.0.expect("Observed pointer should not be null");
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
        Self::as_ref(self)
    }
}

impl<S: ?Sized> DerefMut for Pointer<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut S {
        Self::as_mut(self)
    }
}
