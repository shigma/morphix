use crate::observe::{GeneralHandler, GeneralObserver};

/// A generic observer that only tracks complete replacements.
///
/// `ShallowObserver` provides a basic observer implementation that treats any mutation through
/// [`DerefMut`](std::ops::DerefMut) as a complete replacement of the value. It does not track
/// internal mutations, making it suitable for:
///
/// 1. Primitive types (numbers, booleans, etc.) that cannot be partially modified
/// 2. Types where internal mutation tracking is not needed
/// 3. External types that do not implement `Observe`
///
/// ## Examples
///
/// Built-in implementation for primitive types:
///
/// ```
/// use morphix::{Observe, Observer, JsonAdapter};
///
/// let mut value = 42i32;
/// let mut observer = value.observe();  // ShallowObserver<i32>
/// *observer = 43;  // Recorded as a complete replacement
/// ```
///
/// Explicit usage via `#[observe(shallow)]` attribute:
///
/// ```
/// use morphix::Observe;
/// use serde::Serialize;
///
/// // External type that doesn't implement Observe
/// #[derive(Serialize)]
/// struct External;
///
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     #[observe(shallow)]
///     external: External,  // use ShallowObserver<External>
///     normal: String,      // use StringObserver
/// }
/// ```
///
/// ## Type Parameters
///
/// - `'i` - lifetime of the observed value
/// - `T` - type being observed
pub type ShallowObserver<'i, T> = GeneralObserver<'i, T, ShallowHandler>;

pub struct ShallowHandler {
    replaced: bool,
}

impl<T> GeneralHandler<T> for ShallowHandler {
    fn on_observe(_value: &mut T) -> Self {
        Self { replaced: false }
    }

    fn on_deref_mut(&mut self) {
        self.replaced = true;
    }

    fn on_collect(&self, _value: &T) -> bool {
        self.replaced
    }
}
