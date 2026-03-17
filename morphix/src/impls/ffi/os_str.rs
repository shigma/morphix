use std::ffi::OsStr;

use crate::helper::macros::shallow_observer;

shallow_observer! {
    struct OsStrObserver(OsStr);
}
