use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::helper::macros::spec_impl_ref_observe;
use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::impls::slice::{ObserverSlice, SliceIndexImpl, SliceObserver};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, Mutation, Observe};

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

/// Observer implementation for [array](core::array).
///
/// `ArrayObserver` provides element-level change tracking for fixed-size arrays by building on
/// [`SliceObserver`]. It tracks modifications to individual array elements through indexing
/// operations.
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

impl<'ob, const N: usize, O, S> Assignable for ArrayObserver<'ob, N, O, S>
where
    O: Observer<'ob, InnerDepth = Zero, Head: Sized>,
{
    type Depth = Succ<Succ<Zero>>;
}

impl<'ob, const N: usize, O, S: ?Sized, D, T> Observer<'ob> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'ob,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Succ<Zero>;
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
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        unsafe { SliceObserver::collect_unchecked::<A>(&mut this.inner) }
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T> Debug for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ArrayObserver").field(self.as_deref()).finish()
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T, U> PartialEq<U> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    [T; N]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T, U> PartialOrd<U> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    [T; N]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<'ob, const N: usize, O, S: ?Sized, D, T, I> Index<I> for ArrayObserver<'ob, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
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
    S: AsDerefMut<D, Target = Vec<T>> + 'ob,
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
