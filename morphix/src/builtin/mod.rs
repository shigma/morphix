//! Built-in observation strategies.
//!
//! ## Usage
//!
//! Most users will interact with this module through attributes like `#[morphix(shallow)]` for
//! field-level control. Direct use of types from this module is typically only needed for advanced
//! use cases.

mod general;
mod noop;
mod pointer;
mod shallow;
pub(crate) mod snapshot;

pub use general::{DebugHandler, GeneralHandler, GeneralObserver, ReplaceHandler, SerializeHandler};
pub use noop::NoopObserver;
pub use pointer::PointerObserver;
pub use shallow::ShallowObserver;
pub use snapshot::{Snapshot, SnapshotObserver};
