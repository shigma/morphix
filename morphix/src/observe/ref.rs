use crate::helper::{AsDeref, AsDerefMut, Succ, Unsigned};
use crate::observe::general::ReplaceHandler;
use crate::observe::{
    DefaultSpec, GeneralHandler, GeneralObserver, Observer, RefObserve, SnapshotObserver, SnapshotSpec,
};

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
/// Use [`RefObserver`] for types where:
/// 1. Pointer identity is a reliable indicator of value identity
/// 2. Value comparison is expensive or unavailable
/// 3. The type has no interior mutability
///
/// For types where value comparison is cheap and preferred, consider using [`SnapshotObserver`] for
/// references.
pub type RefObserver<'a, 'ob, S, D> = GeneralObserver<'ob, RefHandler<'a, <S as AsDeref<Succ<D>>>::Target>, S, D>;

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

impl<'a, T: ?Sized> GeneralHandler for RefHandler<'a, T> {
    type Target = &'a T;
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

impl<'a, T: ?Sized> ReplaceHandler for RefHandler<'a, T> {
    #[inline]
    fn flush_replace(&mut self, value: &&'a T) -> bool {
        !std::ptr::eq(*value, unsafe { self.ptr.unwrap_unchecked() })
    }
}
