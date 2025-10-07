use std::cell::UnsafeCell;
use std::collections::{HashMap, TryReserveError};
use std::marker::PhantomData;
use std::ops::{Index, IndexMut, RangeBounds};

use crate::{Ob, Observe, Observer, Operation};

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

pub type VecObserver<'i, T> = Ob<'i, Vec<T>, VecObserverInner<'i, T>>;

impl<T: Observe> Observe for Vec<T> {
    type Target<'i>
        = VecObserver<'i, T>
    where
        Self: 'i;
}

impl<'i, T: Observe> VecObserver<'i, T> {
    pub fn reserve(&mut self, additional: usize) {
        Self::get_mut(self).reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        Self::get_mut(self).reserve_exact(additional);
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        Self::get_mut(self).try_reserve(additional)
    }

    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        Self::get_mut(self).try_reserve_exact(additional)
    }

    pub fn shrink_to_fit(&mut self) {
        Self::get_mut(self).shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        Self::get_mut(self).shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: T) {
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).push(value);
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        if other.is_empty() {
            return;
        }
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).append(other);
    }
}

impl<'i, T: Observe + Clone> VecObserver<'i, T> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).extend_from_within(range);
    }
}

impl<'i, T: Observe> Extend<T> for VecObserver<'i, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).extend(other);
    }
}

impl<'i, 'a, T: Observe + Copy + 'a> Extend<&'a T> for VecObserver<'i, T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, other: I) {
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).extend(other);
    }
}

// TODO: handle range
impl<'i, T: Observe> Index<usize> for VecObserver<'i, T> {
    type Output = T::Target<'i>;
    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.inner.obs.get() };
        obs.entry(index)
            .or_insert_with(|| value.observe(self.ctx.as_ref().map(|ctx| ctx.extend(index.to_string().into()))))
    }
}

impl<'i, T: Observe> IndexMut<usize> for VecObserver<'i, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.inner.obs.get() };
        obs.entry(index)
            .or_insert_with(|| value.observe(self.ctx.as_ref().map(|ctx| ctx.extend(index.to_string().into()))))
    }
}
