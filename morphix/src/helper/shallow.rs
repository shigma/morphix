//! [`ShallowMut`]: a thin mutable wrapper that invalidates a shared state on `DerefMut`.

use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::helper::quasi::DerefMutUntracked;
use crate::helper::{QuasiInvalidate, QuasiObserver, Zero};

impl QuasiInvalidate for bool {
    fn invalidate(this: &mut Self) {
        *this = true;
    }
}

/// A mutable handle to a value `T` that invalidates a shared state `V` whenever it is mutated.
///
/// [`ShallowMut`] decouples the borrowed value from its invalidation target: [`Self::inner`] is the
/// value the caller mutates, while [`Self::state`] is a raw pointer to a separate piece of state
/// (often living on a parent observer) that gets invalidated through the [`QuasiInvalidate`] trait
/// on each [`DerefMut`].
pub struct ShallowMut<'ob, T: ?Sized, V: ?Sized> {
    pub(crate) inner: &'ob mut T,
    pub(crate) state: *mut V,
}

impl<'ob, T: ?Sized, V: ?Sized> ShallowMut<'ob, T, V> {
    /// Constructs a [`ShallowMut`] from a mutable borrow and a raw pointer to its invalidation
    /// state.
    ///
    /// The state pointer must remain valid for the lifetime `'ob`.
    pub fn new(inner: &'ob mut T, state: *mut V) -> Self {
        Self { inner, state }
    }
}

impl<'ob, T: ?Sized, V: ?Sized> Deref for ShallowMut<'ob, T, V> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ob, T: ?Sized, V: QuasiInvalidate + ?Sized> DerefMut for ShallowMut<'ob, T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { QuasiInvalidate::invalidate(&mut *self.state) }
        self.inner
    }
}

impl<'ob, T: ?Sized, V: QuasiInvalidate + ?Sized> DerefMutUntracked for ShallowMut<'ob, T, V> {}

impl<'ob, T: ?Sized, V: ?Sized> QuasiObserver for ShallowMut<'ob, T, V> {
    type Head = T;
    type OuterDepth = Zero;
    type InnerDepth = Zero;

    fn invalidate(_: &mut Self) {}
}

impl<'ob, T: ?Sized, V: QuasiInvalidate + ?Sized> AsMut<T> for ShallowMut<'ob, T, V> {
    fn as_mut(&mut self) -> &mut T {
        self.tracked_mut()
    }
}

impl<'ob, T: Debug + ?Sized, V: ?Sized> Debug for ShallowMut<'ob, T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShallowMut").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, T1: ?Sized, T2: ?Sized, V1: ?Sized, V2: ?Sized> PartialEq<ShallowMut<'ob, T2, V2>> for ShallowMut<'ob, T1, V1>
where
    T1: PartialEq<T2>,
{
    fn eq(&self, other: &ShallowMut<'ob, T2, V2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, T: Eq + ?Sized, V: ?Sized> Eq for ShallowMut<'ob, T, V> {}

impl<'ob, T1: ?Sized, T2: ?Sized, V1: ?Sized, V2: ?Sized> PartialOrd<ShallowMut<'ob, T2, V2>>
    for ShallowMut<'ob, T1, V1>
where
    T1: PartialOrd<T2>,
{
    fn partial_cmp(&self, other: &ShallowMut<'ob, T2, V2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<'ob, T: Ord + ?Sized, V: ?Sized> Ord for ShallowMut<'ob, T, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
    }
}

impl<'ob, T: Index<I> + ?Sized, I, V: ?Sized> Index<I> for ShallowMut<'ob, T, V> {
    type Output = T::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.untracked_ref().index(index)
    }
}

impl<'ob, T: IndexMut<I> + ?Sized, I, V: QuasiInvalidate + ?Sized> IndexMut<I> for ShallowMut<'ob, T, V> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? T: ?Sized, V: ?Sized> PartialEq<$ty> for ShallowMut<'ob, T, V>
            where
                T: PartialEq<$ty>,
            {
                fn eq(&self, other: &$ty) -> bool {
                    (**self).eq(other)
                }
            }

            impl<'ob, $($($gen)*,)? T: ?Sized, V: ?Sized> PartialOrd<$ty> for ShallowMut<'ob, T, V>
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
