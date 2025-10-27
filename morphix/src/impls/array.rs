use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::impls::slice::{SliceIndexImpl, SliceObserver};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, Mutation, Observe};

/// An observer for [`[T; N]`](core::array) that tracks both replacements and appends.
pub struct ArrayObserver<'i, const N: usize, O, S: ?Sized, D = Zero> {
    inner: SliceObserver<'i, [O; N], S, D>,
}

// impl<'i, const N: usize, O, S: ?Sized, D> ArrayObserver<'i, N, O, S, D> {
//     pub fn as_slice(&self) -> &[O] {
//         unsafe { &*self.obs.get() }
//     }

//     pub fn as_mut_slice(&mut self) -> &mut [O] {
//         unsafe { &mut *self.obs.get() }
//     }

//     pub fn each_ref(&self) -> [&O; N] {
//         unsafe { &*self.obs.get() }.each_ref()
//     }

//     pub fn each_mut(&mut self) -> [&mut O; N] {
//         unsafe { &mut *self.obs.get() }.each_mut()
//     }
// }

impl<'i, const N: usize, O, S: ?Sized, D> Default for ArrayObserver<'i, N, O, S, D>
where
    O: Default,
{
    #[inline]
    fn default() -> Self {
        Self {
            inner: Default::default(),
        }
    }
}

impl<'i, const N: usize, O, S: ?Sized, D> Deref for ArrayObserver<'i, N, O, S, D> {
    type Target = SliceObserver<'i, [O; N], S, D>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'i, const N: usize, O, S: ?Sized, D> DerefMut for ArrayObserver<'i, N, O, S, D>
where
    O: Default,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'i, const N: usize, O, S> Assignable for ArrayObserver<'i, N, O, S>
where
    O: Default,
{
    type Depth = Succ<Zero>;
}

impl<'i, const N: usize, O, S: ?Sized, D, T> Observer<'i> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Succ<Zero>;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            inner: SliceObserver::<[O; N], S, D>::observe(value),
        }
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T> SerializeObserver<'i> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'i,
    O: SerializeObserver<'i, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        unsafe { SliceObserver::collect_unchecked(&mut this.inner) }
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T> Debug for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ArrayObserver").field(self.as_deref()).finish()
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T, U> PartialEq<U> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T; N]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T, U> PartialOrd<U> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T; N]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T, I> Index<I> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T, I> IndexMut<I> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<T>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T, const N: usize> Observe for [T; N]
where
    T: Observe + ArrayObserveImpl<T, N, T::Spec>,
{
    type Observer<'i, S, D>
        = <T as ArrayObserveImpl<T, N, T::Spec>>::Observer<'i, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = T::Spec;
}

/// Helper trait for selecting appropriate observer implementations for [`[T; N]`](core::array).
#[doc(hidden)]
pub trait ArrayObserveImpl<T: Observe, const N: usize, Spec> {
    /// The observer type for [`[T; N]`](core::array) with the given specification.
    type Observer<'i, S, D>: Observer<'i, Head = S, InnerDepth = D>
    where
        T: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = [T; N]> + ?Sized + 'i;
}

impl<T, const N: usize> ArrayObserveImpl<T, N, DefaultSpec> for T
where
    T: Observe<Spec = DefaultSpec>,
{
    type Observer<'i, S, D>
        = ArrayObserver<'i, N, T::Observer<'i, T, Zero>, S, D>
    where
        T: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = [T; N]> + ?Sized + 'i;
}
