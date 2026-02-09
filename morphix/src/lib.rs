#![cfg_attr(docsrs, allow(internal_features))]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(docsrs, feature(rustdoc_internals))]
#![allow(rustdoc::private_intra_doc_links)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

pub mod adapter;
mod batch;
pub mod builtin;
mod error;
pub mod helper;
pub mod impls;
mod mutation;
pub mod observe;
mod path;

pub use adapter::Adapter;
pub use batch::BatchTree;
pub use error::MutationError;
#[cfg(feature = "derive")]
pub use morphix_derive::{Observe, observe};
pub use mutation::{Mutation, MutationKind, Mutations};
pub use observe::Observe;
pub use path::{Path, PathSegment};
