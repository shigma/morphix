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
//! *(&mut value).observed_mut() = 42;
//! ```
//!
//! - For normal values: `&mut T` returns `&mut &mut T`, becoming `**(&mut &mut value) = 42`
//! - For observers: returns [`&mut Pointer`](crate::helper::Pointer), properly dereferencing
//!   through the observer chain
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
//! *(&lhs).observed_ref() == *(&rhs).observed_ref();
//! ```
//!
//! - For normal values: `&T` returns `&&T`, becoming `**(&&lhs) == **(&&rhs)`
//! - For observers: returns [`&Pointer`](crate::helper::Pointer), comparing the underlying observed
//!   values
//!
//! This creates a form of specialization without requiring the unstable specialization feature.

use std::ops::{Deref, DerefMut};

use crate::helper::{AsDeref, AsDerefMut, AsDerefMutCoinductive, Pointer, Unsigned, Zero};

/// A trait that unifies observers and plain references for the [`observe!`](crate::observe!) macro.
///
/// Both real observers and ordinary references (`&T`, `&mut T`) need to participate in the
/// assignment and comparison transformations performed by the [`observe!`](crate::observe!) macro
/// (see the [module documentation](self) for details). This trait provides a common interface for
/// both: the macro calls [`observed_ref`](QuasiObserver::observed_ref) and
/// [`observed_mut`](QuasiObserver::observed_mut) on all values uniformly, and autoref-based method
/// resolution selects the correct implementation.
///
/// The name "quasi-observer" reflects that plain references are not real observers, but they behave
/// like ones for the purpose of the macro's transformations.
///
/// ## Dereference Chain
///
/// A `QuasiObserver` defines a two-segment dereference chain:
///
/// ```text
/// Self --[OuterDepth]-> Pointer<Head> --> Head --[InnerDepth]-> Target
///        (coinductive)                           (inductive)
/// ```
///
/// - [`OuterDepth`](QuasiObserver::OuterDepth): The number of coinductive dereferences from `Self`
///   to its internal [`Pointer`]. For a simple observer this is `Succ<Zero>` (one). For a composite
///   observer like [`VecObserver`](crate::impls::VecObserver) which wraps
///   [`SliceObserver`](crate::impls::SliceObserver), it is `Succ<Succ<Zero>>` (two). For `&T`,
///   `&mut T`, and [`Pointer<T>`] it is `Zero`.
///
/// - [`InnerDepth`](QuasiObserver::InnerDepth): The number of inductive dereferences from the
///   `Head` type (stored inside the [`Pointer`]) to the final observed type. For example, when
///   observing a [`Vec<T>`], the `Head` is [`Vec<T>`] and the observed type is `[T]`, so
///   `InnerDepth = Succ<Zero>`.
///
/// ## Implementation Notes
///
/// 1. **Every type implementing [`Observer`](crate::observe::Observer) should manually implement
///    [`QuasiObserver`]**. Without this implementation, assignments and comparisons in the
///    [`observe!`](crate::observe!) macro may not work as expected, potentially causing compilation
///    errors or incorrect behavior. We cannot provide a blanket implementation `impl<T: Observer>
///    QuasiObserver for T` because it would conflict with the `impl<T> QuasiObserver for &T` and
///    `impl<T> QuasiObserver for &mut T` implementations.
///
/// 2. **Do not implement [`QuasiObserver`] for types other than `&T`, `&mut T`, [`Pointer<T>`], and
///    [`Observer`](crate::observe::Observer) types**. Implementing [`QuasiObserver`] for other
///    [`Deref`](std::ops::Deref) types (like [`Box`], [`MutexGuard`](std::sync::MutexGuard), etc.)
///    may cause unexpected behavior in the [`observe!`](crate::observe!) macro, as it would
///    interfere with the autoref-based specialization mechanism.
pub trait QuasiObserver
where
    Self: AsDerefMutCoinductive<Self::OuterDepth>,
    Self::Target: Deref<Target: AsDeref<Self::InnerDepth>>,
{
    /// The number of coinductive dereferences from `Self` to its internal [`Pointer`].
    ///
    /// For plain references (`&T`, `&mut T`) and [`Pointer<T>`] this is [`Zero`]. For most
    /// observers this is [`Succ<Zero>`](crate::helper::Succ). Composite observers that wrap
    /// another observer (e.g., [`VecObserver`](crate::impls::VecObserver) wrapping
    /// [`SliceObserver`](crate::impls::SliceObserver)) have a higher depth.
    type OuterDepth: Unsigned;

    /// The number of inductive dereferences from the head type (stored inside the [`Pointer`]) to
    /// the final observed type.
    ///
    /// For plain references (`&T`, `&mut T`) and [`Pointer<T>`] this is [`Zero`]. For observers,
    /// this is typically the generic depth parameter `D`, which accounts for any
    /// [`Deref`](std::ops::Deref) chain on the head type (e.g., `Vec<T>` → `[T]`).
    type InnerDepth: Unsigned;

    /// Returns an immutable reference to the observed value by traversing the full dereference
    /// chain.
    ///
    /// The [`observe!`](crate::observe!) macro calls this method on both sides of comparison
    /// operators. For plain references this is a no-op identity; for observers it dereferences
    /// through the observer chain to reach the underlying value.
    #[inline]
    fn observed_ref(&self) -> &<<Self::Target as Deref>::Target as AsDeref<Self::InnerDepth>>::Target {
        self.as_deref_coinductive().deref().as_deref()
    }

    /// Returns a mutable reference to the observed value by traversing the full dereference chain.
    ///
    /// The [`observe!`](crate::observe!) macro calls this method on the left-hand side of
    /// assignment operators. For plain references this is a no-op identity; for observers it
    /// dereferences through the observer chain, triggering [`DerefMut`] hooks along the way.
    #[inline]
    fn observed_mut(&mut self) -> &mut <<Self::Target as Deref>::Target as AsDeref<Self::InnerDepth>>::Target
    where
        Self::Target: DerefMut<Target: AsDerefMut<Self::InnerDepth>>,
    {
        self.as_deref_mut_coinductive().deref_mut().as_deref_mut()
    }
}

impl<T: ?Sized> QuasiObserver for &T {
    type OuterDepth = Zero;
    type InnerDepth = Zero;
}

impl<T: ?Sized> QuasiObserver for &mut T {
    type OuterDepth = Zero;
    type InnerDepth = Zero;
}

impl<T: ?Sized> QuasiObserver for Pointer<T> {
    type OuterDepth = Zero;
    type InnerDepth = Zero;
}
