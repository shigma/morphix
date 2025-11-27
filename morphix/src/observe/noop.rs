use crate::helper::Zero;
use crate::observe::{DebugHandler, DefaultSpec, GeneralHandler, GeneralObserver};

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

#[derive(Default)]
pub struct NoopHandler;

impl<T> GeneralHandler<T> for NoopHandler {
    type Spec = DefaultSpec;

    #[inline]
    fn on_observe(_value: &mut T) -> Self {
        Self
    }

    #[inline]
    fn on_deref_mut(&mut self) {}

    #[inline]
    fn on_collect(&self, _value: &T) -> bool {
        false
    }
}

impl<T> DebugHandler<T> for NoopHandler {
    const NAME: &'static str = "NoopObserver";
}
