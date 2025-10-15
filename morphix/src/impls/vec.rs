use std::cell::UnsafeCell;
use std::collections::TryReserveError;
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Bound, Deref, DerefMut, Index, IndexMut, RangeBounds};
use std::slice::SliceIndex;

use serde::Serialize;

use crate::helper::{Assignable, RangeLike};
use crate::observe::{DefaultSpec, MutationState, StatefulObserver};
use crate::{Adapter, Batch, Mutation, MutationKind, Observe, Observer};

/// An observer for [`Vec`] that tracks both replacements and appends.
///
/// `VecObserver` provides special handling for vector append operations, distinguishing them from
/// complete replacements for efficiency.
///
/// ## Supported Operations
///
/// The following mutations are tracked as [`Append`](MutationKind::Append):
///
/// - [Vec::push](std::vec::Vec::push)
/// - [Vec::append](std::vec::Vec::append)
/// - [Vec::extend](std::iter::Extend)
/// - [Vec::extend_from_slice](std::vec::Vec::extend_from_slice)
/// - [Vec::extend_from_within](std::vec::Vec::extend_from_within)
pub struct VecObserver<'i, O: Observer<'i, Target: Sized>> {
    ptr: *mut Vec<O::Target>,
    mutation: Option<MutationState>,
    obs: UnsafeCell<Vec<O>>,
    phantom: PhantomData<&'i mut O::Target>,
}

impl<'i, O: Observer<'i, Target: Sized>> Default for VecObserver<'i, O> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> Deref for VecObserver<'i, O> {
    type Target = Vec<O::Target>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> DerefMut for VecObserver<'i, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        take(&mut self.obs);
        self.__as_mut()
    }
}

impl<'i, O: Observer<'i, Target: Sized>> Assignable for VecObserver<'i, O> {}

impl<'i, O: Observer<'i, Target: Serialize + Sized>> Observer<'i> for VecObserver<'i, O> {
    type Spec = DefaultSpec;

    fn inner(this: &Self) -> *mut Self::Target {
        this.ptr
    }

    #[inline]
    fn observe(value: &'i mut Vec<O::Target>) -> Self {
        Self {
            ptr: value,
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }

    unsafe fn collect_unchecked<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        let mut max_index = None;
        if let Some(mutation) = Self::mutation_state(&mut this).take() {
            mutations.push(Mutation {
                path_rev: vec![],
                operation: match mutation {
                    MutationState::Replace => MutationKind::Replace(A::serialize_value(&*this)?),
                    MutationState::Append(start_index) => {
                        max_index = Some(start_index);
                        MutationKind::Append(A::serialize_value(&(*this)[start_index..])?)
                    }
                },
            });
        };
        let obs = take(unsafe { &mut *this.obs.get() });
        for (index, observer) in obs.into_iter().enumerate() {
            if let Some(max_index) = max_index
                && index >= max_index
            {
                // already included in append
                continue;
            }
            if let Some(mut mutation) = Observer::collect::<A>(observer)? {
                mutation.path_rev.push(index.to_string().into());
                mutations.push(mutation);
            }
        }
        Ok(Batch::build(mutations))
    }
}

impl<'i, O: Observer<'i, Target: Sized>> StatefulObserver<'i> for VecObserver<'i, O> {
    #[inline]
    fn mutation_state(this: &mut Self) -> &mut Option<MutationState> {
        &mut this.mutation
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'i>
        = VecObserver<'i, T::Observer<'i>>
    where
        Self: 'i;
}

impl<'i, O: Observer<'i, Target: Sized>> VecObserver<'i, O> {
    #[inline]
    fn __as_mut(&mut self) -> &mut Vec<O::Target> {
        unsafe { &mut *self.ptr }
    }

    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.__as_mut().reserve(additional);
    }

    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.__as_mut().reserve_exact(additional);
    }

    #[inline]
    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.__as_mut().try_reserve(additional)
    }

    #[inline]
    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.__as_mut().try_reserve_exact(additional)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.__as_mut().shrink_to_fit();
    }

    #[inline]
    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.__as_mut().shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: O::Target) {
        Self::mark_append(self, self.len());
        self.__as_mut().push(value);
    }

    pub fn append(&mut self, other: &mut Vec<O::Target>) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__as_mut().append(other);
    }
}

impl<'i, T: Observe + Clone, O: Observer<'i, Target = T>> VecObserver<'i, O> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__as_mut().extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Self::mark_append(self, self.len());
        self.__as_mut().extend_from_within(range);
    }
}

impl<'i, O: Observer<'i, Target: Sized>, U> Extend<U> for VecObserver<'i, O>
where
    Vec<O::Target>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.__as_mut().extend(other);
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default> Index<usize> for VecObserver<'i, O> {
    type Output = O;

    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs: &mut Vec<O> = unsafe { &mut *self.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        obs[index] = O::observe(value);
        &obs[index]
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default> IndexMut<usize> for VecObserver<'i, O> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        if index >= obs.len() {
            obs.resize_with(index + 1, Default::default);
        }
        obs[index] = O::observe(value);
        &mut obs[index]
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default, I> Index<I> for VecObserver<'i, O>
where
    I: RangeLike<usize> + SliceIndex<[O], Output = [O]>,
{
    type Output = [O];

    fn index(&self, index: I) -> &Self::Output {
        let obs = unsafe { &mut *self.obs.get() };
        let start = match index.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match index.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => self.len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = unsafe { &mut (&mut *self.ptr)[start + i] };
            *obs_item = O::observe(value);
        }
        &obs[index]
    }
}

impl<'i, O: Observer<'i, Target: Sized> + Default, I> IndexMut<I> for VecObserver<'i, O>
where
    I: RangeLike<usize> + SliceIndex<[O], Output = [O]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        let obs = unsafe { &mut *self.obs.get() };
        let start = match index.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };
        let end = match index.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => self.len(),
        };
        if end > obs.len() {
            obs.resize_with(end, Default::default);
        }
        for (i, obs_item) in obs[start..end].iter_mut().enumerate() {
            let value = unsafe { &mut (&mut *self.ptr)[start + i] };
            *obs_item = O::observe(value);
        }
        &mut obs[index]
    }
}
