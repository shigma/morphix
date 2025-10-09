use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

use crate::observe::{GeneralHandler, GeneralObserver};

/// A general observer that uses hash comparison to detect changes.
///
/// `HashObserver` computes and stores a hash of the initial value, then compares it
/// with the final value's hash to detect changes. This can be more efficient than
/// full value comparison for large structures.
///
/// ## Requirements
///
/// The observed type must implement [`Hash`].
///
/// ## Derive Usage
///
/// Can be used via the `#[observe(hash)]` attribute in derive macros:
///
/// ```
/// # use morphix::Observe;
/// # use serde::Serialize;
/// # #[derive(Serialize, Hash)]
/// # struct LargeConfig;
/// #[derive(Serialize, Hash, Observe)]
/// struct MyStruct {
///     #[observe(hash)]
///     config: LargeConfig,    // Large struct where hashing is cheaper than cloning
/// }
/// ```
///
/// # When to Use
///
/// `HashObserver` is suitable when:
/// 1. The type implements [`Hash`] and can be hashed efficiently
/// 2. The value may change frequently (so that [`ShallowObserver`](super::ShallowObserver) would
///    become less efficient or yield false positives)
/// 3. Hash computation is cheaper than cloning and comparison
///
/// ## Limitations
///
/// 1. **Hash collisions**: Different values might have the same hash (though rare)
/// 2. **Performance**: For small types, hashing might be slower than
///    [`ShallowObserver`](super::ShallowObserver)
pub type HashObserver<'i, T, H = DefaultHasher> = GeneralObserver<'i, T, HashHandler<H>>;

#[derive(Default)]
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
