use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::{AsDerefMut, AsNormalized, Pointer, Succ, Unsigned, Zero};
use crate::observe::{Observer, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations};

/// A handler trait for implementing change detection strategies in [`GeneralObserver`].
///
/// [`GeneralHandler`] defines the interface for pluggable change detection strategies used
/// exclusively with [`GeneralObserver`]. Each handler implementation encapsulates a specific
/// approach to detecting whether a value has changed.
///
/// ## Example
///
/// A [`ShallowObserver`](super::ShallowObserver) implementation that treats any mutation through
/// [`DerefMut`] as a complete replacement:
///
/// ```
/// # use std::marker::PhantomData;
/// # use morphix::builtin::{GeneralHandler, GeneralObserver};
/// # use morphix::observe::DefaultSpec;
/// struct ShallowHandler<T> {
///     mutated: bool,
///     phantom: PhantomData<T>,
/// }
///
/// impl<T> GeneralHandler for ShallowHandler<T> {
///     type Target = T;
///     type Spec = DefaultSpec;
///
///     fn uninit() -> Self {
///        Self { mutated: false, phantom: PhantomData }
///     }
///
///     fn observe(_value: &T) -> Self {
///         Self { mutated: false, phantom: PhantomData }
///     }
///
///     fn deref_mut(&mut self) {
///         self.mutated = true;
///     }
/// }
///
/// type ShallowObserver<'ob, T> = GeneralObserver<'ob, T, ShallowHandler<T>>;
/// ```
pub trait GeneralHandler {
    /// The target type being observed.
    type Target: ?Sized;

    /// Associated specification type for [`GeneralObserver`].
    type Spec;

    /// Implementation for [`Observer::uninit`].
    fn uninit() -> Self;

    /// Implementation for [`Observer::observe`].
    fn observe(value: &Self::Target) -> Self;

    /// Called when the value is accessed through [`DerefMut`].
    fn deref_mut(&mut self);
}

/// A handler that can serialize mutations for [`GeneralObserver`].
///
/// This trait extends [`GeneralHandler`] with serialization capabilities. A [`GeneralHandler`]
/// must implement [`SerializeHandler`] for its corresponding [`GeneralObserver`] to implement
/// [`SerializeObserver`].
///
/// ## Blanket Implementation
///
/// A blanket implementation is provided for all types that implement [`ReplaceHandler`]
/// where the observed type implements [`Serialize`]. This automatically converts the
/// boolean result from [`flush_replace`](ReplaceHandler::flush_replace) into a
/// [`Replace`](MutationKind::Replace) mutation when changes are detected.
///
/// Most handlers only need to implement [`ReplaceHandler`] to gain full serialization
/// support. Direct implementation of [`SerializeHandler`] is only necessary for handlers
/// that need to emit non-replace mutations (like [`Append`](MutationKind::Append)).
pub trait SerializeHandler: GeneralHandler {
    /// Implementation for [`SerializeObserver::flush_unchecked`].
    ///
    /// ## Safety
    ///
    /// See [`SerializeObserver::flush_unchecked`].
    unsafe fn flush<A: Adapter>(&mut self, value: &Self::Target) -> Result<Mutations<A::Value>, A::Error>;
}

/// A handler that can only express replace-style mutations.
///
/// This trait provides a simplified interface for handlers that only need to track whether the
/// observed value has changed, without distinguishing between different mutation kinds (like
/// [`Append`](MutationKind::Append) or [`Truncate`](MutationKind::Truncate)). Most
/// [`GeneralHandler`] implementations implement this trait rather than [`SerializeHandler`]
/// directly.
pub trait ReplaceHandler: GeneralHandler {
    /// Determines whether the observed value should be reported as replaced.
    ///
    /// This method is called during [`flush`](SerializeHandler::flush) to check if the value has
    /// changed. It also resets the handler's internal state, so that an immediate subsequent call
    /// will return `false` unless new mutations occur.
    ///
    /// ## Returns
    ///
    /// - `true`: The value has changed and should be serialized as a
    ///   [`Replace`](MutationKind::Replace) mutation
    /// - `false`: No changes detected, no mutation will be emitted
    fn flush_replace(&mut self, value: &Self::Target) -> bool;
}

impl<H> SerializeHandler for H
where
    H: ReplaceHandler,
    H::Target: Serialize,
{
    #[inline]
    unsafe fn flush<A: Adapter>(&mut self, value: &Self::Target) -> Result<Mutations<A::Value>, A::Error> {
        if self.flush_replace(value) {
            Ok(MutationKind::Replace(A::serialize_value(value)?).into())
        } else {
            Ok(Mutations::new())
        }
    }
}

/// A helper trait for providing a custom name when formatting [`GeneralObserver`] with [`Debug`].
///
/// [`DebugHandler`] extends [`GeneralHandler`] by adding a [`NAME`](DebugHandler::NAME) constant
/// used as the type label in [`Debug`] output for [`GeneralObserver`].
///
/// ## Example
///
/// ```
/// # use std::marker::PhantomData;
/// use morphix::builtin::{DebugHandler, GeneralHandler, GeneralObserver};
/// use morphix::observe::Observer;
///
/// pub struct MyHandler<T>(PhantomData<T>);
///
/// impl<T> GeneralHandler for MyHandler<T> {
///     // omitted for brevity
/// #   type Target = T;
/// #   type Spec = morphix::observe::DefaultSpec;
/// #   fn uninit() -> Self { Self(PhantomData) }
/// #   fn observe(_value: &T) -> Self { Self(PhantomData) }
/// #   fn deref_mut(&mut self) {}
/// }
///
/// impl<T> DebugHandler for MyHandler<T> {
///     const NAME: &'static str = "MyObserver";
/// }
///
/// let mut value = 123;
/// let ob = GeneralObserver::<MyHandler<i32>, i32>::observe(&mut value);
/// println!("{:?}", ob); // prints: MyObserver(123)
/// ```
pub trait DebugHandler: GeneralHandler {
    /// The name displayed when formatting the observer with [`Debug`].
    const NAME: &'static str;
}

/// A general-purpose [`Observer`] implementation with extensible change detection strategies.
///
/// [`GeneralObserver`] provides a flexible framework for implementing different change detection
/// strategies through the [`GeneralHandler`] trait. It serves as the foundation for several
/// built-in observer types.
///
/// ## Capabilities and Limitations
///
/// [`GeneralObserver`] can:
/// - Detect whether a value has changed via [`DerefMut`]
/// - Produce [`Replace`](MutationKind::Replace) mutations when changes are detected
///
/// [`GeneralObserver`] cannot:
/// - Track field-level changes or interior mutations within complex types
/// - Add specialized implementations for common traits (e.g. [`AddAssign`](std::ops::AddAssign))
///
/// For types that benefit from more sophisticated change tracking, morphix provides specialized
/// observer implementations. These include built-in support for [`String`] and [`Vec`] (which can
/// track append operations), as well as custom observers generated by `#[derive(Observe)]` (which
/// can track field-level changes).
///
/// ## Built-in Implementations
///
/// The following observer types are built on [`GeneralObserver`]:
///
/// - [`ShallowObserver`](super::ShallowObserver) - Tracks any [`DerefMut`] access as a change
/// - [`NoopObserver`](super::NoopObserver) - Ignores all changes
/// - [`SnapshotObserver`](super::SnapshotObserver) - Compares cloned snapshots to detect changes
pub struct GeneralObserver<'ob, H, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    handler: H,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, H, S: ?Sized, D> Deref for GeneralObserver<'ob, H, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, H, S: ?Sized, D> DerefMut for GeneralObserver<'ob, H, S, D>
where
    H: GeneralHandler,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.handler.deref_mut();
        &mut self.ptr
    }
}

impl<'ob, H, S: ?Sized, D> AsNormalized for GeneralObserver<'ob, H, S, D> {
    type OuterDepth = Succ<Zero>;
}

impl<'ob, H, S: ?Sized, D, T: ?Sized> Observer for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D, Target = T> + 'ob,
    H: GeneralHandler<Target = T>,
    D: Unsigned,
{
    type InnerDepth = D;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            handler: H::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        Pointer::set(this, value);
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            handler: H::observe(value.as_deref()),
            phantom: PhantomData,
        }
    }
}

impl<'ob, H, S: ?Sized, D, T: ?Sized> SerializeObserver for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D, Target = T> + 'ob,
    H: SerializeHandler<Target = T>,
    D: Unsigned,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        unsafe { this.handler.flush::<A>(this.ptr.as_deref()) }
    }
}

macro_rules! impl_fmt {
    ($($trait:ident),* $(,)?) => {
        $(
            impl<'ob, H, S: ?Sized, D> std::fmt::$trait for GeneralObserver<'ob, H, S, D>
            where
                S: crate::helper::AsDeref<D>,
                D: Unsigned,
                S::Target: std::fmt::$trait,
            {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::$trait::fmt(self.as_deref(), f)
                }
            }
        )*
    };
}

impl_fmt! {
    Binary,
    Display,
    LowerExp,
    LowerHex,
    Octal,
    Pointer,
    UpperExp,
    UpperHex,
}

impl<'ob, H, S: ?Sized, D, T: ?Sized> Debug for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D, Target = T>,
    H: DebugHandler<Target = T>,
    D: Unsigned,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(H::NAME).field(&self.as_deref()).finish()
    }
}

impl<'ob, H, S: ?Sized, D, I> std::ops::Index<I> for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D>,
    D: Unsigned,
    S::Target: std::ops::Index<I>,
{
    type Output = <S::Target as std::ops::Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.as_deref().index(index)
    }
}

impl<'ob, H, S: ?Sized, D, T: ?Sized, I> std::ops::IndexMut<I> for GeneralObserver<'ob, H, S, D>
where
    S: AsDerefMut<D, Target = T>,
    H: GeneralHandler<Target = T>,
    D: Unsigned,
    T: std::ops::IndexMut<I> + 'ob,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        Observer::as_inner(self).index_mut(index)
    }
}

impl<'ob, H1, H2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<GeneralObserver<'ob, H2, S2, D2>>
    for GeneralObserver<'ob, H1, S1, D1>
where
    S1: crate::helper::AsDeref<D1>,
    S2: crate::helper::AsDeref<D2>,
    D1: Unsigned,
    D2: Unsigned,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &GeneralObserver<'ob, H2, S2, D2>) -> bool {
        self.as_deref().eq(other.as_deref())
    }
}

impl<'ob, H, S: ?Sized, D> Eq for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D>,
    D: Unsigned,
    S::Target: Eq,
{
}

impl<'ob, H1, H2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<GeneralObserver<'ob, H2, S2, D2>>
    for GeneralObserver<'ob, H1, S1, D1>
where
    S1: crate::helper::AsDeref<D1>,
    S2: crate::helper::AsDeref<D2>,
    D1: Unsigned,
    D2: Unsigned,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &GeneralObserver<'ob, H2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other.as_deref())
    }
}

impl<'ob, H, S: ?Sized, D> Ord for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D>,
    D: Unsigned,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_deref().cmp(other.as_deref())
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'ob, H, S: ?Sized, D, T: ?Sized, U> std::ops::$trait<U> for GeneralObserver<'ob, H, S, D>
            where
                S: AsDerefMut<D, Target = T>,
                H: GeneralHandler<Target = T>,
                D: Unsigned,
                T: std::ops::$trait<U>,
            {
                #[inline]
                fn $method(&mut self, rhs: U) {
                    Observer::as_inner(self).$method(rhs);
                }
            }
        )*
    };
}

impl_assign_ops! {
    AddAssign => add_assign,
    SubAssign => sub_assign,
    MulAssign => mul_assign,
    DivAssign => div_assign,
    RemAssign => rem_assign,
    BitAndAssign => bitand_assign,
    BitOrAssign => bitor_assign,
    BitXorAssign => bitxor_assign,
    ShlAssign => shl_assign,
    ShrAssign => shr_assign,
}

macro_rules! impl_ops_copy {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'ob, H, S: ?Sized, D, U> std::ops::$trait<U> for GeneralObserver<'ob, H, S, D>
            where
                S: crate::helper::AsDeref<D>,
                D: Unsigned,
                S::Target: std::ops::$trait<U> + Copy,
            {
                type Output = <S::Target as std::ops::$trait<U>>::Output;

                #[inline]
                fn $method(self, rhs: U) -> Self::Output {
                    self.as_deref().$method(rhs)
                }
            }
        )*
    };
}

impl_ops_copy! {
    Add => add,
    Sub => sub,
    Mul => mul,
    Div => div,
    Rem => rem,
    BitAnd => bitand,
    BitOr => bitor,
    BitXor => bitxor,
    Shl => shl,
    Shr => shr,
}

impl<'ob, H, S: ?Sized, D> std::ops::Neg for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D>,
    D: Unsigned,
    S::Target: std::ops::Neg + Copy,
{
    type Output = <S::Target as std::ops::Neg>::Output;

    #[inline]
    fn neg(self) -> Self::Output {
        (*self.as_deref()).neg()
    }
}

impl<'ob, H, S: ?Sized, D> std::ops::Not for GeneralObserver<'ob, H, S, D>
where
    S: crate::helper::AsDeref<D>,
    D: Unsigned,
    S::Target: std::ops::Not + Copy,
{
    type Output = <S::Target as std::ops::Not>::Output;

    #[inline]
    fn not(self) -> Self::Output {
        (*self.as_deref()).not()
    }
}

macro_rules! impl_partial_eq {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'ob, H, S: ?Sized, D> PartialEq<$ty> for GeneralObserver<'ob, H, S, D>
            where
                S: crate::helper::AsDeref<D, Target = $ty>,
                D: Unsigned,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    (***self).as_deref().eq(other)
                }
            }
        )*
    };
}

impl_partial_eq! {
    (), usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    core::net::IpAddr, core::net::Ipv4Addr, core::net::Ipv6Addr,
    core::net::SocketAddr, core::net::SocketAddrV4, core::net::SocketAddrV6,
    core::time::Duration, std::time::SystemTime,
}

#[cfg(feature = "chrono")]
impl_partial_eq! {
    chrono::Days, chrono::FixedOffset, chrono::Month, chrono::Months, chrono::IsoWeek,
    chrono::NaiveDate, chrono::NaiveDateTime, chrono::NaiveTime, chrono::NaiveWeek,
    chrono::TimeDelta, chrono::Utc, chrono::Weekday, chrono::WeekdaySet,
}

#[cfg(feature = "uuid")]
impl_partial_eq! {
    uuid::Uuid, uuid::NonNilUuid,
}

macro_rules! impl_partial_ord {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'ob, H, S: ?Sized, D> PartialOrd<$ty> for GeneralObserver<'ob, H, S, D>
            where
                S: crate::helper::AsDeref<D, Target = $ty>,
                D: Unsigned,
            {
                #[inline]
                fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                    (***self).as_deref().partial_cmp(other)
                }
            }
        )*
    };
}

impl_partial_ord! {
    (), usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    core::net::IpAddr, core::net::Ipv4Addr, core::net::Ipv6Addr,
    core::net::SocketAddr, core::net::SocketAddrV4, core::net::SocketAddrV6,
    core::time::Duration, std::time::SystemTime,
}

#[cfg(feature = "chrono")]
impl_partial_ord! {
    chrono::Days, chrono::Month, chrono::Months, chrono::IsoWeek,
    chrono::NaiveDate, chrono::NaiveDateTime, chrono::NaiveTime,
    chrono::TimeDelta, chrono::WeekdaySet,
}

#[cfg(feature = "uuid")]
impl_partial_ord! {
    uuid::Uuid, uuid::NonNilUuid,
}

macro_rules! generic_impl_cmp {
    ($(impl $([$($gen:tt)*])? _ for $ty:ty);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? H, S: ?Sized, D> PartialEq<$ty> for GeneralObserver<'ob, H, S, D>
            where
                S: crate::helper::AsDeref<D>,
                D: Unsigned,
                S::Target: PartialEq<$ty>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    (***self).as_deref().eq(other)
                }
            }

            impl<'ob, $($($gen)*,)? H, S: ?Sized, D> PartialOrd<$ty> for GeneralObserver<'ob, H, S, D>
            where
                S: crate::helper::AsDeref<D>,
                D: Unsigned,
                S::Target: PartialOrd<$ty>,
            {
                #[inline]
                fn partial_cmp(&self, other: &$ty) -> Option<std::cmp::Ordering> {
                    (***self).as_deref().partial_cmp(other)
                }
            }
        )*
    };
}

generic_impl_cmp! {
    impl [U] _ for std::marker::PhantomData<U>;
    impl ['a, U] _ for &'a [U];
    impl _ for str;
    impl _ for String;
    impl _ for std::ffi::OsStr;
    impl _ for std::ffi::OsString;
    impl _ for std::path::Path;
    impl _ for std::path::PathBuf;
    impl ['a] _ for std::borrow::Cow<'a, str>;
}

#[cfg(feature = "chrono")]
generic_impl_cmp! {
    impl [Tz: chrono::TimeZone] _ for chrono::DateTime<Tz>;
}
