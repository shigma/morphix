use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::spec_impl_ref_observe;
use crate::helper::{AsDerefMut, AsNormalized, Succ, Unsigned, Zero};
use crate::impls::slice::{ObserverSlice, SliceIndexImpl, SliceObserver};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, Mutations, Observe};

impl<'ob, O, const N: usize> ObserverSlice<'ob> for [O; N]
where
    O: Observer<'ob, InnerDepth = Zero, Head: Sized>,
{
    type Item = O;

    #[inline]
    fn as_slice(&self) -> &[O] {
        self
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [O] {
        self
    }

    #[inline]
    fn uninit() -> Self {
        std::array::from_fn(|_| O::uninit())
    }

    #[inline]
    fn init_range(&self, _start: usize, _end: usize, _values: &'ob mut [<Self::Item as Observer<'ob>>::Head]) {
        // No need to re-initialize fixed-size array.
    }
}

/// Observer implementation for arrays `[T; N]`.
pub struct ArrayObserver<'ob, const N: usize, O, S: ?Sized, D = Zero> {
    inner: SliceObserver<'ob, [O; N], (), S, D>,
}

impl<'ob, const N: usize, O, S: ?Sized, D, T> ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
{
    /// See [`array::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &[O] {
        self.inner.__force();
        self.inner.obs.as_slice()
    }

    /// See [`array::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [O] {
        self.inner.__force();
        self.inner.obs.as_mut_slice()
    }

    /// See [`array::each_ref`].
    #[inline]
    pub fn each_ref(&self) -> [&O; N] {
        self.inner.__force();
        self.inner.obs.each_ref()
    }

    /// See [`array::each_mut`].
    #[inline]
    pub fn each_mut(&mut self) -> [&mut O; N] {
        self.inner.__force();
        self.inner.obs.each_mut()
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D> Deref for ArrayObserver<'ob, N, O, S, D> {
    type Target = SliceObserver<'ob, [O; N], (), S, D>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D> DerefMut for ArrayObserver<'ob, N, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D> AsNormalized for ArrayObserver<'ob, N, O, S, D> {
    type OuterDepth = Succ<Succ<Zero>>;
}

impl<'ob, const N: usize, O, S: ?Sized, D, T> Observer<'ob> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            inner: SliceObserver::uninit(),
        }
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            inner: SliceObserver::<[O; N], (), S, D>::observe(value),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        unsafe { SliceObserver::refresh(&mut this.inner, value) }
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T> SerializeObserver<'ob> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'ob,
    O: SerializeObserver<'ob, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        unsafe { SliceObserver::flush_unchecked::<A>(&mut this.inner) }
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D> Debug for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ArrayObserver").field(&self.as_deref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for [_; N]);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? const N: usize, O, S: ?Sized, D> PartialEq<$ty> for ArrayObserver<'ob, N, O, S, D>
            where
                D: Unsigned,
                S: AsDerefMut<D>,
                S::Target: PartialEq<$ty>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.as_deref().eq(other)
                }
            }
        )*
    };
}

generic_impl_partial_eq! {
    impl [U] PartialEq<[U; N]> for [_; N];
    impl [U] PartialEq<[U]> for [_; N];
    impl ['a, U] PartialEq<&'a U> for [_; N];
    impl ['a, U] PartialEq<&'a mut U> for [_; N];
}

impl<'ob, const N: usize, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<ArrayObserver<'ob, N, O2, S2, D2>>
    for ArrayObserver<'ob, N, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDerefMut<D1>,
    S2: AsDerefMut<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &ArrayObserver<'ob, N, O2, S2, D2>) -> bool {
        self.as_deref().eq(other.as_deref())
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D> Eq for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Eq,
{
}

impl<'ob, const N: usize, O, S: ?Sized, D, U> PartialOrd<[U; N]> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: PartialOrd<[U; N]>,
{
    #[inline]
    fn partial_cmp(&self, other: &[U; N]) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<'ob, const N: usize, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<ArrayObserver<'ob, N, O2, S2, D2>>
    for ArrayObserver<'ob, N, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDerefMut<D1>,
    S2: AsDerefMut<D2>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &ArrayObserver<'ob, N, O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other.as_deref())
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D> Ord for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_deref().cmp(other.as_deref())
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T, I> Index<I> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T, I> IndexMut<I> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T: Observe, const N: usize> Observe for [T; N] {
    type Observer<'ob, S, D>
        = ArrayObserver<'ob, N, T::Observer<'ob, T, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

spec_impl_ref_observe! {
    ArrayRefObserveImpl,
    [Self; N],
    [T; N],
    const N: usize,
}

impl<T: Snapshot, const N: usize> Snapshot for [T; N] {
    type Snapshot = [T::Snapshot; N];

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        std::array::from_fn(|i| self[i].to_snapshot())
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        (0..N).all(|i| self[i].eq_snapshot(&snapshot[i]))
    }
}
