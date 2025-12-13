use crate::helper::{AsDeref, Succ};
use crate::observe::general::ReplaceHandler;
use crate::observe::{DebugHandler, DefaultSpec, GeneralHandler, GeneralObserver};

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
/// Use [`RefObserver`] for types where:
/// 1. Pointer identity is a reliable indicator of value identity
/// 2. Value comparison is expensive or unavailable
/// 3. The type has no interior mutability
///
/// For types where value comparison is cheap and preferred, consider using [`SnapshotObserver`] for
/// references.
pub type RefObserver<'a, 'ob, S, D> = GeneralObserver<'ob, RefHandler<'a, <S as AsDeref<Succ<D>>>::Target>, S, D>;

pub struct RefHandler<'a, T: ?Sized> {
    ptr: Option<&'a T>,
}

impl<'a, T: ?Sized> GeneralHandler for RefHandler<'a, T> {
    type Target = &'a T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self { ptr: None }
    }

    #[inline]
    fn observe(value: &&'a T) -> Self {
        Self { ptr: Some(value) }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<'a, T: ?Sized> ReplaceHandler for RefHandler<'a, T> {
    #[inline]
    fn flush_replace(&mut self, value: &&'a T) -> bool {
        !std::ptr::eq(
            *value,
            self.ptr.expect("Pointer should not be null in GeneralHandler::flush"),
        )
    }
}

impl<'a, T: ?Sized> DebugHandler for RefHandler<'a, T> {
    const NAME: &'static str = "RefHandler";
}
