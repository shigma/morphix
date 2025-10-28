use std::array::from_fn;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, Range, RangeBounds};
use std::slice::{
    ChunkByMut, ChunksExactMut, ChunksMut, IterMut, RChunksExactMut, RChunksMut, RSplitMut, RSplitNMut, SliceIndex,
    SplitInclusiveMut, SplitMut, SplitNMut,
};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, PathSegment};

pub trait SliceObserverInner<'i> {
    type Item: Observer<'i, InnerDepth = Zero, Head: Sized>;
    fn as_slice(&self) -> &[Self::Item];
    fn as_mut_slice(&mut self) -> &mut [Self::Item];
    fn init_empty() -> Self;
    fn init_range(&self, range: Range<usize>, slice: &'i mut [<Self::Item as Observer<'i>>::Head]);
}

impl<'i, O, const N: usize> SliceObserverInner<'i> for [O; N]
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    type Item = O;

    fn as_slice(&self) -> &[O] {
        self
    }

    fn as_mut_slice(&mut self) -> &mut [O] {
        self
    }

    fn init_empty() -> Self {
        from_fn(|_| Default::default())
    }

    fn init_range(&self, _range: Range<usize>, _slice: &'i mut [<Self::Item as Observer<'i>>::Head]) {
        // No need to re-initialize fixed-size array.
    }
}

impl<'i, O> SliceObserverInner<'i> for UnsafeCell<Vec<O>>
where
    O: Observer<'i, InnerDepth = Zero, Head: Sized>,
{
    type Item = O;

    fn as_slice(&self) -> &[Self::Item] {
        unsafe { &*self.get() }
    }

    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        unsafe { &mut *self.get() }
    }

    fn init_empty() -> Self {
        Default::default()
    }

    fn init_range(&self, range: Range<usize>, slice: &'i mut [<Self::Item as Observer<'i>>::Head]) {
        let inner = unsafe { &mut *self.get() };
        if range.end > inner.len() {
            inner.resize_with(range.end, Default::default);
        }
        let ob_iter = inner[range.clone()].iter_mut();
        let value_iter = slice[range].iter_mut();
        for (ob, value) in ob_iter.zip(value_iter) {
            ObserverPointer::set(O::as_ptr(ob), value);
        }
    }
}

pub(super) trait SliceIndexImpl<T: ?Sized, Output: ?Sized> {
    fn start_bound_inclusive(&self) -> usize;
    fn end_bound_exclusive(&self, len: usize) -> usize;
}

impl<T> SliceIndexImpl<[T], T> for usize {
    #[inline]
    fn start_bound_inclusive(&self) -> usize {
        *self
    }

    #[inline]
    fn end_bound_exclusive(&self, _len: usize) -> usize {
        self + 1
    }
}

impl<T, I: SliceIndex<[T], Output = [T]> + RangeBounds<usize>> SliceIndexImpl<[T], [T]> for I {
    #[inline]
    fn start_bound_inclusive(&self) -> usize {
        match self.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        }
    }

    #[inline]
    fn end_bound_exclusive(&self, len: usize) -> usize {
        match self.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        }
    }
}

/// Observer implementation for [slice](core::slice).
///
/// `SliceObserver` provides element-level change tracking for slices through indexing operations.
/// It serves as the foundation for both [`VecObserver`](crate::impls::vec::VecObserver) and
/// [`ArrayObserver`](crate::impls::array::ArrayObserver), enabling them to track mutations to
/// individual elements.
pub struct SliceObserver<'i, V, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    pub(super) obs: V,
    mutation: Option<usize>,
    phantom: PhantomData<&'i mut D>,
}

impl<'i, V, S: ?Sized, D> SliceObserver<'i, V, S, D> {
    #[inline]
    pub(super) fn __mark_replace(&mut self) {
        self.mutation = None;
    }
}

impl<'i, V, S: ?Sized, D> Default for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i>,
{
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            obs: V::init_empty(),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, V, S: ?Sized, D> Deref for SliceObserver<'i, V, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, V, S: ?Sized, D> DerefMut for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.__mark_replace();
        self.obs = V::init_empty();
        &mut self.ptr
    }
}

impl<'i, V, S> Assignable for SliceObserver<'i, V, S>
where
    V: SliceObserverInner<'i>,
{
    type Depth = Succ<Zero>;
}

impl<'i, V, S: ?Sized, D, O, T> Observer<'i> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            obs: V::init_empty(),
            mutation: Some(value.as_deref().as_ref().len()),
            phantom: PhantomData,
        }
    }
}

impl<'i, V, S: ?Sized, D, O, T> SerializeObserver<'i> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: SerializeObserver<'i, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        let len = this.as_deref().as_ref().len();
        let initial_len = if let Some(initial_len) = this.mutation.replace(len) {
            if len > initial_len {
                mutations.push(Mutation {
                    path: Default::default(),
                    kind: MutationKind::Append(A::serialize_value(&this.as_deref().as_ref()[initial_len..])?),
                });
            }
            initial_len
        } else {
            mutations.push(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref().as_ref())?),
            });
            0
        };
        for (index, observer) in this.obs.as_mut_slice().iter_mut().take(initial_len).enumerate() {
            if let Some(mut mutation) = SerializeObserver::collect::<A>(observer)? {
                mutation.path.push(PathSegment::NegIndex(len - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<'i, V, S: ?Sized, D, O, T> SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: 'i,
{
    #[inline]
    pub(super) fn __initial_len(&mut self) -> usize {
        self.mutation.unwrap_or(0)
    }

    /// Manually trigger [`DerefMut`] down to [`ObserverPointer`].
    pub(super) fn __track_index(&mut self, index: usize) {
        self.obs.init_range(index..index + 1, Self::as_inner(self).as_mut());
        self.obs.as_mut_slice()[index].as_deref_mut_coinductive();
    }

    fn __obs_range(&mut self, range: Range<usize>) -> Option<&mut [O]> {
        let len = self.as_deref().as_ref().len();
        if range.end > len {
            return None;
        }
        self.obs.init_range(range, Self::as_inner(self).as_mut());
        Some(self.obs.as_mut_slice())
    }

    fn __obs_full(&mut self) -> &mut [O] {
        let len = self.as_deref().as_ref().len();
        self.obs.init_range(0..len, Self::as_inner(self).as_mut());
        self.obs.as_mut_slice()
    }

    /// See [`slice::first_mut`].
    pub fn first_mut(&mut self) -> Option<&mut O> {
        self.__obs_range(0..1)?.first_mut()
    }

    /// See [`slice::split_first_mut`].
    pub fn split_first_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.__obs_full().split_first_mut()
    }

    /// See [`slice::split_last_mut`].
    pub fn split_last_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.__obs_full().split_last_mut()
    }

    /// See [`slice::last_mut`].
    pub fn last_mut(&mut self) -> Option<&mut O> {
        let len = self.as_deref().as_ref().len();
        if len == 0 {
            return None;
        }
        self.__obs_range(len - 1..len)?.last_mut()
    }

    /// See [`slice::first_chunk_mut`].
    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        self.__obs_range(0..N)?.first_chunk_mut()
    }

    /// See [`slice::split_first_chunk_mut`].
    pub fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O; N], &mut [O])> {
        self.__obs_full().split_first_chunk_mut()
    }

    /// See [`slice::split_last_chunk_mut`].
    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O], &mut [O; N])> {
        self.__obs_full().split_last_chunk_mut()
    }

    /// See [`slice::last_chunk_mut`].
    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        let len = self.as_deref().as_ref().len();
        if len < N {
            return None;
        }
        self.__obs_range(len - N..len)?.last_chunk_mut()
    }

    /// See [`slice::get_mut`].
    pub fn get_mut(&mut self, index: usize) -> Option<&mut O> {
        self.__obs_range(index..index + 1)?.get_mut(index)
    }

    /// See [`slice::swap`].
    pub fn swap(&mut self, a: usize, b: usize) {
        Self::as_inner(self).as_mut().swap(a, b);
        self.__track_index(a);
        self.__track_index(b);
    }

    /// See [`slice::iter_mut`].
    pub fn iter_mut(&mut self) -> IterMut<'_, O> {
        self.__obs_full().iter_mut()
    }

    /// See [`slice::chunks_mut`].
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, O> {
        self.__obs_full().chunks_mut(chunk_size)
    }

    /// See [`slice::chunks_exact_mut`].
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, O> {
        self.__obs_full().chunks_exact_mut(chunk_size)
    }

    /// See [`slice::as_chunks_mut`].
    pub fn as_chunks_mut<const N: usize>(&mut self) -> (&mut [[O; N]], &mut [O]) {
        self.__obs_full().as_chunks_mut()
    }

    /// See [`slice::as_rchunks_mut`].
    pub fn as_rchunks_mut<const N: usize>(&mut self) -> (&mut [O], &mut [[O; N]]) {
        self.__obs_full().as_rchunks_mut()
    }

    /// See [`slice::rchunks_mut`].
    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, O> {
        self.__obs_full().rchunks_mut(chunk_size)
    }

    /// See [`slice::rchunks_exact_mut`].
    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, O> {
        self.__obs_full().rchunks_exact_mut(chunk_size)
    }

    /// See [`slice::chunk_by_mut`].
    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, O, F>
    where
        F: FnMut(&O, &O) -> bool,
    {
        self.__obs_full().chunk_by_mut(pred)
    }

    /// See [`slice::split_at_mut`].
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [O], &mut [O]) {
        self.__obs_full().split_at_mut(mid)
    }

    /// See [`slice::split_at_mut_checked`].
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [O], &mut [O])> {
        self.__obs_full().split_at_mut_checked(mid)
    }

    /// See [`slice::split_mut`].
    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__obs_full().split_mut(pred)
    }

    /// See [`slice::split_inclusive_mut`].
    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__obs_full().split_inclusive_mut(pred)
    }

    /// See [`slice::rsplit_mut`].
    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__obs_full().rsplit_mut(pred)
    }

    /// See [`slice::splitn_mut`].
    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__obs_full().splitn_mut(n, pred)
    }

    /// See [`slice::rsplitn_mut`].
    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__obs_full().rsplitn_mut(n, pred)
    }
}

impl<'i, V, S: ?Sized, D, O, T> Debug for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.as_deref().as_ref()).finish()
    }
}

impl<'i, V, S: ?Sized, D, O, T, U> PartialEq<U> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().as_ref().eq(other)
    }
}

impl<'i, V, S: ?Sized, D, O, T, U> PartialOrd<U> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
    [T]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().as_ref().partial_cmp(other)
    }
}

impl<'i, V, S: ?Sized, D, O, T, I> Index<I> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        let len = self.as_deref().as_ref().len();
        let start = index.start_bound_inclusive();
        let end = index.end_bound_exclusive(len);
        if end > len {
            panic!("index out of bounds");
        }
        self.obs.init_range(start..end, Self::as_inner(self).as_mut());
        self.obs.as_slice().index(index)
    }
}

impl<'i, V, S: ?Sized, D, O, T, I> IndexMut<I> for SliceObserver<'i, V, S, D>
where
    V: SliceObserverInner<'i, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'i,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'i, InnerDepth = Zero, Head = T> + 'i,
    T: 'i,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let len = self.as_deref().as_ref().len();
        let start = index.start_bound_inclusive();
        let end = index.end_bound_exclusive(len);
        if end > len {
            panic!("index out of bounds");
        }
        self.obs.init_range(start..end, Self::as_inner(self).as_mut());
        self.obs.as_mut_slice().index_mut(index)
    }
}

impl<T: Observe> Observe for [T] {
    type Observer<'i, S, D>
        = SliceObserver<'i, UnsafeCell<Vec<T::Observer<'i, T, Zero>>>, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::observe::{ObserveExt, SerializeObserverExt, ShallowObserver};
    use crate::{JsonAdapter, MutationKind};

    #[derive(Debug, Serialize, Clone, PartialEq, Eq)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'i, S, D>
            = ShallowObserver<'i, S, D>
        where
            Self: 'i,
            D: Unsigned,
            S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

        type Spec = DefaultSpec;
    }

    #[test]
    fn index_by_usize() {
        let slice: &mut [Number] = &mut [Number(0), Number(1), Number(2)];
        let mut ob = slice.observe();
        assert_eq!(ob[2], Number(2));
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());
        **ob[2] = Number(42);
        assert_eq!(ob[2], Number(42));
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.path, vec![(-1).into()].into());
        assert_eq!(mutation.kind, MutationKind::Replace(json!(Number(42))));
    }

    #[test]
    fn get_mut() {
        let slice: &mut [Number] = &mut [Number(0), Number(1), Number(2)];
        let mut ob = slice.observe();
        assert_eq!(*ob.get_mut(2).unwrap(), Number(2));
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());
        ***ob.get_mut(2).unwrap() = Number(42);
        assert_eq!(*ob.get_mut(2).unwrap(), Number(42));
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.path, vec![(-1).into()].into());
        assert_eq!(mutation.kind, MutationKind::Replace(json!(Number(42))));
    }

    #[test]
    fn swap() {
        let slice: &mut [Number] = &mut [Number(0), Number(1), Number(2)];
        let mut ob = slice.observe();
        ob.swap(0, 1);
        assert_eq!(**ob, [Number(1), Number(0), Number(2)]);
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(
            mutation,
            Mutation {
                path: vec![].into(),
                kind: MutationKind::Batch(vec![
                    Mutation {
                        path: vec![(-3).into()].into(),
                        kind: MutationKind::Replace(json!(Number(1))),
                    },
                    Mutation {
                        path: vec![(-2).into()].into(),
                        kind: MutationKind::Replace(json!(Number(0))),
                    }
                ]),
            }
        );
    }
}
