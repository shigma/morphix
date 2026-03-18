use std::ffi::{CStr, CString};

use crate::general::{Unsize, UnsizeObserver};
use crate::helper::macros::shallow_observer;
use crate::helper::{AsDeref, Unsigned};
use crate::observe::{DefaultSpec, RefObserve};

shallow_observer! {
    struct CStrObserver(CStr);
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? S: ?Sized, D> PartialEq<$ty> for CStrObserver<'ob, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
            {
                fn eq(&self, other: &$ty) -> bool {
                    (***self).as_deref().eq(other)
                }
            }
        )*
    };
}

generic_impl_partial_eq! {
    impl _ for CStr;
    impl _ for CString;
    impl ['a, T] _ for &'a T;
    impl ['a, T: ToOwned] _ for std::borrow::Cow<'a, T>;
}

impl Unsize for CStr {
    type Slice = [u8];

    fn len(&self) -> usize {
        self.to_bytes().len()
    }

    fn range_from(&self, from: usize) -> &Self::Slice {
        &self.to_bytes()[from..]
    }
}

impl RefObserve for CStr {
    type Observer<'ob, S, D>
        = UnsizeObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}
