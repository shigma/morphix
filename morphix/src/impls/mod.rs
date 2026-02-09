//! Observer implementations for library types.
//!
//! This module provides specialized [`Observer`](crate::observe::Observer) implementations
//! for common library types. These observers enable precise mutation tracking tailored to each
//! type's semantics.
//!
//! ## Usage
//!
//! These observers are typically used automatically through the [`Observe`](crate::Observe)
//! trait implementations. Direct usage is rarely needed unless implementing custom observers.

mod array;
mod atomic;
mod btree_map;
mod deref;
mod hash_map;
mod option;
mod slice;
mod string;
mod tuple;
mod unsize;
mod vec;

pub use array::ArrayObserver;
pub use btree_map::BTreeMapObserver;
pub use hash_map::HashMapObserver;
pub use option::OptionObserver;
pub use slice::SliceObserver;
pub use string::StringObserver;
pub use vec::VecObserver;
