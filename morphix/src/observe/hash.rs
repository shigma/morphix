use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

use crate::observe::{GeneralHandler, GeneralObserver};

/// An observer that uses hashing for efficient change detection.
///
/// `HashObserver` computes a hash of the initial value and compares it with the hash of the final
/// value to detect changes. This is more efficient than full value comparison for large structures,
/// though it cannot detect the specific nature of the change.
///
/// ## Use Cases
///
/// This observer is ideal for:
/// - Large structures where full comparison is expensive
/// - Types that implement `Hash` but not `Clone` or where cloning is expensive
/// - Scenarios where you only need to know if something changed, not what changed
/// - Configuration objects with many fields
///
/// ## Limitations
///
/// - Only produces `Replace` mutations (cannot detect `Append` operations)
/// - Hash collisions are theoretically possible (though extremely rare)
/// - Requires recomputing the hash on collection
///
/// ## Example
///
/// ```
/// use std::collections::HashMap;
/// use morphix::{Observe, Observer, JsonAdapter};
///
/// #[derive(Serialize, Hash, Observe)]
/// struct LargeConfig {
///     #[observe(hash)]
///     data: Vec<u8>,  // Large binary data
/// }
///
/// let mut config = LargeConfig {
///     data: vec![0; 1024],
/// };
///
/// let mutation = observe!(JsonAdapter, |mut config| {
///     config.data[0] = 1;  // Modify the data
/// }).unwrap();
///
/// // Efficiently detected change without cloning the entire Vec
/// assert!(mutation.is_some());
/// ```
pub type HashObserver<'i, T, H = DefaultHasher> = GeneralObserver<'i, T, HashHandler<H>>;

pub struct HashHandler<H> {
    initial_hash: u64,
    phantom: PhantomData<H>,
}

impl<H: Hasher + Default> HashHandler<H> {
    fn hash<T: Hash>(value: &T) -> u64 {
        let mut hasher = H::default();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

impl<T: Hash, H: Hasher + Default> GeneralHandler<T> for HashHandler<H> {
    fn on_observe(value: &mut T) -> Self {
        Self {
            initial_hash: Self::hash(value),
            phantom: PhantomData,
        }
    }

    fn on_deref_mut(&mut self) {}

    fn on_collect(&self, value: &T) -> bool {
        self.initial_hash != Self::hash(value)
    }
}
