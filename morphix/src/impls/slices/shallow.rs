use std::ops::{Deref, DerefMut};

pub struct ShallowMut<'ob, T: ?Sized> {
    pub(crate) inner: &'ob mut T,
    pub(crate) mutated: *mut bool,
}

impl<'ob, T: ?Sized> Deref for ShallowMut<'ob, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ob, T: ?Sized> DerefMut for ShallowMut<'ob, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { *self.mutated = true }
        self.inner
    }
}

impl<'ob, T: ?Sized> ShallowMut<'ob, T> {
    pub fn new(inner: &'ob mut T, mutated: *mut bool) -> Self {
        Self { inner, mutated }
    }
}
