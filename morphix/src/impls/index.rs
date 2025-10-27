use std::ops::{Bound, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;

use crate::helper::{AsDeref, AsDerefMut, Unsigned, Zero};
use crate::impls::array::ArrayObserver;
use crate::impls::slice::SliceObserver;
use crate::impls::vec::VecObserver;
use crate::observe::{Observer, ObserverPointer};

pub trait SliceObserverImpl<'i, O>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    Self: Observer<'i, Head: AsDeref<Self::InnerDepth, Target: AsMut<[O::Head]>>>,
{
    #[expect(clippy::mut_from_ref)]
    unsafe fn as_obs_unchecked(&self, len: usize) -> &mut [O];

    fn as_obs_checked(&mut self, index: usize) -> Option<&mut [O]> {
        let current_len = Self::as_inner(self).as_mut().len();
        (current_len > index).then(|| unsafe { Self::as_obs_unchecked(self, index + 1) })
    }

    fn as_obs_full(&mut self) -> &mut [O] {
        let len = Self::as_inner(self).as_mut().len();
        unsafe { Self::as_obs_unchecked(self, len) }
    }
}

pub trait SliceIndexImpl<'i, O, Output: ?Sized>: Sized
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[track_caller]
    #[expect(clippy::mut_from_ref)]
    unsafe fn ob_get_impl<P>(self, observer: &P) -> Option<&mut Output>
    where
        P: SliceObserverImpl<'i, O>;

    // fn ob_get<P>(self, observer: &P) -> Option<&Output>
    // where
    //     P: SliceObserverImpl<'i, O>,
    // {
    //     unsafe { self.ob_get_impl(observer) }
    // }

    fn ob_get_mut<P>(self, observer: &mut P) -> Option<&mut Output>
    where
        P: SliceObserverImpl<'i, O>,
    {
        unsafe { self.ob_get_impl(observer) }
    }

    fn ob_index<P>(self, observer: &P) -> &Output
    where
        P: SliceObserverImpl<'i, O>,
    {
        unsafe { self.ob_get_impl(observer).expect("index out of bounds") }
    }

    fn ob_index_mut<P>(self, observer: &mut P) -> &mut Output
    where
        P: SliceObserverImpl<'i, O>,
    {
        unsafe { self.ob_get_impl(observer).expect("index out of bounds") }
    }
}

impl<'i, O> SliceIndexImpl<'i, O, O> for usize
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    unsafe fn ob_get_impl<P>(self, observer: &P) -> Option<&mut O>
    where
        P: SliceObserverImpl<'i, O>,
    {
        let value = P::as_inner(observer).as_mut().get_mut(self)?;
        let obs = unsafe { P::as_obs_unchecked(observer, self + 1) };
        if *O::as_ptr(&obs[self]) != ObserverPointer::new(value) {
            obs[self] = O::observe(value);
        }
        Some(&mut obs[self])
    }
}

impl<'i, O, I> SliceIndexImpl<'i, O, [O]> for I
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: RangeBounds<usize> + SliceIndex<[O], Output = [O]>,
{
    unsafe fn ob_get_impl<P>(self, observer: &P) -> Option<&mut [O]>
    where
        P: SliceObserverImpl<'i, O>,
    {
        let slice = P::as_inner(observer).as_mut();
        let start = match self.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match self.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => slice.len(),
        };
        let slice_iter = slice.get_mut(start..end)?.iter_mut();

        let obs = unsafe { P::as_obs_unchecked(observer, end) };
        let obs_iter = obs[start..end].iter_mut();

        for (value, obs_item) in slice_iter.zip(obs_iter) {
            if *O::as_ptr(obs_item) != ObserverPointer::new(value) {
                *obs_item = O::observe(value);
            }
        }
        Some(&mut obs[self])
    }
}

impl<'i, const N: usize, O, S: ?Sized, D> SliceObserverImpl<'i, O> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head; N]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    unsafe fn as_obs_unchecked(&self, _len: usize) -> &mut [O] {
        let obs = unsafe { &mut *self.obs.get() };
        obs.as_mut()
    }
}

impl<'i, O, S: ?Sized, D, I, Output: ?Sized> Index<I> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: SliceIndexImpl<'i, O, Output> + SliceIndex<[O], Output = Output>,
{
    type Output = Output;

    fn index(&self, index: I) -> &Self::Output {
        index.ob_index(self)
    }
}

impl<'i, O, S: ?Sized, D, I, Output: ?Sized> IndexMut<I> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: SliceIndexImpl<'i, O, Output> + SliceIndex<[O], Output = Output>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.ob_index_mut(self)
    }
}

impl<'i, O, S: ?Sized, D, I, Output: ?Sized> Index<I> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<O::Head>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: SliceIndexImpl<'i, O, Output> + SliceIndex<[O], Output = Output>,
{
    type Output = Output;

    fn index(&self, index: I) -> &Self::Output {
        index.ob_index(&**self)
    }
}

impl<'i, O, S: ?Sized, D, I, Output: ?Sized> IndexMut<I> for VecObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Vec<O::Head>> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: SliceIndexImpl<'i, O, Output> + SliceIndex<[O], Output = Output>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.ob_index_mut(&mut **self)
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, I, Output: ?Sized> Index<I> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head; N]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: SliceIndexImpl<'i, O, Output> + SliceIndex<[O], Output = Output>,
{
    type Output = Output;

    fn index(&self, index: I) -> &Self::Output {
        index.ob_index(self)
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, I, Output: ?Sized> IndexMut<I> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head; N]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    I: SliceIndexImpl<'i, O, Output> + SliceIndex<[O], Output = Output>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        index.ob_index_mut(self)
    }
}
