use crate::Observe;
use crate::helper::{AsDeref, AsDerefMut, Succ, Unsigned};
use crate::observe::general::ReplaceHandler;
use crate::observe::{DefaultSpec, GeneralHandler, GeneralObserver, Observer, SnapshotObserver, SnapshotSpec};

/// A general observer implementation for reference types.
///
/// This observer stores the initial pointer value and compares it with the current value at
/// collection time using [`std::ptr::eq`]. A change is detected if the reference now points to a
/// different memory location.
///
/// ## Limitations
///
/// - **False negatives**: If the referenced value contains interior mutability and is mutated
///   without changing the pointer, the mutation will not be detected.
/// - **False positives**: If two distinct references point to equal values, changing from one to
///   the other will be detected as a change, even if the underlying value is effectively the same.
///
/// ## When to Use
///
/// Use `RefObserver` for types where:
/// 1. Pointer identity is a reliable indicator of value identity
/// 2. Value comparison is expensive or unavailable
/// 3. The type has no interior mutability
///
/// For types where value comparison is cheap and preferred, consider using [`SnapshotObserver`] for
/// references.
pub type RefObserver<'a, 'ob, S, D> = GeneralObserver<'ob, RefHandler<'a, <S as AsDeref<Succ<D>>>::Target>, S, D>;

/// A trait for types whose references can be observed for mutations.
///
/// `RefObserve` provides observation capability for reference types. A type `T` implements
/// `RefObserve` if and only if `&T` implements [`Observe`]. This is analogous to the relationship
/// between [`UnwindSafe`](std::panic::UnwindSafe) and [`RefUnwindSafe`](std::panic::RefUnwindSafe).
///
/// A single type `T` may have many possible [`Observer<'ob, Target = &T>`] implementations in
/// theory, each with different change-tracking strategies. The `RefObserve` trait selects one
/// of these as the default observer to be used by `#[derive(Observe)]` and other generic code
/// that needs an observer for `&T`.
///
/// See also: [`Observe`].
pub trait RefObserve {
    /// The observer type for `&'a Self`.
    ///
    /// This associated type specifies the *default* observer implementation for the type, when used
    /// in contexts where an [`Observe`] implementation is required.
    type Observer<'a, 'ob, S, D>: Observer<'ob, Head = S, InnerDepth = D>
    where
        Self: 'a + 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

    /// Specification type controlling nested reference observation behavior.
    ///
    /// This determines how `&&T`, `&&&T`, etc. are observed. See the [trait
    /// documentation](RefObserve) for available specs.
    type Spec;
}

impl<'a, T: ?Sized> Observe for &'a T
where
    T: RefObserve,
{
    type Observer<'ob, S, D>
        = T::Observer<'a, 'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl<'b, T: ?Sized> RefObserve for &'b T
where
    T: RefObserve + RefObserveImpl<T::Spec>,
{
    type Observer<'a, 'ob, S, D>
        = <T as RefObserveImpl<T::Spec>>::Observer<'a, 'b, 'ob, S, D>
    where
        Self: 'a + 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

    type Spec = T::Spec;
}

/// Helper trait for selecting appropriate observer implementations for `&&T`.
pub trait RefObserveImpl<Spec> {
    type Observer<'a, 'b, 'ob, S, D>: Observer<'ob, Head = S, InnerDepth = D>
    where
        Self: 'b + 'ob,
        'b: 'a,
        D: Unsigned,
        S: AsDerefMut<D, Target = &'a &'b Self> + ?Sized + 'ob;
}

impl<T: ?Sized> RefObserveImpl<DefaultSpec> for T
where
    T: RefObserve<Spec = DefaultSpec>,
{
    type Observer<'a, 'b, 'ob, S, D>
        = RefObserver<'a, 'ob, S, D>
    where
        Self: 'b + 'ob,
        'b: 'a,
        D: Unsigned,
        S: AsDerefMut<D, Target = &'a &'b Self> + ?Sized + 'ob;
}

impl<T: ?Sized> RefObserveImpl<SnapshotSpec> for T
where
    T: PartialEq + RefObserve<Spec = SnapshotSpec>,
{
    type Observer<'a, 'b, 'ob, S, D>
        = SnapshotObserver<'ob, S, D>
    where
        Self: 'b + 'ob,
        'b: 'a,
        D: Unsigned,
        S: AsDerefMut<D, Target = &'a &'b Self> + ?Sized + 'ob;
}

pub struct RefHandler<'a, T: ?Sized> {
    ptr: Option<&'a T>,
}

impl<'a, T: ?Sized> GeneralHandler<&'a T> for RefHandler<'a, T> {
    type Spec = DefaultSpec;

    #[inline]
    fn uninit() -> Self {
        Self { ptr: None }
    }

    #[inline]
    fn observe(value: &mut &'a T) -> Self {
        Self { ptr: Some(value) }
    }

    #[inline]
    fn deref_mut(&mut self) {}
}

impl<'a, T: ?Sized> ReplaceHandler<&'a T> for RefHandler<'a, T> {
    #[inline]
    fn flush_replace(&mut self, value: &&'a T) -> bool {
        !std::ptr::eq(*value, unsafe { self.ptr.unwrap_unchecked() })
    }
}

macro_rules! impl_ref_observe {
    ($($ty_self:ty),* $(,)?) => {
        $(
            impl RefObserve for $ty_self {
                type Observer<'a, 'ob, S, D>
                    = GeneralObserver<'ob, RefHandler<'a, $ty_self>, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

                type Spec = DefaultSpec;
            }
        )*
    };
}

impl_ref_observe! {
    str, // TODO: better implementation for str
}

macro_rules! impl_snapshot_ref_observe {
    ($($ty_self:ty),* $(,)?) => {
        $(
            impl RefObserve for $ty_self {
                type Observer<'a, 'ob, S, D>
                    = SnapshotObserver<'ob, S, D>
                where
                    Self: 'ob,
                    D: Unsigned,
                    S: AsDerefMut<D, Target = &'a Self> + ?Sized + 'ob;

                type Spec = SnapshotSpec;
            }
        )*
    };
}

impl_snapshot_ref_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool, char,
    ::core::net::IpAddr, ::core::net::Ipv4Addr, ::core::net::Ipv6Addr,
    ::core::net::SocketAddr, ::core::net::SocketAddrV4, ::core::net::SocketAddrV6,
    ::core::time::Duration, ::std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    // TODO: enable tests after further implementation
    // use crate::adapter::Json;
    // use crate::observe::{ObserveExt, SerializeObserverExt};

    // #[test]
    // fn test_ptr_eq() {
    //     let a = 42u8;
    //     let b = 42u8;
    //     let mut ptr = &a;
    //     let mut ob = ptr.__observe();
    //     **ob = &a;
    //     let Json(mutation) = ob.collect().unwrap();
    //     assert!(mutation.is_none());
    //     **ob = &b;
    //     let Json(mutation) = ob.collect().unwrap();
    //     assert!(mutation.is_some());
    // }
}
