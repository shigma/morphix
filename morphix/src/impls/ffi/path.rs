use std::path::Path;
use std::ptr::NonNull;

use crate::Mutations;
use crate::general::{DebugHandler, GeneralHandler, GeneralObserver, SerializeHandler};
use crate::helper::macros::shallow_observer;
use crate::helper::{AsDeref, ObserverState, Unsigned};
use crate::observe::{DefaultSpec, RefObserve};

shallow_observer! {
    struct PathObserver(Path);
}

pub struct PathHandler {
    raw_parts: Option<Option<(NonNull<()>, usize)>>,
}

impl ObserverState for PathHandler {
    type Target = Path;

    fn invalidate(this: &mut Self, value: &Path) {
        this.raw_parts.get_or_insert_with(|| {
            value
                .to_str()
                .map(|str| (NonNull::from(str).cast::<()>(), str.chars().count()))
        });
    }
}

impl GeneralHandler for PathHandler {
    fn observe(_: &Path) -> Self {
        Self { raw_parts: None }
    }
}

impl SerializeHandler for PathHandler {
    unsafe fn flush(&mut self, value: &Path) -> Mutations {
        let (old_addr, old_len) = match self.raw_parts.take() {
            None => return Mutations::new(),
            Some(None) => return Mutations::replace(value),
            Some(Some(parts)) => parts,
        };
        let Some(str) = value.to_str() else {
            return Mutations::replace(value);
        };
        let new_addr = NonNull::from(str).cast::<()>();
        let new_len = str.chars().count();
        if new_addr != old_addr {
            return Mutations::replace(value);
        }
        if new_len < old_len {
            #[cfg(not(feature = "truncate"))]
            return Mutations::replace(value);
            #[cfg(feature = "truncate")]
            return Mutations::truncate(old_len - new_len);
        }
        if new_len > old_len {
            #[cfg(not(feature = "append"))]
            return Mutations::replace(value);
            #[cfg(feature = "append")]
            return Mutations::append(&str[old_len..]);
        }
        Mutations::new()
    }
}

impl DebugHandler for PathHandler {
    const NAME: &'static str = "PathHandler";
}

impl RefObserve for Path {
    type Observer<'ob, S, D>
        = GeneralObserver<'ob, PathHandler, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}
