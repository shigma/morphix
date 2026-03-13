use std::ops::{Range, RangeInclusive};
use std::slice::{GetDisjointMutError, SliceIndex};

pub trait GetDisjointMutIndexImpl<T>: SliceIndex<[T]> + Sized {
    fn get_disjoint_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> Result<[&mut Self::Output; N], GetDisjointMutError>;

    unsafe fn get_disjoint_unchecked_mut<const N: usize>(slice: &mut [T], indices: [Self; N])
    -> [&mut Self::Output; N];
}

impl<T> GetDisjointMutIndexImpl<T> for usize {
    fn get_disjoint_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> Result<[&mut Self::Output; N], GetDisjointMutError> {
        slice.get_disjoint_mut(indices)
    }

    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> [&mut Self::Output; N] {
        unsafe { slice.get_disjoint_unchecked_mut(indices) }
    }
}

impl<T> GetDisjointMutIndexImpl<T> for Range<usize> {
    fn get_disjoint_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> Result<[&mut Self::Output; N], GetDisjointMutError> {
        slice.get_disjoint_mut(indices)
    }

    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> [&mut Self::Output; N] {
        unsafe { slice.get_disjoint_unchecked_mut(indices) }
    }
}

impl<T> GetDisjointMutIndexImpl<T> for RangeInclusive<usize> {
    fn get_disjoint_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> Result<[&mut Self::Output; N], GetDisjointMutError> {
        slice.get_disjoint_mut(indices)
    }

    unsafe fn get_disjoint_unchecked_mut<const N: usize>(
        slice: &mut [T],
        indices: [Self; N],
    ) -> [&mut Self::Output; N] {
        unsafe { slice.get_disjoint_unchecked_mut(indices) }
    }
}
