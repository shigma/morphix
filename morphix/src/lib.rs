#![doc = include_str!("../README.md")]

mod adapter;
mod batch;
mod change;
mod error;
mod observe;

pub use adapter::Adapter;
#[cfg(feature = "json")]
pub use adapter::json::JsonAdapter;
#[cfg(feature = "yaml")]
pub use adapter::yaml::YamlAdapter;
pub use batch::Batch;
pub use change::{Change, Operation};
pub use error::ChangeError;
#[cfg(feature = "derive")]
pub use morphix_derive::{Observe, observe};
pub use observe::{Mutation, MutationObserver, Observe, Observer, ShallowObserver};
