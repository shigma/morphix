use std::ops::{Index, IndexMut};
use std::slice::SliceIndex;

use crate::Observe;
use crate::builtin::{ShallowObserver, UnsizeObserver};
use crate::helper::macros::delegate_methods;
use crate::helper::{AsDeref, AsDerefMut, QuasiObserver, Unsigned};
use crate::observe::{DefaultSpec, RefObserve};

impl Observe for str {
    type Observer<'ob, S, D>
        = ShallowObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
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

impl<'ob, S: ?Sized, D> ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    delegate_methods! { tracked_mut() as str =>
        pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8];
        pub fn as_mut_ptr(&mut self) -> *mut u8;
        pub fn get_mut<I: SliceIndex<str>>(&mut self, i: I) -> Option<&mut I::Output>;
        pub unsafe fn get_unchecked_mut<I: SliceIndex<str>>(&mut self, i: I) -> &mut I::Output;
        pub fn split_at_mut(&mut self, mid: usize) -> (&mut str, &mut str);
        pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut str, &mut str)>;
        pub fn make_ascii_uppercase(&mut self);
        pub fn make_ascii_lowercase(&mut self);
    }
}

impl<'ob, S: ?Sized, D> AsMut<str> for ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    fn as_mut(&mut self) -> &mut str {
        self.tracked_mut()
    }
}

impl<'ob, S: ?Sized, D, I> Index<I> for ShallowObserver<'ob, S, D>
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

impl<'ob, S: ?Sized, D, I> IndexMut<I> for ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
    I: SliceIndex<str>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}
