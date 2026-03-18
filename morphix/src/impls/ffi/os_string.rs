use std::ffi::OsString;

use crate::helper::macros::default_impl_ref_observe;

default_impl_ref_observe! {
    impl RefObserve for OsString;
}
