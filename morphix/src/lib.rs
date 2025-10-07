#![doc = include_str!("../README.md")]

mod adapter;
mod batch;
mod change;
mod error;
mod observe;

pub use adapter::Adapter;
pub use adapter::json::JsonAdapter;
pub use adapter::observe::ObserveAdapter;
pub use batch::Batch;
pub use change::{Change, Operation};
pub use error::ChangeError;
#[cfg(feature = "derive")]
pub use morphix_derive::{Observe, observe};
pub use observe::{Context, Ob, Observe, Observer};
