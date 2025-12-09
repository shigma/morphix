//! Types and traits for observing mutations to data structures.
//!
//! This module provides the core observation infrastructure for morphix, including:
//!
//! - **Core traits**: [`Observe`] and [`Observer`] define the observation protocol
//! - **General-purpose observers**: [`GeneralObserver`], handler trait [`GeneralHandler`] for
//!   implementing custom detection strategies, and pre-configured types: [`ShallowObserver`],
//!   [`SnapshotObserver`], [`NoopObserver`]
//! - **Specialized observers**: Type-specific implementations for common types, such as [`String`]
//!   and [`Vec<T>`]
//!
//! ## How Observers Work
//!
//! Observers are types that implement [`DerefMut`](std::ops::DerefMut) to the target type being
//! observed. When any method requiring `&mut self` is called, it triggers the
//! [`DerefMut`](std::ops::DerefMut) hook where change tracking occurs. Additionally, for specific
//! methods like [`String::push_str`] and [`Vec::push`], observers provide specialized
//! implementations for more precise tracking.
//!
//! For types that already implement [`Deref`](std::ops::Deref) (like [`Box<T>`]), implementing
//! observers is more challenging. If type `A` dereferences to `B`, and we have corresponding
//! observers `A'` and `B'`, where should `A'` deref to?
//!
//! - If `A'` → `A` → `B`: Changes on B cannot be precisely tracked (because no `B'` in the
//!   dereference chain)
//! - If `A'` → `B'` → `B`: Properties and methods on A become inaccessible (because no `A` in the
//!   dereference chain)
//!
//! To solve this, we use a [`ObserverPointer`] type to create the dereference chain: `A'` → `B'` →
//! `ObserverPointer<A>` → `A` → `B`. This allows tracking changes on both `A` and `B`.
//!
//! ## Usage
//!
//! Most users will interact with this module through attributes like `#[morphix(shallow)]` for
//! field-level control. Direct use of types from this module is typically only needed for advanced
//! use cases.

use crate::helper::{AsDeref, AsDerefMut, AsDerefMutCoinductive, AsNormalized, Unsigned, Zero};
use crate::{Adapter, Mutation};

mod general;
mod noop;
mod pointer;
mod r#ref;
mod shallow;
mod snapshot;

pub use general::{DebugHandler, GeneralHandler, GeneralObserver, SerializeHandler};
pub use noop::NoopObserver;
pub use pointer::ObserverPointer;
pub use r#ref::{RefObserve, RefObserver};
pub use shallow::ShallowObserver;
pub use snapshot::{SnapshotObserver, SnapshotSpec};

/// A trait for types that can be observed for mutations.
///
/// Types implementing [`Observe`] can be wrapped in [`Observer`]s that track mutations. The trait
/// is typically derived using the `#[derive(Observe)]` macro and used in `observe!` macros.
///
/// A single type `T` may have many possible [`Observer<'ob, Target = T>`] implementations in
/// theory, each with different change-tracking strategies. The [`Observe`] trait selects one
/// of these as the default observer to be used by `#[derive(Observe)]` and other generic code
/// that needs an observer for `T`.
///
/// When you `#[derive(Observe)]` on a struct, the macro requires that each field type
/// implements [`Observe`] so it can select an appropriate default observer for that field.
/// The [`Observer`] associated type of each field's [`Observe`] implementation determines which
/// observer will be instantiated in the generated code.
///
/// ## Example
///
/// ```
/// use morphix::adapter::Json;
/// use morphix::{Observe, observe};
/// use serde::Serialize;
///
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     field: String,
/// }
///
/// let mut data = MyStruct { field: "value".to_string() };
/// let Json(mutation) = observe!(data => {
///     // Mutations through observer are tracked
///     data.field.push_str(" modified");
/// }).unwrap();
/// ```
pub trait Observe {
    /// Associated observer type.
    ///
    /// This associated type specifies the *default* observer implementation for the type, when used
    /// in contexts where an [`Observe`] implementation is required.
    type Observer<'ob, S, D>: Observer<'ob, Head = S, InnerDepth = D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    /// Associated specification type for this observable.
    ///
    /// The [`Spec`](Observe::Spec) associated type is used as a marker to select specialized
    /// implementations of observers in certain contexts. For most types, this will be
    /// [`DefaultSpec`], but types can specify alternative specs to enable specialized
    /// observation strategies.
    ///
    /// ## Usage
    ///
    /// One important use of [`Spec`](Observe::Spec) is to select the appropriate observer
    /// implementation for wrapper types like [`Option<T>`]:
    ///
    /// - [`DefaultSpec`] → use [`OptionObserver`](crate::impls::option::OptionObserver) wrapping
    ///   `T`'s observer
    /// - [`SnapshotSpec`] → use [`SnapshotObserver<Option<T>>`] for snapshot-based change detection
    ///
    /// This allows [`Option<T>`] to automatically inherit more accurate or efficient change
    /// detection strategies based on its element type, without requiring manual implementation.
    type Spec;
}

/// Extension trait providing ergonomic methods for types implementing [`Observe`].
///
/// This trait is automatically implemented for all types that implement [`Observe`] and provides a
/// convenient way to create observers without needing to specify type parameters.
///
/// ## Example
///
/// ```
/// use morphix::observe::ObserveExt;
///
/// let mut data = 42;
/// let ob = data.__observe();
/// ```
pub trait ObserveExt: Observe {
    /// Creates an observer for this value.
    ///
    /// This is a convenience method that calls [`Observer::observe`] with the appropriate type
    /// parameters automatically inferred.
    #[inline]
    fn __observe<'ob>(&'ob mut self) -> Self::Observer<'ob, Self, Zero> {
        Observer::observe(self)
    }
}

impl<T: Observe + ?Sized> ObserveExt for T {}

/// A trait for observer types that wrap and track mutations to values.
///
/// Observers provide transparent access to the underlying value while recording any mutations that
/// occur. They form a dereference chain that allows multiple levels of observation.
///
/// ## Type Parameters
///
/// - [`Head`](Observer::Head): The type stored in the internal [`ObserverPointer`], representing
///   the head of the dereference chain
/// - [`InnerDepth`](Observer::InnerDepth): Type-level number indicating how many times
///   [`Head`](Observer::Head) must be dereferenced to reach the observed type
///
/// See the [module documentation](self) for more details about how observers work with dereference
/// chains.
pub trait Observer<'ob>: Sized
where
    Self: AsNormalized<Target = ObserverPointer<Self::Head>>,
    Self: AsDerefMutCoinductive<Self::OuterDepth>,
{
    /// Type-level number of dereferences from [`Head`](Observer::Head) to the observed type.
    type InnerDepth: Unsigned;

    /// The head type of the dereference chain.
    type Head: AsDeref<Self::InnerDepth> + ?Sized + 'ob;

    /// Creates an uninitialized observer.
    ///
    /// The returned observer is not associated with any value and must be initialized via
    /// [`observe`](Observer::observe) or [`force`](Observer::force) before use. Attempting to
    /// dereference an uninitialized observer results in *undefined behavior*.
    ///
    /// ## Use Cases
    ///
    /// This method is useful for:
    /// - Pre-allocating observer storage in containers before values are known
    /// - Creating placeholder observers that will be lazily initialized
    fn uninit() -> Self;

    /// Creates a new observer for the given value.
    ///
    /// This is the primary way to create an observer. The observer will track all mutations to the
    /// provided value.
    ///
    /// ## Example
    ///
    /// ```
    /// use morphix::observe::{Observer, ShallowObserver};
    ///
    /// let mut value = 42;
    /// let observer = ShallowObserver::<i32>::observe(&mut value);
    /// ```
    fn observe(value: &'ob mut Self::Head) -> Self;

    /// Refreshes the observer's internal pointer after the observed value has moved.
    ///
    /// This method updates the observer's internal pointer to point to the new location
    /// of the observed value. It is necessary when the observed value is relocated in
    /// memory (e.g., due to [`Vec`] reallocation) while the observer remains active.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that:
    /// 1. `this` was properly initialized via [`observe`](Observer::observe) or
    ///    [`force`](Observer::force)
    /// 2. `value` refers to the same logical value with which the observer was initialized, just
    ///    potentially at a new memory location
    ///
    /// ## Example
    ///
    /// Implementing [`Observer`] for [`Option<T>`]:
    ///
    /// ```
    /// # use morphix::helper::{AsDerefMut, AsNormalized, Succ, Unsigned, Zero};
    /// # use morphix::observe::{Observer, ObserverPointer};
    /// # use std::marker::PhantomData;
    /// #
    /// pub struct OptionObserver<'ob, O, S: ?Sized, N = Zero> {
    ///     ptr: ObserverPointer<S>,
    ///     mutated: bool,
    ///     ob: Option<O>,
    ///     phantom: PhantomData<&'ob mut N>,
    /// }
    ///
    /// # impl<'ob, O, S: ?Sized, N> Default for OptionObserver<'ob, O, S, N> {
    /// #    fn default() -> Self { todo!() }
    /// # }
    /// #
    /// # impl<'ob, O, S: ?Sized, N> std::ops::Deref for OptionObserver<'ob, O, S, N> {
    /// #     type Target = ObserverPointer<S>;
    /// #     fn deref(&self) -> &Self::Target { &self.ptr }
    /// # }
    /// #
    /// # impl<'ob, O, S: ?Sized, N> std::ops::DerefMut for OptionObserver<'ob, O, S, N> {
    /// #     fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ptr }
    /// # }
    /// #
    /// # impl<'ob, O, S: ?Sized, N> AsNormalized for OptionObserver<'ob, O, S, N> {
    /// #     type OuterDepth = Succ<Zero>;
    /// # }
    /// #
    /// impl<'ob, O, S: ?Sized, N> Observer<'ob> for OptionObserver<'ob, O, S, N>
    /// where
    ///     N: Unsigned,
    ///     S: AsDerefMut<N, Target = Option<O::Head>> + 'ob,
    ///     O: Observer<'ob, InnerDepth = Zero>,
    ///     O::Head: Sized,
    /// {
    ///     # type InnerDepth = N;
    ///     # type Head = S;
    ///     #
    ///     unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
    ///         // Refresh the outer pointer
    ///         ObserverPointer::set(Self::as_ptr(this), value);
    ///
    ///         // Refresh nested observer if present
    ///         match (&mut this.ob, value.as_deref_mut()) {
    ///             (Some(inner), Some(value)) => unsafe { Observer::refresh(inner, value) },
    ///             (None, None) => {}
    ///             _ => unreachable!("inconsistent observer state"),
    ///         }
    ///     }
    ///     #
    ///     # fn uninit() -> Self { todo!() }
    ///     # fn observe(value: &'ob mut Self::Head) -> Self { todo!() }
    /// }
    /// ```
    ///
    /// ## When to Call
    ///
    /// This method should be called after any operation that may relocate the observed
    /// value in memory while the observer is still in use.
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head);

    /// Forces the observer into a valid state for the given value.
    ///
    /// This method ensures the observer is properly associated with the observed value,
    /// regardless of its current state:
    /// - If the observer is uninitialized, it initializes the observer via
    ///   [`observe`](Observer::observe).
    /// - If the observer is already initialized but points to a different address, it updates the
    ///   pointer via [`refresh`](Observer::refresh).
    /// - If the observer already points to the same address, it does nothing.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that, if the observer is already initialized, `value` must refer to
    /// the same logical value that was passed to [`observe`](Observer::observe) when the observer
    /// was created, just potentially at a new memory location.
    ///
    /// Note: If the observer is uninitialized, no safety requirements apply since
    /// [`observe`](Observer::observe) will be called to initialize it.
    ///
    /// ## Use Cases
    ///
    /// This method is particularly useful in container observers (like
    /// [`VecObserver`](crate::impls::vec::VecObserver)) where element observers may need to be:
    /// - Lazily initialized on first access, and
    /// - Refreshed after container reallocation moves elements in memory.
    unsafe fn force(this: &mut Self, value: &'ob mut Self::Head) {
        match ObserverPointer::get(Self::as_ptr(this)) {
            None => *this = Self::observe(value),
            Some(ptr) => {
                if !std::ptr::addr_eq(ptr, value) {
                    // SAFETY: The observer was previously initialized via `observe`, and the caller
                    // guarantees that `value` refers to the same logical value.
                    unsafe { Self::refresh(this, value) }
                }
            }
        }
    }

    /// Gets a reference to the internal pointer.
    ///
    /// This is primarily used internally by observer implementations.
    #[inline]
    fn as_ptr(this: &Self) -> &ObserverPointer<Self::Head> {
        this.as_deref_coinductive()
    }

    /// Gets a mutable reference to the inner observed value without triggering observation.
    ///
    /// This method bypasses the entire observer chain, directly accessing the observed value
    /// through the internal pointer. No [`DerefMut`](std::ops::DerefMut) hooks are triggered,
    /// making this useful for internal operations that shouldn't be recorded as mutations.
    ///
    /// ## Usage
    ///
    /// This method is primarily used internally by observer implementations when they need
    /// to perform operations on the observed value without recording them as changes.
    #[inline]
    fn as_inner<'i>(this: &Self) -> &'i mut <Self::Head as AsDeref<Self::InnerDepth>>::Target
    where
        'ob: 'i,
        Self::Head: AsDerefMut<Self::InnerDepth>,
    {
        let head = unsafe { ObserverPointer::as_mut(Self::as_ptr(this)) };
        AsDerefMut::<Self::InnerDepth>::as_deref_mut(head)
    }

    /// Gets a mutable reference to the inner observed value while triggering observation.
    ///
    /// This method traverses the entire dereference chain, triggering
    /// [`DerefMut`](std::ops::DerefMut) hooks on all observers in the chain. This ensures that
    /// any mutations through the returned reference are properly tracked by all relevant
    /// observers.
    ///
    /// ## Usage
    ///
    /// Use this method when you need to access the inner value in a way that should be recorded as
    /// a potential mutation, such as when implementing specialized methods on observers.
    ///
    /// ## Example
    ///
    /// Implementing [`Vec::pop`] for a [`VecObserver`](crate::impls::vec::VecObserver):
    ///
    /// ```ignore
    /// impl VecObserver<'ob> {
    ///     pub fn pop(&mut self) -> Option<T> {
    ///         if self.as_deref().len() > self.initial_len() {
    ///             // If the current length exceeds the initial length, the pop operation can be
    ///             // expressed by `MutationKind::Append`, so we do not trigger full mutation.
    ///             Observer::as_inner(self).pop()
    ///         } else {
    ///             // Otherwise, we need to treat the pop operation as `MutationKind::Replace`.
    ///             Observer::track_inner(self).pop()
    ///         }
    ///     }
    /// }
    /// ```
    #[inline]
    fn track_inner<'i>(this: &mut Self) -> &'i mut <Self::Head as AsDeref<Self::InnerDepth>>::Target
    where
        'ob: 'i,
        Self::Head: AsDerefMut<Self::InnerDepth>,
    {
        let head = unsafe { ObserverPointer::as_mut(this.as_deref_mut_coinductive()) };
        AsDerefMut::<Self::InnerDepth>::as_deref_mut(head)
    }
}

/// Trait for observers that can serialize their recorded mutations.
///
/// This trait extends [`Observer`] with the ability to collect and serialize mutations using a
/// specific [`Adapter`].
pub trait SerializeObserver<'ob>: Observer<'ob> {
    /// Flushes and serializes all recorded mutations (unsafe version).
    ///
    /// This method extracts all recorded mutations, serializes them using the specified adapter,
    /// and clears the internal mutation state. After calling this method, the observer continues
    /// tracking new mutations from a clean state.
    ///
    /// ## Safety
    ///
    /// This method assumes the observer contains a valid (non-null) pointer. Calling this on an
    /// uninitialized observer results in *undefined behavior*. Most users should call
    /// [`flush`](SerializeObserver::flush) instead, which includes a null pointer check.
    ///
    /// ## Implementation Notes
    ///
    /// Implementations can safely use [`Deref`](std::ops::Deref) and
    /// [`DerefMut`](std::ops::DerefMut) to access the observed value, as this method is only
    /// called when the observer contains a valid pointer. The observer's [`Deref`](std::ops::Deref)
    /// and [`DerefMut`](std::ops::DerefMut) implementations are guaranteed to be safe when
    /// `flush_unchecked` is called.
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error>;

    /// Flushes and serializes all recorded mutations using the specified adapter.
    ///
    /// This method extracts all recorded mutations, serializes them, and resets the observer's
    /// internal state. After calling this method, the observer begins tracking mutations afresh,
    /// meaning an immediate subsequent call to `flush` will return `Ok(None)`.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(mutation))`: The serialized mutations if any were recorded
    /// - `Ok(None)`: If no mutations were recorded, or if the observer is uninitialized
    /// - `Err`: If serialization fails
    ///
    /// ## Example
    ///
    /// ```
    /// use morphix::adapter::Json;
    /// use morphix::observe::{ObserveExt, Observer, SerializeObserverExt, ShallowObserver};
    ///
    /// // Normal usage
    /// let mut value = String::from("Hello");
    /// let mut ob = value.__observe();
    /// ob += " world";
    ///
    /// // First flush returns the recorded mutation
    /// let Json(mutation) = ob.flush().unwrap();
    /// assert!(mutation.is_some());
    ///
    /// // Immediate second flush returns None (state was cleared)
    /// let Json(mutation) = ob.flush().unwrap();
    /// assert!(mutation.is_none());
    ///
    /// // Safe handling of uninitialized observer
    /// let mut empty: ShallowObserver<i32> = ShallowObserver::uninit();
    /// let Json(mutation) = empty.flush().unwrap();
    /// assert!(mutation.is_none());
    /// ```
    fn flush<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        if ObserverPointer::is_null(Self::as_ptr(this)) {
            return Ok(None);
        }
        unsafe { Self::flush_unchecked::<A>(this) }
    }
}

/// Extension trait providing ergonomic methods for [`SerializeObserver`].
///
/// This trait is automatically implemented for all types that implement [`SerializeObserver`] and
/// provides convenient methods that don't require turbofish syntax.
pub trait SerializeObserverExt<'ob>: SerializeObserver<'ob> {
    /// Collects mutations using the specified adapter.
    ///
    /// This is a convenience method that calls [`SerializeObserver::flush`].
    #[inline]
    fn flush<A: Adapter>(&mut self) -> Result<A, A::Error> {
        SerializeObserver::flush::<A>(self).map(A::from_mutation)
    }
}

impl<'ob, T: SerializeObserver<'ob>> SerializeObserverExt<'ob> for T {}

/// Default observation specification.
///
/// [`DefaultSpec`] indicates that no special observation behavior is required for the type. For
/// most types, this means they use their standard [`Observer`] implementation. For example, if `T`
/// implements [`Observe`] with `Spec = DefaultSpec`, then [`Option<T>`] will be observed using
/// [`OptionObserver`](crate::impls::option::OptionObserver) which wraps `T`'s observer.
///
/// All `#[derive(Observe)]` implementations use [`DefaultSpec`] unless overridden with field
/// attributes.
pub struct DefaultSpec;

#[doc(hidden)]
pub type DefaultObserver<'ob, T, S = T, D = Zero> = <T as Observe>::Observer<'ob, S, D>;
