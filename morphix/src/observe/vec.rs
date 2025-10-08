use std::cell::UnsafeCell;
use std::collections::{HashMap, TryReserveError};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};

use crate::observe::{MutationState, StatefulObserver};
use crate::{Adapter, Batch, Mutation, MutationKind, Observe, Observer};

/// An observer for [`Vec`] that tracks both replacements and appends.
///
/// `VecObserver` provides special handling for vector append operations, distinguishing them from
/// complete replacements for efficiency.
///
/// ## Supported Operations
///
/// The following mutations are tracked as `Append`:
///
/// - [Vec::push](std::vec::Vec::push)
/// - [Vec::append](std::vec::Vec::append)
/// - [Vec::extend](std::iter::Extend)
/// - [Vec::extend_from_slice](std::vec::Vec::extend_from_slice)
/// - [Vec::extend_from_within](std::vec::Vec::extend_from_within)
pub struct VecObserver<'i, T: Observe, O: Observer<'i, Target = T> = <T as Observe>::Observer<'i>> {
    ptr: *mut Vec<T>,
    mutation: Option<MutationState>,
    obs: UnsafeCell<HashMap<usize, O>>,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T: Observe, O: Observer<'i, Target = T>> Deref for VecObserver<'i, T, O> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T: Observe, O: Observer<'i, Target = T>> DerefMut for VecObserver<'i, T, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        take(&mut self.obs);
        self.as_mut()
    }
}

impl<'i, T: Observe, O: Observer<'i, Target = T>> Observer<'i> for VecObserver<'i, T, O> {
    #[inline]
    fn observe(value: &'i mut Vec<T>) -> Self {
        Self {
            ptr: value as *mut Vec<T>,
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
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
        for (index, observer) in obs {
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

impl<'i, T: Observe, O: Observer<'i, Target = T>> StatefulObserver<'i> for VecObserver<'i, T, O> {
    fn mutation_state(this: &mut Self) -> &mut Option<MutationState> {
        &mut this.mutation
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'i>
        = VecObserver<'i, T, T::Observer<'i>>
    where
        Self: 'i;
}

impl<'i, T: Observe, O: Observer<'i, Target = T>> VecObserver<'i, T, O> {
    #[inline]
    fn as_mut(&mut self) -> &mut Vec<T> {
        unsafe { &mut *self.ptr }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.as_mut().reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.as_mut().reserve_exact(additional);
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.as_mut().try_reserve(additional)
    }

    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.as_mut().try_reserve_exact(additional)
    }

    pub fn shrink_to_fit(&mut self) {
        self.as_mut().shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.as_mut().shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: T) {
        Self::mark_append(self, self.len());
        self.as_mut().push(value);
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.as_mut().append(other);
    }
}

impl<'i, T: Observe + Clone, O: Observer<'i, Target = T>> VecObserver<'i, T, O> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.as_mut().extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Self::mark_append(self, self.len());
        self.as_mut().extend_from_within(range);
    }
}

impl<'i, T: Observe, O: Observer<'i, Target = T>> Extend<T> for VecObserver<'i, T, O> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.as_mut().extend(other);
    }
}

impl<'i, 'a, T: Observe + Copy + 'a, O: Observer<'i, Target = T>> Extend<&'a T> for VecObserver<'i, T, O> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.as_mut().extend(other);
    }
}

// TODO: handle range
impl<'i, T: Observe, O: Observer<'i, Target = T>> Index<usize> for VecObserver<'i, T, O> {
    type Output = O;
    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        obs.entry(index).or_insert_with(|| O::observe(value))
    }
}

impl<'i, T: Observe, O: Observer<'i, Target = T>> IndexMut<usize> for VecObserver<'i, T, O> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        obs.entry(index).or_insert_with(|| O::observe(value))
    }
}
