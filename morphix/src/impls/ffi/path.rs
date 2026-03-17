use std::path::Path;

use crate::helper::macros::shallow_observer;

shallow_observer! {
    struct PathObserver(Path);
}
