use crate::Observe;
use crate::helper::{AsDerefMut, Unsigned, Zero};
use crate::observe::general::ReplaceHandler;
use crate::observe::{DebugHandler, DefaultSpec, GeneralHandler, GeneralObserver, RefObserve, SnapshotSpec};

/// A general observer that never reports changes.
///
/// `NoopObserver` is a no-operation [`Observer`](crate::observe::Observer) that always returns
/// [`None`] when collecting changes, effectively ignoring all mutations to the observed value.
///
/// ## Derive Usage
///
/// Can be used via the `#[morphix(noop)]` attribute in derive macros:
///
/// ```
/// # use morphix::Observe;
/// # use serde::Serialize;
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     important_field: String,
///     #[morphix(noop)]
///     cache: String,      // Changes to cache are not tracked
/// }
/// ```
///
/// ## When to Use
///
/// Use `NoopObserver` for fields that:
/// - Are only used internally and not part of the public state
/// - Should not trigger change notifications.
pub type NoopObserver<'ob, S, D = Zero> = GeneralObserver<'ob, NoopHandler, S, D>;

pub struct NoopHandler;

impl<T: ?Sized> GeneralHandler<T> for NoopHandler {
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self
    }

    #[inline]
    fn observe(_value: &mut T) -> Self {
        Self
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T: ?Sized> ReplaceHandler<T> for NoopHandler {
    #[inline]
    fn flush_replace(&mut self, _value: &T) -> bool {
        false
    }
}

impl<T: ?Sized> DebugHandler<T> for NoopHandler {
    const NAME: &'static str = "NoopObserver";
}

impl Observe for () {
    type Observer<'ob, S, D>
        = NoopObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = SnapshotSpec;
}

impl RefObserve for () {
    type Observer<'a, 'ob, S, D>
        = NoopObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

    type Spec = SnapshotSpec;
}
