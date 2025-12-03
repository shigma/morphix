use std::array::from_fn;
use std::cell::UnsafeCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::{
    ChunkByMut, ChunksExactMut, ChunksMut, IterMut, RChunksExactMut, RChunksMut, RSplitMut, RSplitNMut, SliceIndex,
    SplitInclusiveMut, SplitMut, SplitNMut,
};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, PathSegment};

pub trait ObserverSlice<'ob> {
    type Item: Observer<'ob, InnerDepth = Zero, Head: Sized>;
    fn uninit() -> Self;
    fn as_slice(&self) -> &[Self::Item];
    fn as_mut_slice(&mut self) -> &mut [Self::Item];
    fn init_range(&self, start: usize, end: usize, values: &'ob mut [<Self::Item as Observer<'ob>>::Head]);
}

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
        from_fn(|_| O::uninit())
    }

    #[inline]
    fn init_range(&self, _start: usize, _end: usize, _values: &'ob mut [<Self::Item as Observer<'ob>>::Head]) {
        // No need to re-initialize fixed-size array.
    }
}

impl<'ob, O> ObserverSlice<'ob> for UnsafeCell<Vec<O>>
where
    O: Observer<'ob, InnerDepth = Zero, Head: Sized>,
{
    type Item = O;

    #[inline]
    fn as_slice(&self) -> &[Self::Item] {
        unsafe { &*self.get() }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Item] {
        unsafe { &mut *self.get() }
    }

    #[inline]
    fn uninit() -> Self {
        Default::default()
    }

    #[inline]
    fn init_range(&self, start: usize, end: usize, values: &'ob mut [<Self::Item as Observer<'ob>>::Head]) {
        let inner = unsafe { &mut *self.get() };
        if end > inner.len() {
            inner.resize_with(end, O::uninit);
        }
        let ob_iter = inner[start..end].iter_mut();
        let value_iter = values[start..end].iter_mut();
        for (ob, value) in ob_iter.zip(value_iter) {
            unsafe { Observer::force(ob, value) }
        }
    }
}

pub(super) trait SliceIndexImpl<T: ?Sized, Output: ?Sized> {
    fn start_inclusive(&self) -> usize;
    fn end_exclusive(&self, len: usize) -> usize;
}

impl<T> SliceIndexImpl<[T], T> for usize {
    #[inline]
    fn start_inclusive(&self) -> usize {
        *self
    }

    #[inline]
    fn end_exclusive(&self, _len: usize) -> usize {
        self + 1
    }
}

impl<T, I: SliceIndex<[T], Output = [T]> + RangeBounds<usize>> SliceIndexImpl<[T], [T]> for I {
    #[inline]
    fn start_inclusive(&self) -> usize {
        match self.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        }
    }

    #[inline]
    fn end_exclusive(&self, len: usize) -> usize {
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
pub struct SliceObserver<'ob, V, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    pub(super) obs: V,
    mutation: Option<usize>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, V, S: ?Sized, D> SliceObserver<'ob, V, S, D> {
    #[inline]
    pub(super) fn __mark_replace(&mut self) {
        self.mutation = None;
    }
}

impl<'ob, V, S: ?Sized, D> Deref for SliceObserver<'ob, V, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, V, S: ?Sized, D> DerefMut for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.__mark_replace();
        self.obs = V::uninit();
        &mut self.ptr
    }
}

impl<'ob, V, S> Assignable for SliceObserver<'ob, V, S>
where
    V: ObserverSlice<'ob>,
{
    type Depth = Succ<Zero>;
}

impl<'ob, V, S: ?Sized, D, O, T> Observer<'ob> for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'ob,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            obs: V::uninit(),
            mutation: None,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            obs: V::uninit(),
            mutation: Some(value.as_deref().as_ref().len()),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        ObserverPointer::set(Self::as_ptr(this), value);
    }
}

impl<'ob, V, S: ?Sized, D, O, T> SerializeObserver<'ob> for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'ob,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: SerializeObserver<'ob, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        let mut mutations = vec![];
        let len = this.as_deref().as_ref().len();
        let Some(initial_len) = this.mutation.replace(len) else {
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref().as_ref())?),
            }));
        };
        #[cfg(feature = "append")]
        if len > initial_len {
            mutations.push(Mutation {
                path: Default::default(),
                kind: MutationKind::Append(A::serialize_value(&this.as_deref().as_ref()[initial_len..])?),
            });
        }
        for (index, observer) in this.obs.as_mut_slice().iter_mut().take(initial_len).enumerate() {
            if let Some(mut mutation) = SerializeObserver::collect::<A>(observer)? {
                mutation.path.push(PathSegment::Negative(len - index));
                mutations.push(mutation);
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<'ob, V, S: ?Sized, D, O, T> SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'ob,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
{
    #[inline]
    pub(super) fn __initial_len(&mut self) -> usize {
        self.mutation.unwrap_or(0)
    }

    fn __init_index<I>(&self, index: &I) -> Option<()>
    where
        I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
    {
        let len = self.as_deref().as_ref().len();
        let start = index.start_inclusive();
        let end = index.end_exclusive(len);
        if end > len {
            return None;
        }
        self.obs.init_range(start, end, Self::as_inner(self).as_mut());
        Some(())
    }

    #[inline]
    fn __get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
    {
        self.__init_index(&index)?;
        Some(self.obs.as_slice().index(index))
    }

    #[inline]
    fn __get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
    {
        self.__init_index(&index)?;
        Some(self.obs.as_mut_slice().index_mut(index))
    }

    fn __full_mut(&mut self) -> &mut [O] {
        let len = self.as_deref().as_ref().len();
        self.obs.init_range(0, len, Self::as_inner(self).as_mut());
        self.obs.as_mut_slice()
    }

    /// See [`slice::first_mut`].
    #[inline]
    pub fn first_mut(&mut self) -> Option<&mut O> {
        self.__get_mut(0)
    }

    /// See [`slice::split_first_mut`].
    #[inline]
    pub fn split_first_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.__full_mut().split_first_mut()
    }

    /// See [`slice::split_last_mut`].
    #[inline]
    pub fn split_last_mut(&mut self) -> Option<(&mut O, &mut [O])> {
        self.__full_mut().split_last_mut()
    }

    /// See [`slice::last_mut`].
    #[inline]
    pub fn last_mut(&mut self) -> Option<&mut O> {
        self.__get_mut(..)?.last_mut()
    }

    /// See [`slice::first_chunk_mut`].
    #[inline]
    pub fn first_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        let len = self.as_deref().as_ref().len();
        if len < N {
            return None;
        }
        self.__get_mut(..N)?.first_chunk_mut()
    }

    /// See [`slice::split_first_chunk_mut`].
    #[inline]
    pub fn split_first_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O; N], &mut [O])> {
        self.__full_mut().split_first_chunk_mut()
    }

    /// See [`slice::split_last_chunk_mut`].
    #[inline]
    pub fn split_last_chunk_mut<const N: usize>(&mut self) -> Option<(&mut [O], &mut [O; N])> {
        self.__full_mut().split_last_chunk_mut()
    }

    /// See [`slice::last_chunk_mut`].
    #[inline]
    pub fn last_chunk_mut<const N: usize>(&mut self) -> Option<&mut [O; N]> {
        let len = self.as_deref().as_ref().len();
        if len < N {
            return None;
        }
        self.__get_mut(len - N..)?.last_chunk_mut()
    }

    /// See [`slice::get_mut`].
    #[inline]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut O> {
        self.__get_mut(index)
    }

    /// See [`slice::swap`].
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) {
        Self::as_inner(self).as_mut().swap(a, b);
        self[a].as_deref_mut_coinductive();
        self[b].as_deref_mut_coinductive();
    }

    /// See [`slice::iter_mut`].
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, O> {
        self.__full_mut().iter_mut()
    }

    /// See [`slice::chunks_mut`].
    #[inline]
    pub fn chunks_mut(&mut self, chunk_size: usize) -> ChunksMut<'_, O> {
        self.__full_mut().chunks_mut(chunk_size)
    }

    /// See [`slice::chunks_exact_mut`].
    #[inline]
    pub fn chunks_exact_mut(&mut self, chunk_size: usize) -> ChunksExactMut<'_, O> {
        self.__full_mut().chunks_exact_mut(chunk_size)
    }

    /// See [`slice::as_chunks_mut`].
    #[inline]
    pub fn as_chunks_mut<const N: usize>(&mut self) -> (&mut [[O; N]], &mut [O]) {
        self.__full_mut().as_chunks_mut()
    }

    /// See [`slice::as_rchunks_mut`].
    #[inline]
    pub fn as_rchunks_mut<const N: usize>(&mut self) -> (&mut [O], &mut [[O; N]]) {
        self.__full_mut().as_rchunks_mut()
    }

    /// See [`slice::rchunks_mut`].
    #[inline]
    pub fn rchunks_mut(&mut self, chunk_size: usize) -> RChunksMut<'_, O> {
        self.__full_mut().rchunks_mut(chunk_size)
    }

    /// See [`slice::rchunks_exact_mut`].
    #[inline]
    pub fn rchunks_exact_mut(&mut self, chunk_size: usize) -> RChunksExactMut<'_, O> {
        self.__full_mut().rchunks_exact_mut(chunk_size)
    }

    /// See [`slice::chunk_by_mut`].
    #[inline]
    pub fn chunk_by_mut<F>(&mut self, pred: F) -> ChunkByMut<'_, O, F>
    where
        F: FnMut(&O, &O) -> bool,
    {
        self.__full_mut().chunk_by_mut(pred)
    }

    /// See [`slice::split_at_mut`].
    #[inline]
    pub fn split_at_mut(&mut self, mid: usize) -> (&mut [O], &mut [O]) {
        self.__full_mut().split_at_mut(mid)
    }

    /// See [`slice::split_at_mut_checked`].
    #[inline]
    pub fn split_at_mut_checked(&mut self, mid: usize) -> Option<(&mut [O], &mut [O])> {
        self.__full_mut().split_at_mut_checked(mid)
    }

    /// See [`slice::split_mut`].
    #[inline]
    pub fn split_mut<F>(&mut self, pred: F) -> SplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__full_mut().split_mut(pred)
    }

    /// See [`slice::split_inclusive_mut`].
    #[inline]
    pub fn split_inclusive_mut<F>(&mut self, pred: F) -> SplitInclusiveMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__full_mut().split_inclusive_mut(pred)
    }

    /// See [`slice::rsplit_mut`].
    #[inline]
    pub fn rsplit_mut<F>(&mut self, pred: F) -> RSplitMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__full_mut().rsplit_mut(pred)
    }

    /// See [`slice::splitn_mut`].
    #[inline]
    pub fn splitn_mut<F>(&mut self, n: usize, pred: F) -> SplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__full_mut().splitn_mut(n, pred)
    }

    /// See [`slice::rsplitn_mut`].
    #[inline]
    pub fn rsplitn_mut<F>(&mut self, n: usize, pred: F) -> RSplitNMut<'_, O, F>
    where
        F: FnMut(&O) -> bool,
    {
        self.__full_mut().rsplitn_mut(n, pred)
    }
}

impl<'ob, V, S: ?Sized, D, O, T> Debug for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    T: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SliceObserver").field(&self.as_deref().as_ref()).finish()
    }
}

impl<'ob, V, S: ?Sized, D, O, T, U> PartialEq<U> for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    [T]: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().as_ref().eq(other)
    }
}

impl<'ob, V, S: ?Sized, D, O, T, U> PartialOrd<U> for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T>,
    [T]: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().as_ref().partial_cmp(other)
    }
}

impl<'ob, V, S: ?Sized, D, O, T, I> Index<I> for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'ob,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.__get(index).expect("index out of bounds")
    }
}

impl<'ob, V, S: ?Sized, D, O, T, I> IndexMut<I> for SliceObserver<'ob, V, S, D>
where
    V: ObserverSlice<'ob, Item = O>,
    D: Unsigned,
    S: AsDerefMut<D> + 'ob,
    S::Target: AsRef<[T]> + AsMut<[T]>,
    O: Observer<'ob, InnerDepth = Zero, Head = T> + 'ob,
    T: 'ob,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.__get_mut(index).expect("index out of bounds")
    }
}

impl<T: Observe> Observe for [T] {
    type Observer<'ob, S, D>
        = SliceObserver<'ob, UnsafeCell<Vec<T::Observer<'ob, T, Zero>>>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt, ShallowObserver};

    #[derive(Debug, Serialize, Clone, PartialEq, Eq)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'ob, S, D>
            = ShallowObserver<'ob, S, D>
        where
            Self: 'ob,
            D: Unsigned,
            S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

        type Spec = DefaultSpec;
    }

    #[test]
    fn index_by_usize() {
        let slice: &mut [Number] = &mut [Number(0), Number(1), Number(2)];
        let mut ob = slice.__observe();
        assert_eq!(ob[2], Number(2));
        let Json(mutation) = ob.collect().unwrap();
        assert!(mutation.is_none());
        **ob[2] = Number(42);
        assert_eq!(ob[2], Number(42));
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![(-1).into()].into(),
                kind: MutationKind::Replace(json!(Number(42)))
            })
        );
    }

    #[test]
    fn get_mut() {
        let slice: &mut [Number] = &mut [Number(0), Number(1), Number(2)];
        let mut ob = slice.__observe();
        assert_eq!(*ob.get_mut(2).unwrap(), Number(2));
        let Json(mutation) = ob.collect().unwrap();
        assert!(mutation.is_none());
        ***ob.get_mut(2).unwrap() = Number(42);
        assert_eq!(*ob.get_mut(2).unwrap(), Number(42));
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![(-1).into()].into(),
                kind: MutationKind::Replace(json!(Number(42)))
            })
        );
    }

    #[test]
    fn swap() {
        let slice: &mut [Number] = &mut [Number(0), Number(1), Number(2)];
        let mut ob = slice.__observe();
        ob.swap(0, 1);
        assert_eq!(**ob, [Number(1), Number(0), Number(2)]);
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
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
            })
        );
    }
}
