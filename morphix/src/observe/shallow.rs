use crate::observe::{GeneralHandler, GeneralObserver};

/// A general observer that tracks any mutation access as a change.
///
/// `ShallowObserver` uses a simple boolean flag to track whether [`DerefMut`](std::ops::DerefMut)
/// has been called, treating any mutable access as a change. This makes it extremely efficient with
/// minimal overhead.
///
/// ## Derive Usage
///
/// Can be used via the `#[observe(shallow)]` attribute in derive macros:
///
/// ```
/// # use morphix::Observe;
/// # use serde::Serialize;
/// # #[derive(Serialize)]
/// # struct ExternalType;
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     #[observe(shallow)]
///     external_data: ExternalType,    // ExternalType doesn't implement Observe
/// }
/// ```
///
/// ## When to Use
///
/// Despite its limitations, `ShallowObserver` is usually the best choice for external types that
/// don't implement the `Observe` trait, as the performance benefits typically outweigh
/// the occasional false positive.
///
/// ## Limitations
///
/// 1. **False positives on round-trip changes**: If a value is modified and then restored to its
///    original value, it's still reported as changes.
/// 2. **False positives on non-semantic changes**: Operations that don't affect serialization (like
///    [`Vec::reserve`]) are still reported as changes.
pub type ShallowObserver<'i, T> = GeneralObserver<'i, T, ShallowHandler>;

#[derive(Default)]
pub struct ShallowHandler {
    mutated: bool,
}

impl<T> GeneralHandler<T> for ShallowHandler {
    #[inline]
    fn on_observe(_value: &mut T) -> Self {
        Self { mutated: false }
    }

    #[inline]
    fn on_deref_mut(&mut self) {
        self.mutated = true;
    }

    #[inline]
    fn on_collect(&self, _value: &T) -> bool {
        self.mutated
    }
}
