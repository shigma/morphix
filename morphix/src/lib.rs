#![doc = include_str!("../README.md")]

mod adapter;
mod batch;
mod error;
mod mutation;
mod observe;

pub use adapter::Adapter;
#[cfg(feature = "json")]
pub use adapter::json::JsonAdapter;
#[cfg(feature = "yaml")]
pub use adapter::yaml::YamlAdapter;
pub use batch::Batch;
pub use error::MutationError;
#[cfg(feature = "derive")]
pub use morphix_derive::{Observe, observe};
pub use mutation::{Mutation, MutationKind};
pub use observe::{MutationState, Observe, Observer, ShallowObserver, StatefulObserver};
