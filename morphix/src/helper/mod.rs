//! Helper utilities for internal implementation details.
//!
//! This module contains traits and types that support morphix's internal machinery. These are
//! implementation details and should not be used directly in most cases.
//!
//! ## Contents
//!
//! - [`Assignable`] - Enables assignment operations on observers via autoref-based specialization
//! - [`Unsigned`], [`Zero`], [`Succ`] - Type-level natural numbers for compile-time depth tracking
//! - [`AsDeref`], [`AsDerefMut`] - Inductive recursive dereferencing
//! - [`AsDerefCoinductive`], [`AsDerefMutCoinductive`] - Coinductive recursive dereferencing
//!
//! ## Stability
//!
//! Items in this module are considered internal implementation details and may change between minor
//! versions without notice. Use at your own risk.

pub mod deref;
pub mod unsigned;

pub use deref::{AsDeref, AsDerefCoinductive, AsDerefMut, AsDerefMutCoinductive};
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
/// 1. The trait provides a method [`__assign`](Assignable::__assign) with a default implementation
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
/// (&mut value).__assign(42);
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
/// # use morphix::helper::{Assignable, Succ, Zero};
/// # struct MyStruct<'i, T>(&'i mut T);
/// # impl<'i, T> std::ops::Deref for MyStruct<'i, T> {
/// #     type Target = T;
/// #     fn deref(&self) -> &Self::Target { &self.0 }
/// # }
/// # impl<'i, T> std::ops::DerefMut for MyStruct<'i, T> {
/// #     fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
/// # }
/// impl<'i, T> Assignable for MyStruct<'i, T> {
///     // Uses the default implementation which calls `DerefMut::deref_mut`
///     type Depth = Zero;
/// }
/// ```
pub trait Assignable: AsDerefMutCoinductive<Succ<Self::Depth>, Target: Sized> {
    type Depth: Unsigned;

    /// Internal method for assignment operations.
    ///
    /// This method is automatically used by the [`observe!`](crate::observe) macro.
    #[doc(hidden)]
    fn __assign(&mut self, value: Self::Target) {
        *self.as_deref_mut_coinductive() = value;
    }
}

impl<T> Assignable for &mut T {
    type Depth = Zero;
}

// The impl below will conflict with `&mut T`, so we have to impl `Assignable` for every single
// `Observer` types.
// impl<'i, T: crate::observe::Observer<'i, UpperDepth = Zero>> Assignable for T {
//     type Depth = T::LowerDepth;
// }
