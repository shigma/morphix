use std::cell::UnsafeCell;
use std::collections::{HashMap, TryReserveError};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Index, IndexMut, RangeBounds};

use serde::{Serialize, Serializer};

use crate::observe::ObInner;
use crate::{Adapter, Change, Ob, Observe, Observer};

pub struct VecObserverInner<'i, T: Observe + 'i> {
    obs: UnsafeCell<HashMap<usize, T::Target<'i>>>,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T: Observe> Default for VecObserverInner<'i, T> {
    fn default() -> Self {
        Self {
            obs: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<'i, T: Observe> ObInner for VecObserverInner<'i, T> {
    fn dump<A: Adapter>(&mut self, changes: &mut Vec<Change<A>>) -> Result<(), A::Error> {
        let obs = take(unsafe { &mut *self.obs.get() });
        for (index, mut ob) in obs {
            if let Some(mut change) = <T as Observe>::Target::<'i>::collect::<A>(&mut ob)? {
                change.path_rev.push(index.to_string().into());
                changes.push(change);
            }
        }
        Ok(())
    }
}

pub type VecObserver<'i, T> = Ob<'i, Vec<T>, VecObserverInner<'i, T>>;

impl<T: Observe> Observe for Vec<T> {
    type Target<'i>
        = VecObserver<'i, T>
    where
        Self: 'i;

    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        self[start_index..].serialize(serializer)
    }
}

impl<'i, T: Observe> VecObserver<'i, T> {
    pub fn reserve(&mut self, additional: usize) {
        self.get_mut().reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.get_mut().reserve_exact(additional);
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.get_mut().try_reserve(additional)
    }

    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.get_mut().try_reserve_exact(additional)
    }

    pub fn shrink_to_fit(&mut self) {
        self.get_mut().shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.get_mut().shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: T) {
        self.mark_append(self.len());
        self.get_mut().push(value);
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        if other.is_empty() {
            return;
        }
        self.mark_append(self.len());
        self.get_mut().append(other);
    }
}

impl<'i, T: Observe + Clone> VecObserver<'i, T> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        self.mark_append(self.len());
        self.get_mut().extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        self.mark_append(self.len());
        self.get_mut().extend_from_within(range);
    }
}

impl<'i, T: Observe> Extend<T> for VecObserver<'i, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        self.mark_append(self.len());
        self.get_mut().extend(other);
    }
}

impl<'i, 'a, T: Observe + Copy + 'a> Extend<&'a T> for VecObserver<'i, T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, other: I) {
        self.mark_append(self.len());
        self.get_mut().extend(other);
    }
}

// TODO: handle range
impl<'i, T: Observe> Index<usize> for VecObserver<'i, T> {
    type Output = T::Target<'i>;
    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.inner.obs.get() };
        obs.entry(index).or_insert_with(|| value.observe())
    }
}

impl<'i, T: Observe> IndexMut<usize> for VecObserver<'i, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.inner.obs.get() };
        obs.entry(index).or_insert_with(|| value.observe())
    }
}
