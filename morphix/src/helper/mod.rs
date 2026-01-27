//! Helper utilities for internal implementation details.
//!
//! This module contains traits and types that support morphix's internal machinery.
//!
//! ## Contents
//!
//! - [`Unsigned`], [`Zero`], [`Succ`] - Type-level natural numbers for compile-time depth tracking
//! - [`AsDeref`], [`AsDerefMut`] - Inductive recursive dereferencing
//! - [`AsDerefCoinductive`], [`AsDerefMutCoinductive`] - Coinductive recursive dereferencing
//! - [`AsNormalized`] - Enables consistent operations between observers and normal references via
//!   autoref-based specialization
//! - [`Pointer`] - Internal pointer type for observer dereference chains

pub mod deref;
pub(crate) mod macros;
pub mod normalized;
mod pointer;
pub mod unsigned;

pub use deref::{AsDeref, AsDerefCoinductive, AsDerefMut, AsDerefMutCoinductive};
pub use normalized::AsNormalized;
pub use pointer::Pointer;
pub use unsigned::{Succ, Unsigned, Zero};
