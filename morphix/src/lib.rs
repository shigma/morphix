#![doc = include_str!("../README.md")]

mod adapter;
mod batch;
mod error;
pub mod helper;
mod impls;
mod mutation;
pub mod observe;
mod path;

pub use adapter::Adapter;
#[cfg(feature = "json")]
pub use adapter::json::JsonAdapter;
#[cfg(feature = "yaml")]
pub use adapter::yaml::YamlAdapter;
pub use batch::BatchTree;
pub use error::MutationError;
#[cfg(feature = "derive")]
pub use morphix_derive::{Observe, observe};
pub use mutation::{Mutation, MutationKind};
pub use observe::{Observe, Observer};
pub use path::{Path, PathSegment};
