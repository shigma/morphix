use std::cell::UnsafeCell;
use std::collections::{HashMap, TryReserveError};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};

use serde::{Serialize, Serializer};

use crate::{Adapter, Batch, Mutation, MutationKind, MutationState, Observe, Observer, StatefulObserver};

/// An observer for [Vec](std::vec::Vec) that tracks both replacements and appends.
///
/// `VecObserver` provides special handling for vector append operations,
/// distinguishing them from complete replacements for efficiency.
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
pub struct VecObserver<'i, T: Observe> {
    ptr: *mut Vec<T>,
    mutation: Option<MutationState>,
    obs: UnsafeCell<HashMap<usize, T::Observer<'i>>>,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T: Observe> Deref for VecObserver<'i, T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T: Observe> DerefMut for VecObserver<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        take(&mut self.obs);
        self.as_mut()
    }
}

impl<'i, T: Observe> Observer<'i, Vec<T>> for VecObserver<'i, T> {
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

impl<'i, T: Observe> StatefulObserver<'i, Vec<T>> for VecObserver<'i, T> {
    fn mutation_state(this: &mut Self) -> &mut Option<MutationState> {
        &mut this.mutation
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Observer<'i>
        = VecObserver<'i, T>
    where
        Self: 'i;

    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        self[start_index..].serialize(serializer)
    }
}

impl<'i, T: Observe> VecObserver<'i, T> {
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

impl<'i, T: Observe + Clone> VecObserver<'i, T> {
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

impl<'i, T: Observe> Extend<T> for VecObserver<'i, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.as_mut().extend(other);
    }
}

impl<'i, 'a, T: Observe + Copy + 'a> Extend<&'a T> for VecObserver<'i, T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.as_mut().extend(other);
    }
}

// TODO: handle range
impl<'i, T: Observe> Index<usize> for VecObserver<'i, T> {
    type Output = T::Observer<'i>;
    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        obs.entry(index).or_insert_with(|| value.observe())
    }
}

impl<'i, T: Observe> IndexMut<usize> for VecObserver<'i, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        obs.entry(index).or_insert_with(|| value.observe())
    }
}
