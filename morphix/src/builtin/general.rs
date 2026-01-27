use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::{AsNormalized, Succ, Unsigned, Zero};
use crate::observe::{Observer, ObserverPointer, SerializeObserver};
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
pub struct GeneralObserver<'ob, H, S: ?Sized, N = Zero> {
    ptr: ObserverPointer<S>,
    handler: H,
    phantom: PhantomData<&'ob mut N>,
}

impl<'ob, H, S: ?Sized, N> Deref for GeneralObserver<'ob, H, S, N> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, H, S: ?Sized, N> DerefMut for GeneralObserver<'ob, H, S, N>
where
    H: GeneralHandler,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.handler.deref_mut();
        &mut self.ptr
    }
}

impl<'ob, H, S: ?Sized, N> AsNormalized for GeneralObserver<'ob, H, S, N> {
    type OuterDepth = Succ<Zero>;
}

impl<'ob, H, S: ?Sized, N> Observer<'ob> for GeneralObserver<'ob, H, S, N>
where
    N: Unsigned,
    S: crate::helper::AsDeref<N> + 'ob,
    H: GeneralHandler<Target = S::Target>,
{
    type InnerDepth = N;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: ObserverPointer::uninit(),
            handler: H::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        ObserverPointer::set(this, value);
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            handler: H::observe(value.as_deref()),
            phantom: PhantomData,
        }
    }
}

impl<'ob, H, S: ?Sized, N> SerializeObserver<'ob> for GeneralObserver<'ob, H, S, N>
where
    N: Unsigned,
    S: crate::helper::AsDeref<N> + 'ob,
    H: SerializeHandler<Target = S::Target>,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        unsafe { this.handler.flush::<A>(this.ptr.as_deref()) }
    }
}

macro_rules! impl_fmt {
    ($($trait:ident),* $(,)?) => {
        $(
            impl<'ob, H, S, N: Unsigned> std::fmt::$trait for GeneralObserver<'ob, H, S, N>
            where
                S: crate::helper::AsDeref<N, Target: std::fmt::$trait> + ?Sized,
                H: GeneralHandler<Target = S::Target>
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

impl<'ob, H, S, N: Unsigned> Debug for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDeref<N, Target: Debug> + ?Sized,
    H: DebugHandler<Target = S::Target>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(H::NAME).field(&self.as_deref()).finish()
    }
}

impl<'ob, H, S, N: Unsigned, I> std::ops::Index<I> for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDeref<N, Target: std::ops::Index<I>> + ?Sized,
    H: GeneralHandler<Target = S::Target>,
{
    type Output = <S::Target as std::ops::Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.as_deref().index(index)
    }
}

impl<'ob, H, S, N: Unsigned, I> std::ops::IndexMut<I> for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDerefMut<N, Target: std::ops::IndexMut<I>> + ?Sized,
    H: GeneralHandler<Target = S::Target>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        Observer::as_inner(self).index_mut(index)
    }
}

impl<'ob, H, S, N: Unsigned, U: ?Sized> PartialEq<U> for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDeref<N, Target: PartialEq<U>> + ?Sized,
    H: GeneralHandler<Target = S::Target>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'ob, H, S, N: Unsigned, U: ?Sized> PartialOrd<U> for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDeref<N, Target: PartialOrd<U>> + ?Sized,
    H: GeneralHandler<Target = S::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'ob, H, S, N: Unsigned, U> std::ops::$trait<U> for GeneralObserver<'ob, H, S, N>
            where
                S: crate::helper::AsDerefMut<N, Target: std::ops::$trait<U>> + ?Sized,
                H: GeneralHandler<Target = S::Target>,
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
            impl<'ob, H, S, N: Unsigned, U> std::ops::$trait<U> for GeneralObserver<'ob, H, S, N>
            where
                S: crate::helper::AsDeref<N, Target: std::ops::$trait<U> + Copy> + ?Sized,
                H: GeneralHandler<Target = S::Target>,
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

impl<'ob, H, S, N: Unsigned> std::ops::Neg for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDeref<N, Target: std::ops::Neg + Copy> + ?Sized,
    H: GeneralHandler<Target = S::Target>,
{
    type Output = <S::Target as std::ops::Neg>::Output;

    #[inline]
    fn neg(self) -> Self::Output {
        (*self.as_deref()).neg()
    }
}

impl<'ob, H, S, N: Unsigned> std::ops::Not for GeneralObserver<'ob, H, S, N>
where
    S: crate::helper::AsDeref<N, Target: std::ops::Not + Copy> + ?Sized,
    H: GeneralHandler<Target = S::Target>,
{
    type Output = <S::Target as std::ops::Not>::Output;

    #[inline]
    fn not(self) -> Self::Output {
        (*self.as_deref()).not()
    }
}
