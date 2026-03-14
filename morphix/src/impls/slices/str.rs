use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

use crate::Observe;
use crate::builtin::UnsizeObserver;
use crate::helper::macros::{delegate_methods, shallow_observer};
use crate::helper::{AsDeref, AsDerefMut, QuasiObserver, Unsigned};
use crate::impls::slices::shallow::ShallowMut;
use crate::observe::{DefaultSpec, RefObserve};

shallow_observer! {
    impl StrObserver for str;
}

impl RefObserve for str {
    type Observer<'ob, S, D>
        = UnsizeObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl<'ob, S: ?Sized, D> StrObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    fn nonempty_mut(&mut self) -> &mut str {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut()
        } else {
            self.tracked_mut()
        }
    }

    delegate_methods! { nonempty_mut() as str =>
        pub fn as_mut_ptr(&mut self) -> *mut u8;
        pub fn make_ascii_uppercase(&mut self);
        pub fn make_ascii_lowercase(&mut self);
    }

    /// See [`str::as_bytes_mut`].
    pub unsafe fn as_bytes_mut(&mut self) -> ShallowMut<'_, [u8]> {
        let inner = unsafe { (*self.ptr).as_deref_mut().as_bytes_mut() };
        ShallowMut::new(inner, &raw mut self.mutated)
    }

    /// See [`str::get_mut`].
    pub fn get_mut<I: SliceIndex<str, Output = str>>(&mut self, i: I) -> Option<ShallowMut<'_, str>> {
        let output = (*self.ptr).as_deref_mut().get_mut(i)?;
        Some(ShallowMut::new(output, &raw mut self.mutated))
    }

    /// See [`str::get_unchecked_mut`].
    pub unsafe fn get_unchecked_mut<I: SliceIndex<str, Output = str>>(&mut self, i: I) -> ShallowMut<'_, str> {
        let output = unsafe { (*self.ptr).as_deref_mut().get_unchecked_mut(i) };
        ShallowMut::new(output, &raw mut self.mutated)
    }

    /// See [`str::split_at_mut`].
    pub fn split_at_mut(&mut self, mid: usize) -> (ShallowMut<'_, str>, ShallowMut<'_, str>) {
        let (left, right) = (*self.ptr).as_deref_mut().split_at_mut(mid);
        (
            ShallowMut::new(left, &raw mut self.mutated),
            ShallowMut::new(right, &raw mut self.mutated),
        )
    }

    /// See [`str::split_at_mut_checked`].
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(ShallowMut<'_, str>, ShallowMut<'_, str>)> {
        let (left, right) = (*self.ptr).as_deref_mut().split_at_mut_checked(mid)?;
        Some((
            ShallowMut::new(left, &raw mut self.mutated),
            ShallowMut::new(right, &raw mut self.mutated),
        ))
    }
}

impl<'ob, S: ?Sized, D> AsMut<str> for StrObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    fn as_mut(&mut self) -> &mut str {
        self.tracked_mut()
    }
}

impl<'ob, S: ?Sized, D, I> Index<I> for StrObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
    I: SliceIndex<str>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.untracked_ref().index(index)
    }
}

impl<'ob, S: ?Sized, D, I> IndexMut<I> for StrObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
    I: SliceIndex<str>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}

impl<'ob> ShallowMut<'ob, str> {
    fn nonempty_mut(&mut self) -> &mut str {
        if !self.inner.is_empty() {
            unsafe { *self.mutated = true }
        }
        self.inner
    }

    delegate_methods! { nonempty_mut() as str =>
        pub fn as_mut_ptr(&mut self) -> *mut u8;
        pub fn make_ascii_uppercase(&mut self);
        pub fn make_ascii_lowercase(&mut self);
    }

    /// See [`str::as_bytes_mut`].
    pub unsafe fn as_bytes_mut(&mut self) -> ShallowMut<'_, [u8]> {
        let inner = unsafe { self.inner.as_bytes_mut() };
        ShallowMut::new(inner, self.mutated)
    }

    /// See [`str::get_mut`].
    pub fn get_mut<I: SliceIndex<str, Output = str>>(&mut self, i: I) -> Option<ShallowMut<'_, str>> {
        let output = self.inner.get_mut(i)?;
        Some(ShallowMut::new(output, self.mutated))
    }

    /// See [`str::get_unchecked_mut`].
    pub unsafe fn get_unchecked_mut<I: SliceIndex<str, Output = str>>(&mut self, i: I) -> ShallowMut<'_, str> {
        let output = unsafe { self.inner.get_unchecked_mut(i) };
        ShallowMut::new(output, self.mutated)
    }

    /// See [`str::split_at_mut`].
    pub fn split_at_mut(&mut self, mid: usize) -> (ShallowMut<'_, str>, ShallowMut<'_, str>) {
        let (left, right) = self.inner.split_at_mut(mid);
        (
            ShallowMut::new(left, self.mutated),
            ShallowMut::new(right, self.mutated),
        )
    }

    /// See [`str::split_at_mut_checked`].
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(ShallowMut<'_, str>, ShallowMut<'_, str>)> {
        let (left, right) = self.inner.split_at_mut_checked(mid)?;
        Some((
            ShallowMut::new(left, self.mutated),
            ShallowMut::new(right, self.mutated),
        ))
    }
}
