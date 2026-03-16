use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::helper::quasi::DerefMutUntracked;
use crate::helper::{QuasiObserver, Zero};

pub struct ShallowMut<'ob, T: ?Sized> {
    pub(crate) inner: &'ob mut T,
    pub(crate) mutated: *mut bool,
}

impl<'ob, T: ?Sized> ShallowMut<'ob, T> {
    pub fn new(inner: &'ob mut T, mutated: *mut bool) -> Self {
        Self { inner, mutated }
    }
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

impl<'ob, T: ?Sized> DerefMutUntracked for ShallowMut<'ob, T> {}

impl<'ob, T: ?Sized> QuasiObserver for ShallowMut<'ob, T> {
    type Head = T;
    type OuterDepth = Zero;
    type InnerDepth = Zero;

    fn invalidate(_: &mut Self) {}
}

impl<'ob, T: ?Sized> AsMut<T> for ShallowMut<'ob, T> {
    fn as_mut(&mut self) -> &mut T {
        self.tracked_mut()
    }
}

impl<'ob, T: Debug + ?Sized> Debug for ShallowMut<'ob, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShallowMut").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, T1: ?Sized, T2: ?Sized> PartialEq<ShallowMut<'ob, T2>> for ShallowMut<'ob, T1>
where
    T1: PartialEq<T2>,
{
    fn eq(&self, other: &ShallowMut<'ob, T2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, T: Eq + ?Sized> Eq for ShallowMut<'ob, T> {}

impl<'ob, T1: ?Sized, T2: ?Sized> PartialOrd<ShallowMut<'ob, T2>> for ShallowMut<'ob, T1>
where
    T1: PartialOrd<T2>,
{
    fn partial_cmp(&self, other: &ShallowMut<'ob, T2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<'ob, T: Ord + ?Sized> Ord for ShallowMut<'ob, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
    }
}

impl<'ob, T: Index<I> + ?Sized, I> Index<I> for ShallowMut<'ob, T> {
    type Output = T::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.untracked_ref().index(index)
    }
}

impl<'ob, T: IndexMut<I> + ?Sized, I> IndexMut<I> for ShallowMut<'ob, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? T: ?Sized> PartialEq<$ty> for ShallowMut<'ob, T>
            where
                T: PartialEq<$ty>,
            {
                fn eq(&self, other: &$ty) -> bool {
                    (**self).eq(other)
                }
            }

            impl<'ob, $($($gen)*,)? T: ?Sized> PartialOrd<$ty> for ShallowMut<'ob, T>
            where
                T: PartialOrd<$ty>,
            {
                fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                    (**self).partial_cmp(other)
                }
            }
        )*
    };
}

generic_impl_cmp! {
    impl _ for str;
    impl _ for String;
    impl _ for std::ffi::OsStr;
    impl _ for std::ffi::OsString;
    impl _ for std::path::Path;
    impl _ for std::path::PathBuf;
    impl ['a] _ for std::borrow::Cow<'a, str>;
}
