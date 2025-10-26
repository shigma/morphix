use std::mem::MaybeUninit;

use crate::Observe;
use crate::helper::{AsDeref, Zero};
use crate::impls::boxed::BoxObserveImpl;
use crate::impls::option::OptionObserveImpl;
use crate::observe::{AsDerefMut, DebugHandler, GeneralHandler, GeneralObserver, Unsigned};

/// A general observer that uses snapshot comparison to detect actual value changes.
///
/// `SnapshotObserver` creates a clone of the initial value and compares it with the
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
/// Can be used via the `#[observe(snapshot)]` attribute in derive macros:
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
///     #[observe(snapshot)]
///     id: Uuid,           // Cheap to clone and compare
///     #[observe(snapshot)]
///     flags: BitFlags,    // Small Copy type
/// }
/// ```
///
/// ## When to Use
///
/// `SnapshotObserver` is ideal when:
/// 1. The type implements [`Clone`] and [`PartialEq`] with low cost
/// 2. Values may be modified and then restored to original (so that
///    [`ShallowObserver`](super::ShallowObserver) would yield false positives)
///
/// ## Built-in Usage
///
/// All primitive types ([`i32`], [`f64`], [`bool`], etc.) use `SnapshotObserver` as their default
/// implementation since they're cheap to clone and compare.
pub type SnapshotObserver<'i, S, N = Zero> = GeneralObserver<'i, SnapshotHandler<<S as AsDeref<N>>::Target>, S, N>;

pub struct SnapshotHandler<T> {
    snapshot: MaybeUninit<T>,
}

impl<T> Default for SnapshotHandler<T> {
    #[inline]
    fn default() -> Self {
        Self {
            snapshot: MaybeUninit::uninit(),
        }
    }
}

impl<T: Clone + PartialEq> GeneralHandler<T> for SnapshotHandler<T> {
    type Spec = SnapshotSpec;

    #[inline]
    fn on_observe(value: &mut T) -> Self {
        Self {
            snapshot: MaybeUninit::new(value.clone()),
        }
    }

    #[inline]
    fn on_deref_mut(&mut self) {}

    #[inline]
    fn on_collect(&self, value: &T) -> bool {
        // SAFETY: `GeneralHandler::on_collect` is only called in `Observer::collect_unchecked`, where the
        // observer is assumed to contain a valid pointer
        value != unsafe { self.snapshot.assume_init_ref() }
    }
}

impl<T: Clone + PartialEq> DebugHandler<T> for SnapshotHandler<T> {
    const NAME: &'static str = "SnapshotObserver";
}

/// Snapshot-based observation specification.
///
/// `SnapshotSpec` marks a type as supporting efficient snapshot comparison (requires [`Clone`] +
/// [`PartialEq`]). When used as the [`Spec`](crate::Observe::Spec) for a type `T`, it affects
/// certain wrapper type observations, such as [`Option<T>`].
pub struct SnapshotSpec;

macro_rules! impl_observe {
    ($($ty:ty),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Observer<'i, S, N>
                    = SnapshotObserver<'i, S, N>
                where
                    Self: 'i,
                    N: Unsigned,
                    S: AsDerefMut<N, Target = Self> + ?Sized + 'i;

                type Spec = SnapshotSpec;
            }
        )*
    };
}

impl_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    ::core::net::IpAddr, ::core::net::Ipv4Addr, ::core::net::Ipv6Addr,
    ::core::net::SocketAddr, ::core::net::SocketAddrV4, ::core::net::SocketAddrV6,
    ::core::time::Duration, ::std::time::SystemTime,
}

impl<T> BoxObserveImpl<T, SnapshotSpec> for T
where
    T: Clone + PartialEq + Observe<Spec = SnapshotSpec>,
{
    type Observer<'i, S, N>
        = SnapshotObserver<'i, S, N>
    where
        T: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Box<T>> + ?Sized + 'i;
}

impl<T> OptionObserveImpl<T, SnapshotSpec> for T
where
    T: Clone + PartialEq + Observe<Spec = SnapshotSpec>,
{
    type Observer<'i, S, N>
        = SnapshotObserver<'i, S, N>
    where
        T: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Option<T>> + ?Sized + 'i;
}
