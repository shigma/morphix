use std::ffi::CStr;

use crate::helper::macros::shallow_observer;

shallow_observer! {
    impl CStrObserver for CStr;
}
