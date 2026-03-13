//! Observer implementations for collection types in [`std::collections`].

mod binary_heap;
pub mod btree_map;
pub mod hash_map;
mod hash_set;
#[cfg(feature = "indexmap")]
pub mod index_map;

pub use btree_map::BTreeMapObserver;
pub use hash_map::HashMapObserver;
#[cfg(feature = "indexmap")]
pub use index_map::IndexMapObserver;
