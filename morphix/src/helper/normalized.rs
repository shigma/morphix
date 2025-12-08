//! Enabling consistent operations between observers and normal references in
//! [`observe!`](crate::observe!) macro via autoref-based specialization.
//!
//! ## Background
//!
//! Rust doesn't allow overloading certain operators, which creates problems for observers:
//!
//! ### Assignment Problem
//!
//! When you write `observer.field = value`, you want to assign to the observed field, not replace
//! the observer itself. While [`DerefMut`](std::ops::DerefMut) handles most operations, it doesn't
//! work for direct assignment due to Rust's assignment semantics.
//!
//! ### Comparison Problem
//!
//! For [`PartialEq`], the following two implementations would conflict:
//!
//! 1. `Observer<T>: PartialEq<U> where T: PartialEq<U>`
//! 2. `Observer<T>: PartialEq<Observer<U>> where T: PartialEq<U>`
//!
//! This means observers cannot naturally support both comparing with raw values and comparing with
//! other observers.
//!
//! ## Autoref-based Specialization
//!
//! This module uses autoref-based specialization to solve these problems. The technique exploits
//! Rust's method resolution rules: when calling a method on a value of type `T`, the compiler first
//! looks for methods on `&T`, then `&&T`, then `&&&T`, and so on (similarly for `&mut T`).
//!
//! By implementing a trait for both `&T` (or `&mut T`) and observer types, we can make the observer
//! implementation take precedence when called on an observer, while the reference implementation
//! handles regular values.
//!
//! ### Assignment Solution
//!
//! The [`observe!`](crate::observe!) macro transforms assignment expressions:
//!
//! ```text
//! // User writes:
//! value = 42;
//!
//! // Macro transforms to:
//! *(&mut value).as_normalized_mut() = 42;
//! ```
//!
//! - For normal values: `&mut T` returns `&mut T`, becoming `*(&mut value) = 42`
//! - For observers: returns `&mut Target`, properly dereferencing through the observer chain
//!
//! ### Comparison Solution
//!
//! The [`observe!`](crate::observe!) macro transforms comparison expressions:
//!
//! ```text
//! // User writes:
//! lhs == rhs;
//!
//! // Macro transforms to:
//! *(&lhs).as_normalized_ref() == *(&rhs).as_normalized_ref();
//! ```
//!
//! - For normal values: `&T` returns `&T`, becoming `*(&lhs) == *(&rhs)`
//! - For observers: returns `&Target`, comparing the underlying observed values
//!
//! This creates a form of specialization without requiring the unstable specialization feature.

use crate::helper::{AsDerefCoinductive, AsDerefMutCoinductive, Unsigned, Zero};

/// A trait for specifying normalized dereference access.
///
/// This trait indicates how many times a value should be dereferenced to reach its normalized form.
/// In the [`observe!`](crate::observe!) macro, values implementing this trait will be automatically
/// dereferenced [`OuterDepth`](AsNormalized::OuterDepth) times when they appear in the following
/// positions:
///
/// - Left-hand side of assignment operator (`=`)
/// - Both sides of comparison operators (`==`, `!=`, `<`, `<=`, `>`, `>=`)
///
/// See the [module documentation](self) for background on why this is necessary.
///
/// ## Implementation Notes
///
/// 1. **Every type implementing [`Observer`](crate::observe::Observer) should manually implement
///    `AsNormalized`**. Without this implementation, assignments and comparisons in the
///    [`observe!`](crate::observe!) macro may not work as expected, potentially causing compilation
///    errors or incorrect behavior. We cannot provide a blanket implementation `impl<T: Observer>
///    AsNormalized for T` because it would conflict with the `impl<T> AsNormalized for &T` and
///    `impl<T> AsNormalized for &mut T` implementations.
///
/// 2. **Do not implement `AsNormalized` for types other than `&T`, `&mut T`, and
///    [`Observer`](crate::observe::Observer) types**. Implementing `AsNormalized` for other
///    [`Deref`](std::ops::Deref) types (like [`Box`], [`MutexGuard`](std::sync::MutexGuard), etc.)
///    may cause unexpected behavior in the [`observe!`](crate::observe!) macro, as it would
///    interfere with the autoref-based specialization mechanism.
pub trait AsNormalized: AsDerefCoinductive<Self::OuterDepth> {
    type OuterDepth: Unsigned;

    /// Returns a normalized reference to the underlying value.
    fn as_normalized_ref(&self) -> &Self::Target {
        self.as_deref_coinductive()
    }

    /// Returns a normalized mutable reference to the underlying value.
    fn as_normalized_mut(&mut self) -> &mut Self::Target
    where
        Self: AsDerefMutCoinductive<Self::OuterDepth>,
    {
        self.as_deref_mut_coinductive()
    }
}

impl<T: ?Sized> AsNormalized for &T {
    type OuterDepth = Zero;
}

impl<T: ?Sized> AsNormalized for &mut T {
    type OuterDepth = Zero;
}
