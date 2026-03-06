//! Types and traits for observing mutations to data structures.
//!
//! See the [Observer Mechanism](https://github.com/shigma/morphix#observer-mechanism) section in
//! the README for a detailed overview of the observer architecture, dereference chains, and
//! mutation tracking primitives.

pub use crate::builtin::snapshot::SnapshotSpec;
use crate::helper::{AsDeref, Pointer, QuasiObserver, Unsigned, Zero};
use crate::{Adapter, Mutations};

/// A trait for types that can be observed for mutations.
///
/// Types implementing [`Observe`] can be wrapped in [`Observer`]s that track mutations. The trait
/// is typically derived using the `#[derive(Observe)]` macro and used in `observe!` macros.
///
/// A single type `T` may have many possible [`Observer<'ob, Target = T>`] implementations in
/// theory, each with different change-tracking strategies. The [`Observe`] trait selects one
/// of these as the *default* observer to be used by `#[derive(Observe)]` and other generic code
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
///     data.field.push_str(" modified");
/// }).unwrap();
/// ```
pub trait Observe {
    /// The default observer implementation for this type.
    type Observer<'ob, S, D>: Observer<Head = S, InnerDepth = D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    /// Marker type for selecting specialized observer implementations in wrapper types.
    ///
    /// For most types, this will be [`DefaultSpec`]. Types can specify [`SnapshotSpec`] to enable
    /// snapshot-based observation strategies. For example, [`Option<T>`] uses
    /// [`OptionObserver`](crate::impls::OptionObserver) when `T::Spec = DefaultSpec`, but
    /// [`SnapshotObserver`](crate::builtin::SnapshotObserver) when `T::Spec = SnapshotSpec`.
    type Spec;
}

/// Counterpart to [`Observe`] for reference types.
///
/// A type `T` implements [`RefObserve`] if `&T` can be observed. Analogous to the relationship
/// between [`UnwindSafe`](std::panic::UnwindSafe) and
/// [`RefUnwindSafe`](std::panic::RefUnwindSafe).
///
/// See also: [`Observe`].
pub trait RefObserve {
    /// The default observer implementation for `&Self`.
    type Observer<'ob, S, D>: Observer<Head = S, InnerDepth = D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    /// Marker type for selecting specialized observer implementations in wrapper types.
    ///
    /// For most types, this will be [`DefaultSpec`]. Types can specify [`SnapshotSpec`] to enable
    /// snapshot-based observation strategies. For example, [`Option<T>`] uses
    /// [`OptionObserver`](crate::impls::OptionObserver) when `T::Spec = DefaultSpec`, but
    /// [`SnapshotObserver`](crate::builtin::SnapshotObserver) when `T::Spec = SnapshotSpec`.
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
/// ## Lifecycle
///
/// - [`uninit()`](Self::uninit) creates an uninitialized observer (null pointer). Used for
///   pre-allocating storage in containers before values are known.
/// - [`observe(head)`](Self::observe) fully initializes the observer: sets up the internal
///   pointer, initializes diff state, and registers any fallback invalidation entries.
/// - [`refresh(this, head)`](Self::refresh) updates the internal pointer after the observed
///   value has moved in memory (e.g., due to [`Vec`] reallocation), keeping diff state intact.
/// - [`force(this, head)`](Self::force) is a convenience: initializes if null, refreshes if
///   moved, no-ops if unchanged. Used by container observers for lazy initialization.
///
/// ## Inline-Field Invariant
///
/// Every [`Observer`]'s [`Deref`](std::ops::Deref) target must be an inline field (or nested
/// inline field) — no [`Box`], [`Arc`](std::sync::Arc), or other heap indirection in the deref
/// chain. This ensures that every field within the observer hierarchy has a **fixed byte offset**
/// relative to the [`Pointer<Head>`](Pointer), invariant under moves. This property is required
/// by [`Pointer`]'s [fallback invalidation](Pointer#fallback-invalidation) mechanism, which uses
/// offset-based addressing to reach sibling observers and states.
///
/// See the [Observer Mechanism](https://github.com/shigma/morphix#observer-mechanism) for a
/// detailed overview of the dereference chain and mutation tracking primitives.
pub trait Observer: QuasiObserver<Target = Pointer<<Self as QuasiObserver>::Head>> + Sized {
    /// Creates an uninitialized observer with a null pointer.
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
    fn observe(head: &Self::Head) -> Self;

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
    /// 2. `head` refers to the same logical value with which the observer was initialized, just
    ///    potentially at a new memory location
    ///
    /// ## Use Cases
    ///
    /// This method should be called after any operation that may relocate the observed
    /// value in memory while the observer is still in use.
    unsafe fn refresh(this: &mut Self, head: &Self::Head);

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
    /// The caller must ensure that, if the observer is already initialized, `head` must refer to
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
    unsafe fn force(this: &mut Self, head: &Self::Head) {
        match Pointer::get((*this).as_deref_coinductive()) {
            None => *this = Self::observe(head),
            Some(ptr) => {
                if !std::ptr::addr_eq(ptr.as_ptr(), head) {
                    // SAFETY: The observer was previously initialized via `observe`, and the caller
                    // guarantees that `head` refers to the same logical value.
                    unsafe { Self::refresh(this, head) }
                }
            }
        }
    }
}

/// Extends [`Observer`] with the ability to flush recorded mutations as serializable values.
///
/// This trait uses type-erased serialization: mutation values are stored as
/// [`Box<dyn erased_serde::Serialize>`](erased_serde::Serialize) and only serialized when an
/// [`Adapter`] converts them.
pub trait SerializeObserver: Observer {
    /// Extracts all recorded mutations and fully resets internal state.
    ///
    /// After calling `flush`, the observer's state is fully reset: an immediately subsequent
    /// `flush` with no intervening mutations must return empty. This invariant applies
    /// recursively to all nested observers and handler types.
    ///
    /// **Replace collapse**: If all inner fields or elements of a composite observer report
    /// [`Replace`](crate::MutationKind::Replace), the observer should collapse them into a
    /// single whole-value [`Replace`](crate::MutationKind::Replace). This applies to structs,
    /// tuples, arrays, and slices.
    ///
    /// ## Safety
    ///
    /// The observer must contain a valid pointer.
    unsafe fn flush(this: &mut Self) -> Mutations;

    /// Flushes mutations for a `#[serde(flatten)]` field.
    ///
    /// Returns `(mutations, is_replace)` where `is_replace` indicates whether the observer's
    /// entire content was replaced. When `is_replace` is true, the returned mutations contain
    /// per-field [`Replace`](crate::MutationKind::Replace) mutations (a flattened decomposition),
    /// not a single root-level [`Replace`](crate::MutationKind::Replace). This is the opposite
    /// of [`Replace`](crate::MutationKind::Replace) collapse: even when the whole value is
    /// replaced, the result is broken apart into individual field mutations so they can be
    /// merged into the parent's mutation set.
    ///
    /// The parent struct uses the `is_replace` flag to decide whether all of its fields
    /// (including this flattened one) were replaced, and if so, collapses everything into a
    /// single whole-struct [`Replace`](crate::MutationKind::Replace).
    ///
    /// The default implementation panics. Only struct observers (generated by the derive macro),
    /// map observers, and wrapper observers that delegate to an inner observer (e.g.,
    /// [`DerefObserver`](crate::impls::DerefObserver),
    /// [`CowObserver`](crate::impls::CowObserver),
    /// [`NewtypeObserver`](crate::impls::NewtypeObserver)) implement this method.
    ///
    /// ## Safety
    ///
    /// Same as [`flush`](Self::flush).
    #[inline]
    unsafe fn flat_flush(_this: &mut Self) -> (Mutations, bool) {
        panic!("flat_flush can only be called on structs and maps")
    }
}

/// Extension trait providing ergonomic methods for [`SerializeObserver`].
///
/// This trait is automatically implemented for all types that implement [`SerializeObserver`] and
/// provides convenient methods that don't require turbofish syntax.
pub trait SerializeObserverExt: SerializeObserver {
    /// Collects mutations using the specified adapter.
    ///
    /// This is a convenience method for [`SerializeObserver::flush`].
    #[inline]
    fn flush<A: Adapter>(&mut self) -> Result<A, A::Error> {
        A::from_mutations(unsafe { SerializeObserver::flush(self) })
    }

    /// Collects flattened mutations using the specified adapter.
    ///
    /// This is a convenience method for [`SerializeObserver::flat_flush`].
    #[inline]
    fn flat_flush<A: Adapter>(&mut self) -> Result<A, A::Error> {
        A::from_mutations(unsafe { SerializeObserver::flat_flush(self) }.0)
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
