use std::ptr::NonNull;

use crate::builtin::{DebugHandler, GeneralHandler, GeneralObserver, ReplaceHandler};
use crate::helper::{AsDeref, Unsigned, Zero};
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
pub type PointerObserver<'ob, S, D, E = Zero> =
    GeneralObserver<'ob, PointerHandler<<S as AsDeref<D>>::Target, E>, S, D>;

pub struct PointerHandler<T, E = Zero>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    ptr: Option<NonNull<T::Target>>,
}

impl<T, E> GeneralHandler for PointerHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    type Target = T;
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self { ptr: None }
    }

    #[inline]
    fn observe(value: &T) -> Self {
        Self {
            ptr: Some(NonNull::from(value.as_deref())),
        }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T, E> ReplaceHandler for PointerHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    #[inline]
    fn flush_replace(&mut self, value: &T) -> bool {
        !std::ptr::eq(
            value.as_deref(),
            self.ptr
                .expect("Pointer should not be null in GeneralHandler::flush")
                .as_ptr(),
        )
    }
}

impl<T, E> DebugHandler for PointerHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    E: Unsigned,
{
    const NAME: &'static str = "PointerObserver";
}
