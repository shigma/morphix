use std::mem::MaybeUninit;

use crate::Observe;
use crate::helper::{AsDeref, AsDerefMut, Zero};
use crate::observe::general::ReplaceHandler;
use crate::observe::{DebugHandler, GeneralHandler, GeneralObserver, RefObserve, Unsigned};

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
/// # #[derive(Serialize, Clone, PartialEq)]
/// # struct Uuid;
/// # #[derive(Serialize, Clone, PartialEq)]
/// # struct BitFlags;
/// #[derive(Serialize, Clone, PartialEq, Observe)]
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
pub type SnapshotObserver<'ob, S, D = Zero, E = Zero> =
    GeneralObserver<'ob, SnapshotHandler<<S as AsDeref<D>>::Target, E>, S, D>;

pub struct SnapshotHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    T::Target: Sized,
    E: Unsigned,
{
    snapshot: MaybeUninit<T::Target>,
}

impl<T, E> GeneralHandler for SnapshotHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    T::Target: Clone + PartialEq + Sized,
    E: Unsigned,
{
    type Target = T;
    type Spec = SnapshotSpec;

    #[inline]
    fn uninit() -> Self {
        Self {
            snapshot: MaybeUninit::uninit(),
        }
    }

    #[inline]
    fn observe(value: &T) -> Self {
        Self {
            snapshot: MaybeUninit::new(value.as_deref().clone()),
        }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<T, E> ReplaceHandler for SnapshotHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    T::Target: Clone + PartialEq + Sized,
    E: Unsigned,
{
    #[inline]
    fn flush_replace(&mut self, value: &T) -> bool {
        // SAFETY: `ReplaceHandler::flush_replace` is only called in `Observer::flush_unchecked`, where the
        // observer is assumed to contain a valid pointer
        value.as_deref() != unsafe { self.snapshot.assume_init_ref() }
    }
}

impl<T, E> DebugHandler for SnapshotHandler<T, E>
where
    T: AsDeref<E> + ?Sized,
    T::Target: Clone + PartialEq + Sized,
    E: Unsigned,
{
    const NAME: &'static str = "SnapshotObserver";
}

/// Snapshot-based observation specification.
///
/// [`SnapshotSpec`] marks a type as supporting efficient snapshot comparison (requires [`Clone`] +
/// [`PartialEq`]). When used as the [`Spec`](crate::Observe::Spec) for a type `T`, it affects
/// certain wrapper type observations, such as [`Option<T>`].
pub struct SnapshotSpec;

macro_rules! impl_snapshot_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Observer<'ob, S, D>
                    = SnapshotObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

                type Spec = SnapshotSpec;
            }

            impl RefObserve for $ty {
                type Observer<'ob, S, D, E>
                    = SnapshotObserver<'ob, S, D, E>
                where
                    Self: 'ob,
                    D: Unsigned,
                    E: Unsigned,
                    S: AsDeref<D> + ?Sized + 'ob, S::Target: AsDeref<E, Target = Self>;

                type Spec = SnapshotSpec;
            }
        )*
    };
}

impl_snapshot_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    core::net::IpAddr, core::net::Ipv4Addr, core::net::Ipv6Addr,
    core::net::SocketAddr, core::net::SocketAddrV4, core::net::SocketAddrV6,
    core::time::Duration, std::time::SystemTime,
}
