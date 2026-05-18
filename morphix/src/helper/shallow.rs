//! Shallow observer infrastructure: [`ShallowMut`] wrapper and the [`shallow_observer!`] macro
//! for generating simple observers that track mutations via a single boolean flag.
//!
//! ## Stability
//!
//! APIs in this module are unstable and may change in a future version.

use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::helper::Invalidate;

impl<T: ?Sized> Invalidate<T> for bool {
    fn invalidate(&mut self, _: &T) {
        *self = true;
    }
}

/// A delegate state that forwards invalidation to an external state `V` via a raw pointer.
///
/// Implements [`Invalidate<T>`] for any `T` where `V: Invalidate<T>`, forwarding the call
/// through the raw pointer. Use this as the state type in generic observers to create
/// views that propagate invalidation to a parent observer's state.
pub struct ShallowDelegate<V: ?Sized> {
    state: *mut V,
}

impl<V: ?Sized> ShallowDelegate<V> {
    /// Creates a new [`ShallowDelegate`] from a raw pointer to the external state.
    pub fn new(state: *mut V) -> Self {
        Self { state }
    }
}

impl<T: ?Sized, V: Invalidate<T> + ?Sized> Invalidate<T> for ShallowDelegate<V> {
    fn invalidate(&mut self, value: &T) {
        unsafe { Invalidate::invalidate(&mut *self.state, value) }
    }
}

/// A mutable handle to a value `T` that invalidates a shared state `V` whenever it is mutated.
///
/// [`ShallowMut`] decouples the borrowed value from its invalidation target: [`Self::inner`] is the
/// value the caller mutates, while [`Self::state`] is a raw pointer to a separate piece of state
/// (often living on a parent observer) that gets invalidated through the [`Invalidate`]
/// trait on each [`DerefMut`].
pub struct ShallowMut<'ob, T: ?Sized, V: ?Sized> {
    pub(crate) inner: &'ob mut T,
    pub(crate) state: *mut V,
}

impl<'ob, T: ?Sized, V: ?Sized> ShallowMut<'ob, T, V> {
    /// Constructs a [`ShallowMut`] from a mutable borrow and a raw pointer to its invalidation
    /// state.
    ///
    /// The state pointer must remain valid for the lifetime `'ob`.
    pub fn new(inner: &'ob mut T, state: *mut V) -> Self {
        Self { inner, state }
    }
}

impl<'ob, T: ?Sized, V: ?Sized> Deref for ShallowMut<'ob, T, V> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'ob, T: ?Sized, V: Invalidate<()> + ?Sized> DerefMut for ShallowMut<'ob, T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { Invalidate::invalidate(&mut *self.state, &()) }
        self.inner
    }
}

impl<'ob, T: Debug + ?Sized, V: ?Sized> Debug for ShallowMut<'ob, T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShallowMut").field(&self.inner).finish()
    }
}

/// Generates a shallow observer type that tracks mutations via a single `bool` flag.
///
/// The generated observer uses [`Invalidate`] on the flag, and emits a whole-value
/// [`Replace`](crate::MutationKind::Replace) on [`flush`](crate::observe::SerializeObserver::flush)
/// when the flag is set. This is appropriate for types whose internal structure is opaque or where
/// per-field granularity is not needed.
///
/// Also generates [`Observe`](crate::Observe), [`Observer`](crate::observe::Observer),
/// [`SerializeObserver`](crate::observe::SerializeObserver), [`QuasiObserver`], and standard trait
/// impls ([`Deref`], [`DerefMut`], [`Debug`], [`PartialEq`], [`Eq`], [`PartialOrd`], [`Ord`],
/// [`AsMut`]).
#[doc(hidden)]
#[macro_export]
macro_rules! __shallow_observer {
    () => {};

    ($(#[$meta:meta])* struct $ob:ident ($($arg:tt)*); $($rest:tt)*) => {
        $crate::__shallow_observer!(@impl $(#[$meta])* struct $ob [] ($($arg)*););
        $crate::__shallow_observer!($($rest)*);
    };

    ($(#[$meta:meta])* struct $ob:ident < $($rest:tt)*) => {
        $crate::__shallow_observer!(@gen $(#[$meta])* struct $ob [] $($rest)*);
    };

    (@gen $(#[$meta:meta])* struct $ob:ident [$($gen:tt)*] > ($($arg:tt)*); $($rest:tt)*) => {
        $crate::__shallow_observer!(@impl $(#[$meta])* struct $ob [$($gen)* ,] ($($arg)*););
        $crate::__shallow_observer!($($rest)*);
    };

    (@gen $(#[$meta:meta])* struct $ob:ident [$($gen:tt)*] >> ($($arg:tt)*); $($rest:tt)*) => {
        $crate::__shallow_observer!(@impl $(#[$meta])* struct $ob [$($gen)* >,] ($($arg)*););
        $crate::__shallow_observer!($($rest)*);
    };

    (@gen $(#[$meta:meta])* struct $ob:ident [$($gen:tt)*] $tt:tt $($rest:tt)*) => {
        $crate::__shallow_observer!(@gen $(#[$meta])* struct $ob [$($gen)* $tt] $($rest)*);
    };

    (@impl $(#[$meta:meta])* struct $ob:ident [$($gen:tt)*] ($ty:ty);) => {
        #[doc = concat!("Observer implementation for [`", stringify!($ty), "`].")]
        #[doc = ""]
        $(#[$meta])*
        pub struct $ob<'ob, S: ?Sized, D = $crate::helper::Zero> {
            pub(crate) ptr: $crate::helper::Pointer<S>,
            pub(crate) mutated: bool,
            phantom: ::std::marker::PhantomData<&'ob mut D>,
        }

        impl<'ob, S: ?Sized, D> ::std::ops::Deref for $ob<'ob, S, D> {
            type Target = $crate::helper::Pointer<S>;

            fn deref(&self) -> &Self::Target {
                &self.ptr
            }
        }

        impl<'ob, S: ?Sized, D> ::std::ops::DerefMut for $ob<'ob, S, D> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                $crate::helper::Invalidate::invalidate(&mut self.mutated, &());
                $crate::helper::QuasiObserver::invalidate(&mut self.ptr);
                &mut self.ptr
            }
        }

        impl<'ob, S: ?Sized, D> $crate::helper::QuasiObserver for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D>,
        {
            type Head = S;
            type OuterDepth = $crate::helper::Succ<$crate::helper::Zero>;
            type InnerDepth = D;

            fn invalidate(this: &mut Self) {
                $crate::helper::Invalidate::invalidate(&mut this.mutated, &());
            }
        }

        impl<'ob, S: ?Sized, D> $crate::observe::Observer for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDerefMut<D>,
        {
            fn observe(head: &mut Self::Head) -> Self {
                Self {
                    ptr: $crate::helper::Pointer::new(head),
                    mutated: false,
                    phantom: ::std::marker::PhantomData,
                }
            }

            unsafe fn relocate(this: &mut Self, head: &mut Self::Head) {
                $crate::helper::Pointer::set(this, head);
            }
        }

        impl<'ob, S: ?Sized, D> $crate::observe::SerializeObserver for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D, Target: ::serde::Serialize + 'static>,
        {
            unsafe fn flush(this: &mut Self) -> $crate::mutation::Mutations {
                if ::std::mem::take(&mut this.mutated) {
                    $crate::mutation::Mutations::replace((*this.ptr).as_deref())
                } else {
                    $crate::mutation::Mutations::new()
                }
            }
        }

        impl<'ob, $($gen)* S: ?Sized, D> AsMut<$ty> for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDerefMut<D, Target = $ty>,
        {
            fn as_mut(&mut self) -> &mut $ty {
                $crate::helper::QuasiObserver::tracked_mut(self)
            }
        }

        impl<'ob, S: ?Sized, D> std::fmt::Debug for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D>,
            S::Target: std::fmt::Debug,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($ob)).field(&$crate::helper::QuasiObserver::untracked_ref(self)).finish()
            }
        }

        impl<'ob, S: ?Sized, D> PartialEq<$ob<'ob, S, D>> for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D>,
            S::Target: PartialEq,
        {
            fn eq(&self, other: &$ob<'ob, S, D>) -> bool {
                $crate::helper::QuasiObserver::untracked_ref(self).eq($crate::helper::QuasiObserver::untracked_ref(other))
            }
        }

        impl<'ob, S: ?Sized, D> Eq for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D>,
            S::Target: Eq,
        {
        }

        impl<'ob, S: ?Sized, D> PartialOrd<$ob<'ob, S, D>> for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D>,
            S::Target: PartialOrd,
        {
            fn partial_cmp(&self, other: &$ob<'ob, S, D>) -> Option<std::cmp::Ordering> {
                $crate::helper::QuasiObserver::untracked_ref(self).partial_cmp($crate::helper::QuasiObserver::untracked_ref(other))
            }
        }

        impl<'ob, S: ?Sized, D> Ord for $ob<'ob, S, D>
        where
            D: $crate::helper::Unsigned,
            S: $crate::helper::AsDeref<D>,
            S::Target: Ord,
        {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                $crate::helper::QuasiObserver::untracked_ref(self).cmp($crate::helper::QuasiObserver::untracked_ref(other))
            }
        }

        impl <$($gen)*> $crate::observe::Observe for $ty {
            type Observer<'ob, S, D>
                = $ob<'ob, S, D>
            where
                Self: 'ob,
                D: $crate::helper::Unsigned,
                S: $crate::helper::AsDerefMut<D, Target = Self> + ?Sized + 'ob;

            type Spec = $crate::observe::DefaultSpec;
        }
    };
}

#[doc(inline)]
pub use crate::__shallow_observer as shallow_observer;
