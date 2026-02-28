//! Types and traits for observing mutations to data structures.
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
//! To solve this, we use a [`Pointer`] type to create the dereference chain: `A'` → `B'` →
//! `Pointer<A>` → `A` → `B`. This allows tracking changes on both `A` and `B`.

pub use crate::builtin::snapshot::SnapshotSpec;
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Unsigned, Zero};
use crate::{Adapter, Mutations};

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
    type Observer<'ob, S, D>: Observer<Head = S, InnerDepth = D>
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
    /// - [`DefaultSpec`] → use [`OptionObserver`](crate::impls::OptionObserver) wrapping `T`'s
    ///   observer
    /// - [`SnapshotSpec`] → use [`SnapshotObserver<Option<T>>`](crate::builtin::SnapshotObserver)
    ///   for snapshot-based change detection
    ///
    /// This allows [`Option<T>`] to automatically inherit more accurate or efficient change
    /// detection strategies based on its element type, without requiring manual implementation.
    type Spec;
}

/// A trait for types whose references can be observed for mutations.
///
/// [`RefObserve`] provides observation capability for reference types. A type `T` implements
/// [`RefObserve`] if and only if `&T` implements [`Observe`]. This is analogous to the relationship
/// between [`UnwindSafe`](std::panic::UnwindSafe) and [`RefUnwindSafe`](std::panic::RefUnwindSafe).
///
/// A single type `T` may have many possible [`Observer<'ob, Target = &T>`] implementations in
/// theory, each with different change-tracking strategies. The [`RefObserve`] trait selects one
/// of these as the default observer to be used by `#[derive(Observe)]` and other generic code
/// that needs an observer for `&T`.
///
/// See also: [`Observe`].
pub trait RefObserve {
    /// The observer type for `&'a Self`.
    ///
    /// This associated type specifies the *default* observer implementation for the type, when used
    /// in contexts where an [`Observe`] implementation is required.
    type Observer<'ob, S, D>: Observer<Head = S, InnerDepth = D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    /// Specification type controlling nested reference observation behavior.
    ///
    /// This determines how `&&T`, `&&&T`, etc. are observed. See the [trait
    /// documentation](RefObserve) for available specs.
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

/// Extension trait providing untracked mutable access to the inner observed value.
///
/// This trait is automatically implemented for all types that implement [`QuasiObserver`] with a
/// [`Pointer`]-based target. It complements the methods on [`QuasiObserver`]:
///
/// | Method                                        | Receiver    | Triggers tracking |
/// | --------------------------------------------- | ----------- | ----------------- |
/// | [`observed_ref`](QuasiObserver::observed_ref) | `&self`     | No                |
/// | [`observed_mut`](QuasiObserver::observed_mut) | `&mut self` | Yes               |
/// | [`untracked_mut`](ObserverExt::untracked_mut) | `&mut self` | No                |
///
/// [`observed_ref`](QuasiObserver::observed_ref) and [`observed_mut`](QuasiObserver::observed_mut)
/// live on [`QuasiObserver`] because the [`observe!`](crate::observe!) macro needs to call them on
/// both observers and plain references. [`untracked_mut`](ObserverExt::untracked_mut) is
/// observer-specific and lives here.
///
/// ## Dereference Chain
///
/// An observer stores a [`Pointer<Head>`] internally. The [`Head`](ObserverExt::Head) type may
/// itself implement [`Deref`](std::ops::Deref), forming a chain that is traversed
/// [`InnerDepth`](QuasiObserver::InnerDepth) times to reach the final
/// [`Target`](ObserverExt::Target). For example, a [`VecObserver`](crate::impls::VecObserver)
/// has `Head = Vec<T>` and `Target = [T]`, with `InnerDepth = Succ<Zero>` (one dereference).
pub trait ObserverExt: QuasiObserver<Target = Pointer<Self::Head>> {
    /// The type stored inside the observer's [`Pointer`]. It can be dereferenced
    /// [`InnerDepth`](QuasiObserver::InnerDepth) times to reach [`Target`](ObserverExt::Target).
    type Head: AsDeref<Self::InnerDepth, Target = <Self as ObserverExt>::Target> + ?Sized;

    /// The observed type after fully dereferencing [`Head`](ObserverExt::Head).
    type Target: ?Sized;

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
    fn untracked_mut(&mut self) -> &mut <Self as ObserverExt>::Target
    where
        Self::Head: AsDerefMut<Self::InnerDepth>,
    {
        let head = unsafe { Pointer::as_mut((*self).as_deref_coinductive()) };
        AsDerefMut::<Self::InnerDepth>::as_deref_mut(head)
    }
}

impl<O, S: ?Sized, T: ?Sized> ObserverExt for O
where
    O: QuasiObserver<Target = Pointer<S>>,
    S: AsDeref<Self::InnerDepth, Target = T>,
{
    type Head = S;
    type Target = T;
}

/// A trait for observer types that wrap and track mutations to values.
///
/// Observers provide transparent access to the underlying value while recording any mutations that
/// occur. They form a dereference chain that allows multiple levels of observation.
///
/// See the [module documentation](self) for more details about how observers work with dereference
/// chains.
pub trait Observer: ObserverExt + Sized {
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
    /// use morphix::builtin::ShallowObserver;
    /// use morphix::observe::Observer;
    ///
    /// let mut value = 42;
    /// let observer = ShallowObserver::<i32>::observe(&mut value);
    /// ```
    fn observe(value: &Self::Head) -> Self;

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
    /// # use morphix::helper::{AsDeref, AsDerefMut, QuasiObserver, Pointer, Succ, Unsigned, Zero};
    /// # use morphix::observe::{Observer};
    /// # use std::marker::PhantomData;
    /// #
    /// pub struct OptionObserver<'ob, O, S: ?Sized, N = Zero> {
    ///     ptr: Pointer<S>,
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
    /// #     type Target = Pointer<S>;
    /// #     fn deref(&self) -> &Self::Target { &self.ptr }
    /// # }
    /// #
    /// # impl<'ob, O, S: ?Sized, N> std::ops::DerefMut for OptionObserver<'ob, O, S, N> {
    /// #     fn deref_mut(&mut self) -> &mut Self::Target { &mut self.ptr }
    /// # }
    /// #
    /// # impl<'ob, O, S: ?Sized, N> QuasiObserver for OptionObserver<'ob, O, S, N>
    /// # where
    /// #     N: Unsigned,
    /// #     S: AsDeref<N>,
    /// # {
    /// #     type OuterDepth = Succ<Zero>;
    /// #     type InnerDepth = N;
    /// # }
    /// #
    /// impl<'ob, O, S: ?Sized, N> Observer for OptionObserver<'ob, O, S, N>
    /// where
    ///     N: Unsigned,
    ///     S: AsDerefMut<N, Target = Option<O::Head>> + 'ob,
    ///     O: Observer<InnerDepth = Zero>,
    ///     O::Head: Sized,
    /// {
    ///     unsafe fn refresh(this: &mut Self, value: &Self::Head) {
    ///         // Refresh the outer pointer
    ///         Pointer::set(this, value);
    ///
    ///         // Refresh nested observer if present
    ///         match (&mut this.ob, value.as_deref()) {
    ///             (Some(inner), Some(value)) => unsafe { Observer::refresh(inner, value) },
    ///             (None, None) => {}
    ///             _ => unreachable!("inconsistent observer state"),
    ///         }
    ///     }
    ///     #
    ///     # fn uninit() -> Self { todo!() }
    ///     # fn observe(value: & Self::Head) -> Self { todo!() }
    /// }
    /// ```
    ///
    /// ## When to Call
    ///
    /// This method should be called after any operation that may relocate the observed
    /// value in memory while the observer is still in use.
    unsafe fn refresh(this: &mut Self, value: &Self::Head);

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
    /// [`VecObserver`](crate::impls::VecObserver)) where element observers may need to be:
    /// - Lazily initialized on first access, and
    /// - Refreshed after container reallocation moves elements in memory.
    unsafe fn force(this: &mut Self, value: &Self::Head) {
        match Pointer::get((*this).as_deref_coinductive()) {
            None => *this = Self::observe(value),
            Some(ptr) => {
                if !std::ptr::addr_eq(ptr.as_ptr(), value) {
                    // SAFETY: The observer was previously initialized via `observe`, and the caller
                    // guarantees that `value` refers to the same logical value.
                    unsafe { Self::refresh(this, value) }
                }
            }
        }
    }
}

/// Trait for observers that can serialize their recorded mutations.
///
/// This trait extends [`Observer`] with the ability to collect and serialize mutations using a
/// specific [`Adapter`].
pub trait SerializeObserver: Observer {
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
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error>;

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
    /// use morphix::builtin::ShallowObserver;
    /// use morphix::observe::{ObserveExt, Observer, SerializeObserverExt};
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
    fn flush<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        if Pointer::is_null((*this).as_deref_coinductive()) {
            return Ok(Mutations::new());
        }
        unsafe { Self::flush_unchecked::<A>(this) }
    }
}

/// Extension trait providing ergonomic methods for [`SerializeObserver`].
///
/// This trait is automatically implemented for all types that implement [`SerializeObserver`] and
/// provides convenient methods that don't require turbofish syntax.
pub trait SerializeObserverExt: SerializeObserver {
    /// Collects mutations using the specified adapter.
    ///
    /// This is a convenience method that calls [`SerializeObserver::flush`].
    #[inline]
    fn flush<A: Adapter>(&mut self) -> Result<A, A::Error> {
        SerializeObserver::flush::<A>(self).map(A::from_mutation)
    }
}

impl<T: SerializeObserver> SerializeObserverExt for T {}

/// Default observation specification.
///
/// [`DefaultSpec`] indicates that no special observation behavior is required for the type. For
/// most types, this means they use their standard [`Observer`] implementation. For example, if `T`
/// implements [`Observe`] with `Spec = DefaultSpec`, then [`Option<T>`] will be observed using
/// [`OptionObserver`](crate::impls::OptionObserver) which wraps `T`'s observer.
///
/// All `#[derive(Observe)]` implementations use [`DefaultSpec`] unless overridden with field
/// attributes.
pub struct DefaultSpec;

#[doc(hidden)]
pub type DefaultObserver<'ob, T, S = T, D = Zero> = <T as Observe>::Observer<'ob, S, D>;
