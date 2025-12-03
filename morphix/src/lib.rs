#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(rustdoc::private_intra_doc_links)]
#![doc = include_str!("../README.md")]

pub mod adapter;
mod batch;
mod error;
pub mod helper;
mod impls;
mod mutation;
pub mod observe;
mod path;

pub use adapter::Adapter;
pub use batch::BatchTree;
pub use error::MutationError;
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use morphix_derive::{Observe, observe};
pub use mutation::{Mutation, MutationKind};
pub use observe::Observe;
pub use path::{Path, PathSegment};
