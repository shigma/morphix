use std::ptr::NonNull;

use crate::builtin::{DebugHandler, GeneralHandler, GeneralObserver, ReplaceHandler};
use crate::helper::AsDeref;
use crate::observe::DefaultSpec;

/// A general observer implementation for reference types.
///
/// This observer stores the initial pointer value and compares it with the current value at
/// collection time using [`std::ptr::eq`]. A change is detected if the reference now points to a
/// different memory location.
///
/// ## Limitations
///
/// - **False negatives**: If the referenced value contains interior mutability and is mutated
///   without changing the pointer, the mutation will not be detected.
/// - **False positives**: If two distinct references point to equal values, changing from one to
///   the other will be detected as a change, even if the underlying value is effectively the same.
///
/// ## When to Use
///
/// Use [`PointerObserver`] for types where:
/// 1. Pointer identity is a reliable indicator of value identity
/// 2. Value comparison is expensive or unavailable
/// 3. The type has no interior mutability
///
/// For types where value comparison is cheap and preferred, consider using
/// [`SnapshotObserver`](crate::builtin::SnapshotObserver) for references.
pub type PointerObserver<'ob, S, D> = GeneralObserver<'ob, PointerHandler<<S as AsDeref<D>>::Target>, S, D>;

pub struct PointerHandler<T: ?Sized> {
    ptr: Option<NonNull<T>>,
}

impl<T: ?Sized> GeneralHandler for PointerHandler<T> {
    type Target = T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self { ptr: None }
    }

    #[inline]
    fn observe(value: &T) -> Self {
        Self {
            ptr: Some(NonNull::from(value)),
        }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T: ?Sized> ReplaceHandler for PointerHandler<T> {
    #[inline]
    fn flush_replace(&mut self, value: &T) -> bool {
        !std::ptr::eq(
            value,
            self.ptr
                .expect("Pointer should not be null in GeneralHandler::flush")
                .as_ptr(),
        )
    }
}

impl<T: ?Sized> DebugHandler for PointerHandler<T> {
    const NAME: &'static str = "PointerObserver";
}
