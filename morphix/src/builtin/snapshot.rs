use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::Observe;
use crate::builtin::{DebugHandler, GeneralHandler, GeneralObserver, ReplaceHandler};
use crate::helper::{AsDeref, AsDerefMut, Unsigned, Zero};
use crate::observe::RefObserve;

/// A general observer that uses snapshot comparison to detect actual value changes.
///
/// [`SnapshotObserver`] creates a clone of the initial value and compares it with the
/// final value using [`PartialEq`]. This provides accurate change detection by comparing
/// actual values rather than tracking access patterns.
///
/// ## Requirements
///
/// The observed type must implement:
/// - [`Clone`] - for creating the snapshot
/// - [`PartialEq`] - for comparing values
///
/// ## Derive Usage
///
/// Can be used via the `#[morphix(snapshot)]` attribute in derive macros:
///
/// ```
/// # use morphix::Observe;
/// # use serde::Serialize;
/// # #[derive(Serialize, Observe)]
/// # struct Uuid;
/// # #[derive(Serialize, Observe)]
/// # struct BitFlags;
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     #[morphix(snapshot)]
///     id: Uuid,           // Cheap to clone and compare
///     #[morphix(snapshot)]
///     flags: BitFlags,    // Small Copy type
/// }
/// ```
///
/// ## When to Use
///
/// [`SnapshotObserver`] is ideal when:
/// 1. The type implements [`Clone`] and [`PartialEq`] with low cost
/// 2. Values may be modified and then restored to original (so that
///    [`ShallowObserver`](super::ShallowObserver) would yield false positives)
///
/// ## Built-in Usage
///
/// All primitive types ([`i32`], [`f64`], [`bool`], etc.) use [`SnapshotObserver`] as their default
/// implementation since they're cheap to clone and compare.
pub type SnapshotObserver<'ob, S, D = Zero> = GeneralObserver<'ob, SnapshotHandler<<S as AsDeref<D>>::Target>, S, D>;

pub trait Snapshot {
    type Value;

    fn to_snapshot(&self) -> Self::Value;

    fn eq_snapshot(&self, snapshot: &Self::Value) -> bool;
}

pub struct SnapshotHandler<T: Snapshot + ?Sized> {
    snapshot: MaybeUninit<T::Value>,
    phantom: PhantomData<T>,
}

impl<T: Snapshot + ?Sized> GeneralHandler for SnapshotHandler<T> {
    type Target = T;
    type Spec = SnapshotSpec;

    #[inline]
    fn uninit() -> Self {
        Self {
            snapshot: MaybeUninit::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &T) -> Self {
        Self {
            snapshot: MaybeUninit::new(value.to_snapshot()),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T: Snapshot + ?Sized> ReplaceHandler for SnapshotHandler<T> {
    #[inline]
    fn flush_replace(&mut self, value: &T) -> bool {
        // SAFETY: `ReplaceHandler::flush_replace` is only called in `Observer::flush_unchecked`, where the
        // observer is assumed to contain a valid pointer
        !value.eq_snapshot(unsafe { self.snapshot.assume_init_ref() })
    }
}

impl<T: Snapshot + ?Sized> DebugHandler for SnapshotHandler<T> {
    const NAME: &'static str = "SnapshotObserver";
}

/// Snapshot-based observation specification.
///
/// [`SnapshotSpec`] marks a type as supporting efficient snapshot comparison (requires [`Clone`] +
/// [`PartialEq`]). When used as the [`Spec`](crate::Observe::Spec) for a type `T`, it affects
/// certain wrapper type observations, such as [`Option<T>`].
pub struct SnapshotSpec;

macro_rules! impl_snapshot_observe {
    ($($(#[$($meta:tt)*])* $ty:ty),* $(,)?) => {
        $(
            $(#[$($meta)*])*
            impl Snapshot for $ty {
                type Value = Self;
                #[inline]
                fn to_snapshot(&self) -> Self {
                    *self
                }
                #[inline]
                fn eq_snapshot(&self, snapshot: &Self) -> bool {
                    self == snapshot
                }
            }

            $(#[$($meta)*])*
            impl Observe for $ty {
                type Observer<'ob, S, D>
                    = SnapshotObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = SnapshotSpec;
            }

            $(#[$($meta)*])*
            impl RefObserve for $ty {
                type Observer<'ob, S, D>
                    = SnapshotObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDeref<D, Target = Self> + ?Sized + 'ob;

                type Spec = SnapshotSpec;
            }
        )*
    };
}

impl_snapshot_observe! {
    (), usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    core::net::IpAddr, core::net::Ipv4Addr, core::net::Ipv6Addr,
    core::net::SocketAddr, core::net::SocketAddrV4, core::net::SocketAddrV6,
    core::time::Duration, std::time::SystemTime,
    #[cfg(feature = "uuid")]
    uuid::Uuid,
}

macro_rules! generic_impl_snapshot_observe {
    ($($(#[$($meta:tt)*])* impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            $(#[$($meta)*])*
            impl <$($($gen)*)?> Snapshot for $ty {
                type Value = Self;
                #[inline]
                fn to_snapshot(&self) -> Self {
                    *self
                }
                #[inline]
                fn eq_snapshot(&self, snapshot: &Self) -> bool {
                    self == snapshot
                }
            }

            $(#[$($meta)*])*
            impl <$($($gen)*)?> Observe for $ty {
                type Observer<'ob, S, D>
                    = SnapshotObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = SnapshotSpec;
            }

            $(#[$($meta)*])*
            impl <$($($gen)*)?> RefObserve for $ty {
                type Observer<'ob, S, D>
                    = SnapshotObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDeref<D, Target = Self> + ?Sized + 'ob;

                type Spec = SnapshotSpec;
            }
        )*
    };
}

generic_impl_snapshot_observe! {
    impl [T] _ for std::marker::PhantomData<T>;
}
