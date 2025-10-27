use std::ops::{Bound, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;

use crate::helper::{AsDeref, AsDerefMut, Unsigned, Zero};
use crate::impls::array::ArrayObserver;
use crate::impls::slice::SliceObserver;
use crate::impls::vec::VecObserver;
use crate::observe::{Observer, ObserverPointer};

trait SliceObserverImpl<'i, O>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
    Self: Observer<'i, Head: AsDeref<Self::InnerDepth, Target: AsMut<[O::Head]>>>,
{
    #[expect(clippy::mut_from_ref)]
    fn as_obs(this: &Self, max_len: usize) -> &mut [O];
}

trait SliceIndexImpl<'i, O, Output: ?Sized>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[track_caller]
    #[expect(clippy::mut_from_ref)]
    fn index_impl<P>(this: &P, index: Self) -> &mut Output
    where
        P: SliceObserverImpl<'i, O>;
}

impl<'i, O> SliceIndexImpl<'i, O, O> for usize
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized + 'i>,
{
    fn index_impl<P>(this: &P, index: Self) -> &mut O
    where
        P: SliceObserverImpl<'i, O>,
    {
        let value = &mut P::as_inner(this).as_mut()[index];
        let obs = P::as_obs(this, index + 1);
        if *O::as_ptr(&obs[index]) != ObserverPointer::new(value) {
            obs[index] = O::observe(value);
        }
        &mut obs[index]
    }
}

impl<'i, O, I> SliceIndexImpl<'i, O, [O]> for I
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized + 'i>,
    I: RangeBounds<usize> + SliceIndex<[O], Output = [O]>,
{
    fn index_impl<P>(this: &P, index: Self) -> &mut [O]
    where
        P: SliceObserverImpl<'i, O>,
    {
        let slice = P::as_inner(this).as_mut();
        let start = match index.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match index.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => slice.len(),
        };

        let obs = P::as_obs(this, end);
        let obs_iter = obs[start..end].iter_mut();
        let slice_iter = slice[start..end].iter_mut();

        for (value, obs_item) in slice_iter.zip(obs_iter) {
            if *O::as_ptr(obs_item) != ObserverPointer::new(value) {
                *obs_item = O::observe(value);
            }
        }
        &mut obs[index]
    }
}

impl<'i, O, S: ?Sized, D> SliceObserverImpl<'i, O> for SliceObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn as_obs(this: &Self, len: usize) -> &mut [O] {
        let obs = unsafe { &mut *this.obs.get() };
        if len >= obs.len() {
            obs.resize_with(len, Default::default);
        }
        obs.as_mut()
    }
}

impl<'i, const N: usize, O, S: ?Sized, D> SliceObserverImpl<'i, O> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [O::Head; N]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn as_obs(this: &Self, _len: usize) -> &mut [O] {
        let obs = unsafe { &mut *this.obs.get() };
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
        SliceIndexImpl::index_impl(self, index)
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
        SliceIndexImpl::index_impl(self, index)
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
        SliceIndexImpl::index_impl(&**self, index)
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
        SliceIndexImpl::index_impl(&**self, index)
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
        SliceIndexImpl::index_impl(self, index)
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
        SliceIndexImpl::index_impl(self, index)
    }
}
