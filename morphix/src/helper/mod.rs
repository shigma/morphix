//! Helper utilities for internal implementation details.
//!
//! This module contains traits and types that support morphix's internal machinery. These are
//! implementation details and should not be used directly in most cases.
//!
//! ## Contents
//!
//! - [`Assignable`] - Enables assignment operations on observers via autoref-based specialization
//!
//! ## Stability
//!
//! Items in this module are considered internal implementation details and may change between minor
//! versions without notice. Use at your own risk.

use std::ops::DerefMut;

use crate::{Observe, Observer};

pub mod deref;
pub mod pointer;
pub mod unsigned;

pub use deref::{DerefCoinductive, DerefInductive, DerefMutCoinductive, DerefMutInductive};
pub use pointer::Pointer;
pub use unsigned::{Succ, Unsigned, Zero};

/// A trait enabling assignment to observers using autoref-based specialization.
///
/// ## Background
///
/// Rust doesn't allow overloading the assignment operator (`=`). This creates a problem for
/// observers: when you write `observer.field = value`, you want to assign to the observed field,
/// not replace the observer itself. While [`DerefMut`] handles most operations, it doesn't work for
/// direct assignment due to Rust's assignment semantics.
///
/// ## Autoref-based Specialization
///
/// `Assignable` uses a technique called autoref-based specialization to solve this:
///
/// 1. The trait provides a method [`__deref_mut`](Assignable::__deref_mut) with a default
///    implementation
/// 2. We implement it for `&mut T` (all mutable references)
/// 3. We also implement it for each [`Observer`](crate::Observer) type
/// 4. The [`observe!`](crate::observe) macro automatically rewrites assignment expressions:
///
/// ```
/// # use morphix::helper::Assignable;
/// # let mut value = 0i32;
/// // User writes:
/// value = 42;
///
/// // Macro transforms to:
/// *(&mut value).__deref_mut() = 42;
/// ```
///
/// This transformation ensures assignments work correctly for both regular fields and observed
/// fields without requiring different syntax:
/// - For normal values: calls `&mut T` impl, effectively becoming `*(&mut left) = right`
/// - For observers: calls the observer's impl, properly dereferencing through the observer
///
/// This creates a form of specialization without requiring the unstable specialization feature.
///
/// ## Implementation Notes
///
/// 1. **Every type implementing [`Observer`](crate::Observer) should manually implement
///    `Assignable`**. Without this implementation, assignments in the [`observe!`](crate::observe)
///    macro may not work as expected, potentially causing compilation errors or incorrect behavior.
///    We cannot provide a blanket implementation `impl<T: Observer> Assignable for T` because it
///    would conflict with the `impl<T> Assignable for &mut T` implementation.
///
/// 2. **Do not implement `Assignable` for types other than `&mut T` and
///    [`Observer`](crate::Observer) types**. Implementing `Assignable` for other [`DerefMut`] types
///    (like [`Box`], [`MutexGuard`](std::sync::MutexGuard), etc.) may cause unexpected behavior in
///    the [`observe!`](crate::observe) macro, as it would interfere with the autoref-based
///    specialization mechanism.
///
/// ## Example
///
/// Implement `Assignable` for a custom observer type:
///
/// ```
/// # use morphix::helper::Assignable;
/// # struct MyObserver<'i, T>(&'i mut T);
/// # impl<'i, T> std::ops::Deref for MyObserver<'i, T> {
/// #     type Target = T;
/// #     fn deref(&self) -> &Self::Target { &self.0 }
/// # }
/// # impl<'i, T> std::ops::DerefMut for MyObserver<'i, T> {
/// #     fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
/// # }
/// impl<'i, T> Assignable for MyObserver<'i, T> {
///     // Uses the default implementation which calls DerefMut::deref_mut
/// }
/// ```
pub trait Assignable: DerefMut {
    /// Internal method for assignment operations. The default implementation simply calls
    /// [`DerefMut::deref_mut`].
    ///
    /// **Do not call directly**. This method is automatically used by the
    /// [`observe!`](crate::observe) macro.
    #[doc(hidden)]
    fn __deref_mut(&mut self) -> &mut Self::Target {
        self.deref_mut()
    }
}

impl<T> Assignable for &mut T {}

pub(crate) trait RangeLike<T>: std::ops::RangeBounds<T> {}

impl<T> RangeLike<T> for std::ops::RangeFull {}
impl<T> RangeLike<T> for std::ops::RangeTo<T> {}
impl<T> RangeLike<T> for std::ops::RangeFrom<T> {}
impl<T> RangeLike<T> for std::ops::Range<T> {}
impl<T> RangeLike<T> for std::ops::RangeInclusive<T> {}
impl<T> RangeLike<T> for std::ops::RangeToInclusive<T> {}

#[doc(hidden)]
pub trait ObserveExt: Observe {
    #[inline]
    fn __observe<'i>(&'i mut self) -> Self::Observer<'i, Self, Zero> {
        Self::Observer::<'i, Self, Zero>::observe(self)
    }
}

impl<T: Observe> ObserveExt for T {}
