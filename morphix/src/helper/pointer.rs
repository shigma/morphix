use std::cell::Cell;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

use crate::helper::{AsNormalized, Zero};

/// An internal pointer type for observer dereference chains.
///
/// [`Pointer`] is a specialized pointer type used exclusively within observer implementations to
/// store references to observed values. It serves as a critical component in the
/// observer dereference chain, allowing multiple levels of observers to coexist while maintaining
/// access to the original value.
///
/// ## Purpose
///
/// When observing types that already implement [`Deref`] (like [`Vec<T>`]), we need a way to break
/// the dereference chain to insert observer logic at multiple levels. [`Pointer`] provides this
/// break point, enabling chains like: [`VecObserver`](crate::impls::VecObserver) →
/// [`SliceObserver`](crate::impls::SliceObserver) → [`Pointer<Vec<T>>`](Pointer) → [`Vec<T>`] →
/// [`[T]`](std::slice).
///
/// ## Safety
///
/// This type uses raw pointers internally and relies on several safety invariants:
///
/// 1. **Lifetime tracking**: The lifetime `'ob` in observers ensures the pointed-to value remains
///    valid for the observer's lifetime
/// 2. **Initialization**: Pointers must be properly initialized via [`new`](Pointer::new) before
///    dereferencing
/// 3. **Single ownership**: Each [`Pointer`] assumes exclusive access to its referenced value
///    during the observer's lifetime
///
/// ## Internal Use Only
///
/// This type is not intended for direct use outside of observer implementations. All safety
/// invariants are maintained by the observer infrastructure when used correctly within that
/// context.
pub struct Pointer<S: ?Sized>(Cell<Option<NonNull<S>>>);

impl<S: ?Sized> Pointer<S> {
    /// Create an uninitialized pointer.
    #[inline]
    pub const fn uninit() -> Self {
        Self(Cell::new(None))
    }

    /// Creates a new pointer from a mutable reference.
    ///
    /// The returned pointer will remain valid as long as the original reference remains valid,
    /// which is enforced by the lifetime parameter in observer types.
    #[inline]
    pub const fn new(value: &S) -> Self {
        Self(Cell::new(Some(NonNull::from_ref(value))))
    }

    /// Retrieves the internal raw pointer.
    #[inline]
    pub const fn get(this: &Self) -> Option<NonNull<S>> {
        this.0.get()
    }

    /// Updates the internal pointer to a new reference.
    ///
    /// This method is primarily used when observed collections (like [`Vec`]) reallocate their
    /// internal storage. When a vector grows and moves its elements to a new memory location,
    /// any existing [`Pointer`] instances pointing to those elements become invalid. This method
    /// allows updating those pointers to point to the elements' new locations.
    #[inline]
    pub fn set(this: &Self, value: &S) {
        this.0.set(Some(NonNull::from_ref(value)));
    }

    /// Checks if this pointer is null.
    ///
    /// A null pointer indicates the observer was constructed with [`uninit`](Self::uninit) and has
    /// not been properly initialized via [`refresh`](crate::observe::Observer::refresh).
    #[inline]
    pub const fn is_null(this: &Self) -> bool {
        this.0.get().is_none()
    }

    /// Returns a reference to the pointed value.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that:
    /// 1. The pointer is not null (was properly initialized via [`new`](Self::new))
    /// 2. The original value this pointer was created from is still valid
    /// 3. No mutable references to the same value exist elsewhere
    ///
    /// These invariants are automatically maintained when using [`Pointer`] within the observer
    /// infrastructure, but must be manually verified if called directly.
    #[inline]
    pub const unsafe fn as_ref<'ob>(this: &Self) -> &'ob S {
        let ptr = this.0.get().expect("Pointer should not be null");
        // SAFETY: The caller guarantees the pointer is valid and properly aligned,
        // and that the lifetime 'ob does not outlive the original value.
        unsafe { ptr.as_ref() }
    }

    /// Returns a mutable reference to the pointed value.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that:
    /// 1. The pointer is not null (was properly initialized via [`new`](Self::new))
    /// 2. The original value this pointer was created from is still valid
    /// 3. No other references (mutable or immutable) to the same value exist elsewhere
    /// 4. The returned reference is used in a way that maintains Rust's aliasing rules
    ///
    /// These invariants are automatically maintained when using [`Pointer`] within the observer
    /// infrastructure, but must be manually verified if called directly.
    #[inline]
    pub const unsafe fn as_mut<'ob>(this: &Self) -> &'ob mut S {
        let mut ptr = this.0.get().expect("Pointer should not be null");
        // SAFETY: The caller guarantees exclusive access to the pointed value,
        // that the pointer is valid and properly aligned, and that the lifetime
        // 'ob does not outlive the original value.
        unsafe { ptr.as_mut() }
    }
}

impl<S: ?Sized> PartialEq for Pointer<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<S: ?Sized> Eq for Pointer<S> {}

impl<S: ?Sized> Deref for Pointer<S> {
    type Target = S;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { Self::as_ref(self) }
    }
}

impl<S: ?Sized> DerefMut for Pointer<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut S {
        unsafe { Self::as_mut(self) }
    }
}

impl<S: ?Sized> AsNormalized for Pointer<S> {
    type OuterDepth = Zero;
}
