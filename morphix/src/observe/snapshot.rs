use crate::Observe;
use crate::observe::{GeneralHandler, GeneralObserver};

/// An observer that detects changes by comparing snapshots.
///
/// Unlike [`ShallowObserver`](super::ShallowObserver) which tracks any
/// [`DerefMut`](std::ops::DerefMut) access as a mutation, `SnapshotObserver` creates an initial
/// snapshot of the value and only reports mutation if the final value actually differs from the
/// snapshot.
///
/// This observer is ideal for:
/// - Small, cheaply cloneable types (e.g., `Uuid`, `DateTime`, small enums)
/// - Types where [`DerefMut`] might be called without actual modification
/// - Cases where you only care about actual value changes, not access patterns
///
/// ## Requirements
///
/// The observed type must implement:
/// - [`Clone`] - for creating the snapshot (should be cheap)
/// - [`PartialEq`] - for comparing the final value with the snapshot
/// - [`Serialize`] - for generating the mutation
///
/// ## Example
///
/// ```
/// use morphix::{JsonAdapter, Observe, Observer, observe};
/// use serde::Serialize;
/// use uuid::Uuid;
///
/// #[derive(Clone, PartialEq, Serialize, Observe)]
/// struct Config {
///     #[observe(snapshot)]
///     id: Uuid,
///     #[observe(snapshot)]
///     status: Status,
/// }
///
/// #[derive(Clone, PartialEq, Serialize)]
/// enum Status {
///     Active,
///     Inactive,
/// }
///
/// let mut config = Config {
///     id: Uuid::new_v4(),
///     status: Status::Active,
/// };
///
/// let mutation = observe!(JsonAdapter, |mut config| {
///     // `DerefMut` is called but value doesn't change
///     config.status = Status::Active;
/// }).unwrap();
///
/// assert_eq!(mutation, None); // No mutation because value didn't change
/// ```
///
/// ## Performance Considerations
///
/// SnapshotObserver is most efficient when:
/// - The type is cheap to clone (e.g., [`Copy`] types, small structs)
/// - The type is cheap to compare (e.g., simple equality checks)
/// - Changes are relatively rare compared to access
///
/// For large or expensive-to-clone types, consider using [ShallowObserver](super::ShallowObserver)
/// or implementing a custom [Observe] trait.
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
