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
//! the observer itself. While [`DerefMut`] handles most operations, it doesn't work for direct
//! assignment due to Rust's assignment semantics.
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
//! ```
//! # use morphix::helper::QuasiObserver;
//! # let mut value = 0u32;
//! // User writes:
//! value = 42;
//!
//! // Macro transforms to:
//! *(&mut value).observed_mut() = 42;
//! # assert_eq!(value, 42);
//! ```
//!
//! - For normal values: `&mut T` calls [`observed_mut`](QuasiObserver::observed_mut), which returns
//!   `&mut T` via autoref (one layer added, one removed by deref), and the assignment writes
//!   through.
//! - For observers: calls the same method, which traverses the dereference chain and returns a
//!   mutable reference to the underlying observed value.
//!
//! ### Comparison Solution
//!
//! The [`observe!`](crate::observe!) macro transforms comparison expressions:
//!
//! ```
//! # use morphix::helper::QuasiObserver;
//! # let lhs = 0u32;
//! # let rhs = 42u32;
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

/// Enables [`observed_mut`](QuasiObserver::observed_mut) to reach the [`Pointer`] without
/// triggering [`DerefMut`] on any observer layer.
///
/// The default implementation uses
/// [`as_deref_mut_coinductive`](AsDerefMutCoinductive::as_deref_mut_coinductive) followed by
/// [`deref_mut`](DerefMut::deref_mut), which is the standard chain traversal. The key
/// specialization is for [`Pointer<S>`](Pointer), which overrides this to use immutable coinductive
/// traversal followed by unsafe interior-mutable access, completely bypassing all [`DerefMut`]
/// hooks.
pub trait DerefMutUntracked: DerefMut {
    /// Traverses the coinductive dereference chain to reach the underlying value without triggering
    /// any observer [`DerefMut`] hooks, if possible.
    #[inline]
    fn deref_mut_untracked<'a, U, D>(this: &'a mut U) -> &'a mut Self::Target
    where
        Self: 'a,
        D: Unsigned,
        U: AsDerefMutCoinductive<D, Target = Self> + ?Sized,
    {
        this.as_deref_mut_coinductive().deref_mut()
    }
}

impl<T: ?Sized> DerefMutUntracked for &mut T {}

impl<S: ?Sized> DerefMutUntracked for Pointer<S> {
    #[inline]
    fn deref_mut_untracked<'a, U, D>(this: &'a mut U) -> &'a mut Self::Target
    where
        Self: 'a,
        D: Unsigned,
        U: AsDerefMutCoinductive<D, Target = Self> + ?Sized,
    {
        unsafe { Pointer::as_mut(this.as_deref_coinductive()) }
    }
}

/// A trait that unifies observers and plain references for the [`observe!`](crate::observe!) macro.
///
/// Both real observers and ordinary references (`&T`, `&mut T`) need to participate in the
/// assignment and comparison transformations performed by the [`observe!`](crate::observe!) macro
/// (see the [module documentation](self) for details). This trait provides a common interface for
/// both: the macro calls [`observed_mut`](QuasiObserver::observed_mut) and
/// [`observed_ref`](QuasiObserver::observed_ref) on all values uniformly, and autoref-based method
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
///    [`Deref`] types (like [`Box`], [`MutexGuard`](std::sync::MutexGuard), etc.) may cause
///    unexpected behavior in the [`observe!`](crate::observe!) macro, as it would interfere with
///    the autoref-based specialization mechanism.
pub trait QuasiObserver: AsDerefMutCoinductive<Self::OuterDepth, Target: Deref<Target = Self::Head>> {
    /// The type stored inside the [`Pointer`], from which the inductive dereference chain begins.
    ///
    /// For plain references (`&T`, `&mut T`) and [`Pointer<T>`] this is `T`. For observers, this
    /// is the head type parameter `S` (or `O::Head` for deref-mode structs that delegate through
    /// an inner observer).
    type Head: AsDeref<Self::InnerDepth> + ?Sized;

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
    /// [`Deref`] chain on the head type (e.g., `Vec<T>` → `[T]`).
    type InnerDepth: Unsigned;

    /// Returns an immutable reference to the observed value by traversing the full dereference
    /// chain.
    ///
    /// The [`observe!`](crate::observe!) macro calls this method on both sides of comparison
    /// operators. For plain references this is a no-op identity; for observers it dereferences
    /// through the observer chain to reach the underlying value.
    #[inline]
    fn observed_ref<T: ?Sized>(&self) -> &T
    where
        Self::Head: AsDeref<Self::InnerDepth, Target = T>,
    {
        self.as_deref_coinductive().deref().as_deref()
    }

    /// Resets all granular tracking state in this observer.
    ///
    /// Called by [`observed_mut`](Self::observed_mut) before traversing the dereference chain, and
    /// by parent observers to cascade invalidation to their children. After this call, the next
    /// flush should produce a [`Replace`](crate::MutationKind::Replace) mutation.
    ///
    /// For plain references (`&T`, `&mut T`) and [`Pointer<T>`], this is a no-op. For observers,
    /// it delegates to [`ObserverState::invalidate`] on the internal tracking state and/or
    /// recursively invalidates child observers.
    fn invalidate(this: &mut Self);

    /// Returns a mutable reference to the observed value by traversing the full dereference chain.
    ///
    /// Traverses the coinductive outer chain (triggering [`DerefMut`] hooks), then the inductive
    /// inner chain to reach the final observed value. The [`observe!`](crate::observe!) macro
    /// transforms assignment expressions (`lhs = rhs`) into `*(&mut lhs).observed_mut() = rhs`.
    #[inline]
    fn observed_mut<T: ?Sized>(&mut self) -> &mut T
    where
        Self::Target: DerefMutUntracked,
        Self::Head: AsDerefMut<Self::InnerDepth, Target = T>,
    {
        Self::invalidate(self);
        DerefMutUntracked::deref_mut_untracked(self).as_deref_mut()
    }

    /// Returns a mutable reference to the inner observed value **without** triggering observation.
    ///
    /// Unlike [`observed_mut`](QuasiObserver::observed_mut), this method bypasses the
    /// [`DerefMut`](std::ops::DerefMut) chain, so no mutation is recorded. Use this when the
    /// observer will emit a more specific [`MutationKind`](crate::MutationKind) (e.g., append or
    /// truncate) for the operation.
    ///
    /// ## Example
    ///
    /// Implementing [`Vec::pop`] for a [`VecObserver`](crate::impls::VecObserver):
    ///
    /// ```ignore
    /// impl VecObserver {
    ///     pub fn pop(&mut self) -> Option<T> {
    ///         if self.as_deref().len() > self.initial_len() {
    ///             // If the current length exceeds the initial length, the pop operation can be
    ///             // expressed by `MutationKind::Append`, so we do not trigger full mutation.
    ///             self.untracked_mut().pop()
    ///         } else {
    ///             // Otherwise, we need to treat the pop operation as `MutationKind::Replace`.
    ///             self.observed_mut().pop()
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    fn untracked_mut<T: ?Sized>(&mut self) -> &mut T
    where
        Self::Target: DerefMutUntracked,
        Self::Head: AsDerefMut<Self::InnerDepth, Target = T>,
    {
        DerefMutUntracked::deref_mut_untracked(self).as_deref_mut()
    }
}

impl<T: ?Sized> QuasiObserver for &T {
    type Head = T;
    type OuterDepth = Zero;
    type InnerDepth = Zero;

    fn invalidate(_: &mut Self) {}
}

impl<T: ?Sized> QuasiObserver for &mut T {
    type Head = T;
    type OuterDepth = Zero;
    type InnerDepth = Zero;

    fn invalidate(_: &mut Self) {}
}

impl<T: ?Sized> QuasiObserver for Pointer<T> {
    type Head = T;
    type OuterDepth = Zero;
    type InnerDepth = Zero;

    fn invalidate(this: &mut Self) {
        let base = this as *const _ as *const u8;
        let value = unsafe { Pointer::as_ref(this) };
        for &(offset, invalidate) in &this.states {
            unsafe { invalidate(base.offset(offset) as *mut u8, value) }
        }
    }
}

/// A trait for types that carry observer-internal state requiring invalidation.
///
/// When a tail observer's [`DerefMut`] is triggered (fallback invalidation), all registered
/// [`ObserverState`] implementors are invalidated via this trait. The
/// [`invalidate`](Self::invalidate) method resets tracking state so that the next flush produces a
/// [`Replace`](crate::MutationKind::Replace) mutation.
///
/// The method is named `invalidate` rather than `mark_replace` to avoid coupling with
/// [`MutationKind`](crate::MutationKind) — it invalidates the tracking mechanism, and the
/// resulting `Replace` mutation is a consequence, not the intent.
pub trait ObserverState {
    /// The observed value type that this state tracks.
    type Target: ?Sized;

    /// Invalidates all granular tracking state.
    ///
    /// After this call, the next flush should produce a [`Replace`](crate::MutationKind::Replace)
    /// mutation covering the entire observed value. The post-invalidation state is **not** the
    /// "initial" state (which would be the clean state right after `observe`), but rather a state
    /// that signals "all granular tracking is lost."
    fn invalidate(this: &mut Self, value: &Self::Target);
}
