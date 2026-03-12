use std::slice::SliceIndex;

use crate::Observe;
use crate::builtin::ShallowObserver;
use crate::helper::macros::delegate_methods;
use crate::helper::{AsDerefMut, QuasiObserver, Unsigned};
use crate::observe::DefaultSpec;

impl Observe for str {
    type Observer<'ob, S, D>
        = ShallowObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl<'ob, S: ?Sized, D> ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    delegate_methods! { tracked_mut() as str =>
        pub fn as_mut_ptr(&mut self) -> *mut u8;
        pub unsafe fn as_bytes_mut(&mut self) -> &mut [u8];
        pub fn get_mut<I>(&mut self, i: I) -> Option<&mut I::Output> where I: SliceIndex<str>;
        pub unsafe fn get_unchecked_mut<I>(&mut self, i: I) -> &mut I::Output where I: SliceIndex<str>;
        pub fn make_ascii_lowercase(&mut self);
        pub fn make_ascii_uppercase(&mut self);
        pub fn split_at_mut(&mut self, mid: usize) -> (&mut str, &mut str);
        pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut str, &mut str)>;
    }
}

impl<'ob, S: ?Sized, D> AsMut<str> for ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = str>,
{
    #[inline]
    fn as_mut(&mut self) -> &mut str {
        self.tracked_mut()
    }
}
