//! Shallow observer infrastructure: [`ShallowInvalidate`] trait, [`ShallowMut`] wrapper, and the
//! [`shallow_observer!`] macro for generating simple observers that track mutations via a single
//! boolean flag.

use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::helper::quasi::DerefMutUntracked;
use crate::helper::{QuasiObserver, Zero};

/// Value-less counterpart to [`Invalidate`](crate::helper::Invalidate), used by [`ShallowMut`].
///
/// [`ShallowMut`] only sees a raw pointer to its parent observer's state, not the observed value,
/// so it can only invoke this hook.
pub trait ShallowInvalidate {
    /// Invalidates granular tracking state without access to the current value.
    fn invalidate(&mut self);
}

impl ShallowInvalidate for bool {
    fn invalidate(&mut self) {
        *self = true;
    }
}

/// A mutable handle to a value `T` that invalidates a shared state `V` whenever it is mutated.
///
/// [`ShallowMut`] decouples the borrowed value from its invalidation target: [`Self::inner`] is the
/// value the caller mutates, while [`Self::state`] is a raw pointer to a separate piece of state
/// (often living on a parent observer) that gets invalidated through the [`ShallowInvalidate`]
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

impl<'ob, T: ?Sized, V: ShallowInvalidate + ?Sized> DerefMut for ShallowMut<'ob, T, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { ShallowInvalidate::invalidate(&mut *self.state) }
        self.inner
    }
}

impl<'ob, T: ?Sized, V: ShallowInvalidate + ?Sized> DerefMutUntracked for ShallowMut<'ob, T, V> {}

impl<'ob, T: ?Sized, V: ?Sized> QuasiObserver for ShallowMut<'ob, T, V> {
    type Head = T;
    type OuterDepth = Zero;
    type InnerDepth = Zero;

    fn invalidate(_: &mut Self) {}
}

impl<'ob, T: ?Sized, V: ShallowInvalidate + ?Sized> AsMut<T> for ShallowMut<'ob, T, V> {
    fn as_mut(&mut self) -> &mut T {
        self.tracked_mut()
    }
}

impl<'ob, T: Debug + ?Sized, V: ?Sized> Debug for ShallowMut<'ob, T, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ShallowMut").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, T1: ?Sized, T2: ?Sized, V1: ?Sized, V2: ?Sized> PartialEq<ShallowMut<'ob, T2, V2>> for ShallowMut<'ob, T1, V1>
where
    T1: PartialEq<T2>,
{
    fn eq(&self, other: &ShallowMut<'ob, T2, V2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, T: Eq + ?Sized, V: ?Sized> Eq for ShallowMut<'ob, T, V> {}

impl<'ob, T1: ?Sized, T2: ?Sized, V1: ?Sized, V2: ?Sized> PartialOrd<ShallowMut<'ob, T2, V2>>
    for ShallowMut<'ob, T1, V1>
where
    T1: PartialOrd<T2>,
{
    fn partial_cmp(&self, other: &ShallowMut<'ob, T2, V2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<'ob, T: Ord + ?Sized, V: ?Sized> Ord for ShallowMut<'ob, T, V> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
    }
}

impl<'ob, T: Index<I> + ?Sized, I, V: ?Sized> Index<I> for ShallowMut<'ob, T, V> {
    type Output = T::Output;

    fn index(&self, index: I) -> &Self::Output {
        self.untracked_ref().index(index)
    }
}

impl<'ob, T: IndexMut<I> + ?Sized, I, V: ShallowInvalidate + ?Sized> IndexMut<I> for ShallowMut<'ob, T, V> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.tracked_mut().index_mut(index)
    }
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? T: ?Sized, V: ?Sized> PartialEq<$ty> for ShallowMut<'ob, T, V>
            where
                T: PartialEq<$ty>,
            {
                fn eq(&self, other: &$ty) -> bool {
                    (**self).eq(other)
                }
            }

            impl<'ob, $($($gen)*,)? T: ?Sized, V: ?Sized> PartialOrd<$ty> for ShallowMut<'ob, T, V>
            where
                T: PartialOrd<$ty>,
            {
                fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                    (**self).partial_cmp(other)
                }
            }
        )*
    };
}

generic_impl_cmp! {
    impl _ for str;
    impl _ for String;
    impl _ for std::ffi::OsStr;
    impl _ for std::ffi::OsString;
    impl _ for std::path::Path;
    impl _ for std::path::PathBuf;
    impl ['a] _ for std::borrow::Cow<'a, str>;
}

/// Generates a shallow observer type that tracks mutations via a single `bool` flag.
///
/// The generated observer uses [`ShallowInvalidate`] on the flag, and emits a whole-value
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
            ptr: $crate::helper::Pointer<S>,
            mutated: bool,
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
                $crate::helper::shallow::ShallowInvalidate::invalidate(&mut self.mutated);
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
                $crate::helper::shallow::ShallowInvalidate::invalidate(&mut this.mutated);
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
