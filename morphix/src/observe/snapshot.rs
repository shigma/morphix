use crate::Observe;
use crate::observe::{GeneralHandler, GeneralObserver};

/// A general observer that uses snapshot comparison to detect actual value changes.
///
/// `SnapshotObserver` creates a clone of the initial value and compares it with the
/// final value using `PartialEq`. This provides accurate change detection by comparing
/// actual values rather than tracking access patterns.
///
/// ## Requirements
///
/// The observed type must implement:
/// - [`Clone`] - for creating the snapshot
/// - [`PartialEq`] - for comparing values
///
/// ## Derive Usage
///
/// Can be used via the `#[observe(snapshot)]` attribute in derive macros:
///
/// ```
/// # use morphix::Observe;
/// # use serde::Serialize;
/// # #[derive(Clone, PartialEq)]
/// # struct Uuid;
/// # #[derive(Clone, PartialEq)]
/// # struct BitFlags;
/// #[derive(Serialize, Clone, PartialEq, Observe)]
/// struct MyStruct {
///     #[observe(snapshot)]
///     id: Uuid,  // Cheap to clone and compare
///     #[observe(snapshot)]
///     flags: BitFlags,  // Small Copy type
/// }
/// ```
///
/// ## When to Use
///
/// `SnapshotObserver` is ideal when:
/// 1. The type implements [`Clone`] and [`PartialEq`] with low cost
/// 2. Values may be modified and then restored to original (so that
///    [`ShallowObserver`](super::ShallowObserver) would yield false positives)
///
/// ## Built-in Usage
///
/// All primitive types ([`i32`], [`f64`], [`bool`], etc.) use `SnapshotObserver` as their default
/// implementation since they're cheap to clone and compare.
pub type SnapshotObserver<'i, T> = GeneralObserver<'i, T, SnapshotHandler<T>>;

pub struct SnapshotHandler<T> {
    snapshot: T,
}

impl<T: Clone + PartialEq> GeneralHandler<T> for SnapshotHandler<T> {
    fn on_observe(value: &mut T) -> Self {
        Self {
            snapshot: value.clone(),
        }
    }

    fn on_deref_mut(&mut self) {}

    fn on_collect(&self, value: &T) -> bool {
        &self.snapshot != value
    }
}

macro_rules! impl_observe {
    ($($ty:ty $(=> $target:ty)?),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Observer<'i> = SnapshotObserver<'i, $ty>
                where
                    Self: 'i;
            }
        )*
    };
}

impl_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool,
}
