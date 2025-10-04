#![doc = include_str!("../README.md")]

mod batch;
mod change;
mod delta;
mod error;
mod observe;
mod operation;

pub use change::Operation;
pub use delta::{Delta, DeltaComposer, DeltaState};
pub use error::UmiliError;
pub use observe::{Context, Ob, Observe};
#[cfg(feature = "derive")]
pub use umili_derive::{Observe, observe};
