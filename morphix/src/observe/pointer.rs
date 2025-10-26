use std::ops::{Deref, DerefMut};

/// An internal pointer type for observer dereference chains.
///
/// `ObserverPointer` is a specialized pointer type used exclusively within observer implementations
/// to store references to observed values. It serves as a critical component in the observer
/// dereference chain, allowing multiple levels of observers to coexist while maintaining access to
/// the original value.
///
/// ## Purpose
///
/// When observing types that already implement [`Deref`] (like [`Vec<T>`]), we need a way to break
/// the dereference chain to insert observer logic at multiple levels. `ObserverPointer` provides
/// this break point, enabling chains like: [`VecObserver`](crate::impls::vec::VecObserver) →
/// `SliceObserver` → `ObserverPointer<[T]>` → [`Vec<T>`] → [`[T]`](std::slice).
///
/// ## Safety Considerations
///
/// This type uses raw pointers internally and relies on several safety invariants:
///
/// 1. **Lifetime tracking**: The lifetime `'i` in observers ensures the pointed-to value remains
///    valid for the observer's lifetime
/// 2. **Initialization**: Pointers must be properly initialized via [`new`](ObserverPointer::new)
///    before dereferencing
/// 3. **Single ownership**: Each `ObserverPointer` assumes exclusive access to its referenced value
///    during the observer's lifetime
///
/// ## Internal Use Only
///
/// This type is not intended for direct use outside of observer implementations. All safety
/// invariants are maintained by the observer infrastructure when used correctly within that
/// context.
pub struct ObserverPointer<S: ?Sized>(Option<*mut S>);

impl<S: ?Sized> ObserverPointer<S> {
    /// Creates a new pointer from a mutable reference.
    ///
    /// The returned pointer will remain valid as long as the original reference remains valid,
    /// which is enforced by the lifetime parameter in observer types.
    #[inline]
    pub fn new(value: &mut S) -> Self {
        Self(Some(value))
    }

    /// Checks if this pointer is null.
    ///
    /// A null pointer indicates the observer was [`Default`]-constructed and has not been properly
    /// initialized via [`observe`](crate::observe::Observer::observe).
    #[inline]
    pub fn is_null(this: &Self) -> bool {
        this.0.is_none()
    }

    /// Returns a reference to the pointed value.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that:
    /// 1. The pointer is not null (was properly initialized via [`new`](ObserverPointer::new))
    /// 2. The original value this pointer was created from is still valid
    /// 3. No mutable references to the same value exist elsewhere
    ///
    /// These invariants are automatically maintained when using [`ObserverPointer`] within the
    /// observer infrastructure, but must be manually verified if called directly.
    #[inline]
    pub unsafe fn as_ref<'i>(this: &Self) -> &'i S {
        let ptr = this.0.expect("Observed pointer should not be null");
        // SAFETY: The caller guarantees the pointer is valid and properly aligned,
        // and that the lifetime 'i does not outlive the original value.
        unsafe { &*ptr }
    }

    /// Returns a mutable reference to the pointed value.
    ///
    /// ## Safety
    ///
    /// The caller must ensure that:
    /// 1. The pointer is not null (was properly initialized via [`new`](ObserverPointer::new))
    /// 2. The original value this pointer was created from is still valid
    /// 3. No other references (mutable or immutable) to the same value exist elsewhere
    /// 4. The returned reference is used in a way that maintains Rust's aliasing rules
    ///
    /// These invariants are automatically maintained when using [`ObserverPointer`] within the
    /// observer infrastructure, but must be manually verified if called directly.
    #[inline]
    pub unsafe fn as_mut<'i>(this: &Self) -> &'i mut S {
        let ptr = this.0.expect("Observed pointer should not be null");
        // SAFETY: The caller guarantees exclusive access to the pointed value,
        // that the pointer is valid and properly aligned, and that the lifetime
        // 'i does not outlive the original value.
        unsafe { &mut *ptr }
    }
}

impl<S: ?Sized> Default for ObserverPointer<S> {
    #[inline]
    fn default() -> Self {
        Self(None)
    }
}

impl<S: ?Sized> PartialEq for ObserverPointer<S> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<S: ?Sized> Eq for ObserverPointer<S> {}

impl<S: ?Sized> Deref for ObserverPointer<S> {
    type Target = S;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { Self::as_ref(self) }
    }
}

impl<S: ?Sized> DerefMut for ObserverPointer<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut S {
        unsafe { Self::as_mut(self) }
    }
}
