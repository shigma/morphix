use std::fmt::Debug;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::{AsDeref, AsDerefMut, QuasiObserver, Succ, Unsigned, Zero};
use crate::impls::slice::{SliceIndexImpl, SliceObserver, SliceObserverState};
use crate::observe::{DefaultSpec, Observer, RefObserve, SerializeObserver};
use crate::{Mutations, Observe};

impl<O, const N: usize> SliceObserverState for [O; N]
where
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type Slice = [O::Head; N];
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
    fn observe(slice: &Self::Slice) -> Self {
        slice.each_ref().map(O::observe)
    }

    /// Unlike [`UnsafeCell<Vec<O>>`](core::cell::UnsafeCell) which clears its storage on `DerefMut`
    /// (producing a full [`Replace`](crate::MutationKind::Replace)), the array implementation
    /// triggers [`as_deref_mut_coinductive()`][as_deref_mut_coinductive] on each element,
    /// preserving per-element granularity. This is appropriate because arrays have a fixed,
    /// typically small length — the resulting batch of per-element mutations is bounded and
    /// comparable in size to a whole-array [`Replace`](crate::MutationKind::Replace), while
    /// unchanged elements can be filtered out by the element observer (e.g.,
    /// [`SnapshotObserver`](crate::builtin::SnapshotObserver)).
    ///
    /// [as_deref_mut_coinductive]: crate::helper::AsDerefMutCoinductive::as_deref_mut_coinductive
    #[inline]
    fn mark_replace(&mut self) {
        for ob in self.as_mut_slice() {
            ob.as_deref_mut_coinductive();
        }
    }

    #[inline]
    unsafe fn init_range(&self, _start: usize, _end: usize, _slice: &Self::Slice) {
        // No need to re-initialize fixed-size array.
    }

    fn flush(&mut self, slice: &Self::Slice) -> Mutations
    where
        Self::Item: SerializeObserver,
        <Self::Item as crate::observe::ObserverExt>::Head: Serialize + 'static,
    {
        let mut mutations = Mutations::new();
        let mut is_replace = true;
        for (index, ob) in self.iter_mut().enumerate() {
            let inner_mutations = unsafe { SerializeObserver::flush(ob) };
            is_replace &= inner_mutations.is_replace();
            mutations.insert(index, inner_mutations);
        }
        if is_replace {
            return Mutations::replace(slice.as_ref());
        };
        mutations
    }
}

/// Observer implementation for arrays `[T; N]`.
pub struct ArrayObserver<const N: usize, O, S: ?Sized, D = Zero> {
    inner: SliceObserver<[O; N], S, D>,
}

impl<const N: usize, O, S: ?Sized, D, T> ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    /// See [`array::as_slice`].
    #[inline]
    pub fn as_slice(&self) -> &[O] {
        self.inner.__force_ref()
    }

    /// See [`array::as_mut_slice`].
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [O] {
        self.inner.__force_mut()
    }

    /// See [`array::each_ref`].
    #[inline]
    pub fn each_ref(&self) -> [&O; N] {
        self.inner.__force_ref();
        self.inner.state.each_ref()
    }

    /// See [`array::each_mut`].
    #[inline]
    pub fn each_mut(&mut self) -> [&mut O; N] {
        self.inner.__force_mut();
        self.inner.state.each_mut()
    }
}

impl<const N: usize, O, S: ?Sized, D> Deref for ArrayObserver<N, O, S, D> {
    type Target = SliceObserver<[O; N], S, D>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<const N: usize, O, S: ?Sized, D> DerefMut for ArrayObserver<N, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<const N: usize, O, S: ?Sized, D> QuasiObserver for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    type OuterDepth = Succ<Succ<Zero>>;
    type InnerDepth = D;
}

impl<const N: usize, O, S: ?Sized, D, T> Observer for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = [T; N]>,
    O: Observer<InnerDepth = Zero, Head = T>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            inner: SliceObserver::uninit(),
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        Self {
            inner: SliceObserver::<[O; N], S, D>::observe(head),
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        unsafe { SliceObserver::refresh(&mut this.inner, head) }
    }
}

impl<const N: usize, O, S: ?Sized, D, T> SerializeObserver for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = [T; N]>,
    O: SerializeObserver<InnerDepth = Zero, Head = T>,
    T: Serialize + 'static,
{
    #[inline]
    unsafe fn flush(this: &mut Self) -> Mutations {
        unsafe { SliceObserver::flush(&mut this.inner) }
    }
}

impl<const N: usize, O, S: ?Sized, D> Debug for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ArrayObserver").field(&self.observed_ref()).finish()
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for [_; N]);* $(;)?) => {
        $(
            impl<$($($gen)*,)? const N: usize, O, S: ?Sized, D> PartialEq<$ty> for ArrayObserver<N, O, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
                O: Observer<InnerDepth = Zero, Head: Sized>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.observed_ref().eq(other)
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

impl<const N: usize, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<ArrayObserver<N, O2, S2, D2>>
    for ArrayObserver<N, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &ArrayObserver<N, O2, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<const N: usize, O, S: ?Sized, D> Eq for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
}

impl<const N: usize, O, S: ?Sized, D, U> PartialOrd<[U; N]> for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialOrd<[U; N]>,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn partial_cmp(&self, other: &[U; N]) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other)
    }
}

impl<const N: usize, O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialOrd<ArrayObserver<N, O2, S2, D2>>
    for ArrayObserver<N, O1, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    O1: Observer<InnerDepth = Zero, Head: Sized>,
    O2: Observer<InnerDepth = Zero, Head: Sized>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &ArrayObserver<N, O2, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<const N: usize, O, S: ?Sized, D> Ord for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
    O: Observer<InnerDepth = Zero, Head: Sized>,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
    }
}

impl<const N: usize, O, S: ?Sized, D, T, I> Index<I> for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = [T; N]>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<const N: usize, O, S: ?Sized, D, T, I> IndexMut<I> for ArrayObserver<N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]>,
    O: Observer<InnerDepth = Zero, Head = T>,
    I: SliceIndex<[O]> + SliceIndexImpl<[O], I::Output>,
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T: Observe, const N: usize> Observe for [T; N] {
    type Observer<'ob, S, D>
        = ArrayObserver<N, T::Observer<'ob, T, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

impl<T: Observe, const N: usize> RefObserve for [T; N] {
    type Observer<'ob, S, D>
        = ArrayObserver<N, T::Observer<'ob, T, Zero>, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDeref<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
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

#[cfg(test)]
mod tests {
    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_change_returns_none() {
        let mut arr = [1u32, 2, 3];
        let mut ob = arr.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn index_by_usize() {
        let mut arr = [10u32, 20, 30];
        let mut ob = arr.__observe();
        assert_eq!(ob[1], 20);
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
        **ob[1] = 99;
        assert_eq!(ob[1], 99);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(1, json!(99))));
    }

    #[test]
    fn multiple_index_mutations() {
        let mut arr = [1u32, 2, 3];
        let mut ob = arr.__observe();
        **ob[0] = 10;
        **ob[2] = 30;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(batch!(_, replace!(0, json!(10)), replace!(2, json!(30)))),
        );
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut arr = [1u32, 2, 3];
        let mut ob = arr.__observe();
        ***ob = [4, 5, 6];
        let Json(mutation) = ob.flush().unwrap();
        // DerefMut on array: all elements changed, so the optimization collapses into a single
        // whole-array Replace instead of a batch of per-element mutations.
        assert_eq!(mutation.unwrap(), replace!(_, json!([4, 5, 6])));
    }

    #[test]
    fn deref_mut_same_value_returns_none() {
        let mut arr = [1u32, 2, 3];
        let mut ob = arr.__observe();
        ***ob = [1, 2, 3];
        let Json(mutation) = ob.flush().unwrap();
        // ShallowObserver detects no change on each element.
        assert!(mutation.is_none());
    }

    #[test]
    fn swap() {
        let mut arr = [10u32, 20, 30];
        let mut ob = arr.__observe();
        ob.swap(0, 2);
        assert_eq!(ob, [30, 20, 10]);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation,
            Some(batch!(_, replace!(0, json!(30)), replace!(2, json!(10)))),
        );
    }

    #[test]
    fn nested_string_append() {
        let mut arr = ["hello".to_string(), "world".to_string()];
        let mut ob = arr.__observe();
        ob[0].push_str("!");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(0, json!("!"))));
    }

    #[test]
    fn flush_resets_state() {
        let mut arr = ["a".to_string(), "b".to_string()];
        let mut ob = arr.__observe();
        ob[0].push_str("!");
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_some());
        // Second flush with no new changes returns None.
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none(), "expected None, got {mutation:?}");
    }
}
