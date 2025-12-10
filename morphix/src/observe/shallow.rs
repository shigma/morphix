use std::marker::PhantomData;

use crate::Observe;
use crate::helper::{AsDeref, AsDerefMut, Unsigned, Zero};
use crate::observe::general::ReplaceHandler;
use crate::observe::{DebugHandler, DefaultSpec, GeneralHandler, GeneralObserver};

/// A general observer that tracks any mutation access as a change.
///
/// [`ShallowObserver`] uses a simple boolean flag to track whether [`DerefMut`](std::ops::DerefMut)
/// has been called, treating any mutable access as a change. This makes it extremely efficient with
/// minimal overhead.
///
/// ## Derive Usage
///
/// Can be used via the `#[morphix(shallow)]` attribute in derive macros:
///
/// ```
/// # use morphix::Observe;
/// # use serde::Serialize;
/// # #[derive(Serialize)]
/// # struct ExternalType;
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     #[morphix(shallow)]
///     external_data: ExternalType,    // ExternalType doesn't implement Observe
/// }
/// ```
///
/// ## When to Use
///
/// Despite its limitations, [`ShallowObserver`] is usually the best choice for external types that
/// don't implement the [`Observe`] trait, as the performance benefits typically outweigh
/// the occasional false positive.
///
/// ## Limitations
///
/// 1. **False positives on round-trip changes**: If a value is modified and then restored to its
///    original value, it's still reported as changes.
/// 2. **False positives on non-semantic changes**: Operations that don't affect serialization (such
///    as [`Vec::reserve`]) are still reported as changes.
pub type ShallowObserver<'ob, S, D = Zero> = GeneralObserver<'ob, ShallowHandler<<S as AsDeref<D>>::Target>, S, D>;

pub struct ShallowHandler<T: ?Sized> {
    mutated: bool,
    phantom: PhantomData<T>,
}

impl<T: ?Sized> GeneralHandler for ShallowHandler<T> {
    type Target = T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self {
            mutated: false,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(_value: &mut T) -> Self {
        Self {
            mutated: false,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn deref_mut(&mut self) {
        self.mutated = true;
    }
}

impl<T: ?Sized> ReplaceHandler for ShallowHandler<T> {
    #[inline]
    fn flush_replace(&mut self, _value: &T) -> bool {
        self.mutated
    }
}

impl<T: ?Sized> DebugHandler for ShallowHandler<T> {
    const NAME: &'static str = "ShallowObserver";
}

macro_rules! impl_shallow_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Observer<'ob, S, D>
                    = ShallowObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }
        )*
    };
}

impl_shallow_observe! {
    str,
}
