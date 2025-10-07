use std::cell::UnsafeCell;
use std::collections::{HashMap, TryReserveError};
use std::marker::PhantomData;
use std::mem::take;
use std::ops::{Deref, DerefMut, Index, IndexMut, RangeBounds};

use serde::{Serialize, Serializer};

use crate::observe::Mutation;
use crate::{Adapter, Batch, Change, MutationObserver, Observe, Observer, Operation};

pub struct VecOb<'i, T: Observe> {
    ptr: *mut Vec<T>,
    mutation: Option<Mutation>,
    obs: UnsafeCell<HashMap<usize, T::Target<'i>>>,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T: Observe> Deref for VecOb<'i, T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T: Observe> DerefMut for VecOb<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        take(&mut self.obs);
        self.__mut()
    }
}

impl<'i, T: Observe> Observer<'i, Vec<T>> for VecOb<'i, T> {
    #[inline]
    fn observe(value: &'i mut Vec<T>) -> Self {
        Self {
            ptr: value as *mut Vec<T>,
            mutation: None,
            obs: Default::default(),
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(this: &mut Self) -> Result<Option<Change<A>>, A::Error> {
        let mut changes = vec![];
        if let Some(mutation) = Self::mutation(this).take() {
            changes.push(Change {
                path_rev: vec![],
                operation: match mutation {
                    Mutation::Replace => Operation::Replace(A::new_replace(&**this)?),
                    Mutation::Append(start_index) => Operation::Append(A::new_append(&**this, start_index)?),
                },
            });
        };
        let obs = take(unsafe { &mut *this.obs.get() });
        for (index, mut ob) in obs {
            if let Some(mut change) = <T as Observe>::Target::<'i>::collect::<A>(&mut ob)? {
                change.path_rev.push(index.to_string().into());
                changes.push(change);
            }
        }
        Ok(Batch::build(changes))
    }
}

impl<'i, T: Observe> MutationObserver<'i, Vec<T>> for VecOb<'i, T> {
    fn mutation(this: &mut Self) -> &mut Option<Mutation> {
        &mut this.mutation
    }
}

impl<T: Observe> Observe for Vec<T> {
    type Target<'i>
        = VecOb<'i, T>
    where
        Self: 'i;

    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        self[start_index..].serialize(serializer)
    }
}

impl<'i, T: Observe> VecOb<'i, T> {
    #[inline]
    fn __mut(&mut self) -> &mut Vec<T> {
        unsafe { &mut *self.ptr }
    }

    pub fn reserve(&mut self, additional: usize) {
        self.__mut().reserve(additional);
    }

    pub fn reserve_exact(&mut self, additional: usize) {
        self.__mut().reserve_exact(additional);
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.__mut().try_reserve(additional)
    }

    pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError> {
        self.__mut().try_reserve_exact(additional)
    }

    pub fn shrink_to_fit(&mut self) {
        self.__mut().shrink_to_fit();
    }

    pub fn shrink_to(&mut self, min_capacity: usize) {
        self.__mut().shrink_to(min_capacity);
    }

    pub fn push(&mut self, value: T) {
        Self::mark_append(self, self.len());
        self.__mut().push(value);
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__mut().append(other);
    }
}

impl<'i, T: Observe + Clone> VecOb<'i, T> {
    pub fn extend_from_slice(&mut self, other: &[T]) {
        if other.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__mut().extend_from_slice(other);
    }

    pub fn extend_from_within<R: RangeBounds<usize>>(&mut self, range: R) {
        Self::mark_append(self, self.len());
        self.__mut().extend_from_within(range);
    }
}

impl<'i, T: Observe> Extend<T> for VecOb<'i, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.__mut().extend(other);
    }
}

impl<'i, 'a, T: Observe + Copy + 'a> Extend<&'a T> for VecOb<'i, T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, other: I) {
        Self::mark_append(self, self.len());
        self.__mut().extend(other);
    }
}

// TODO: handle range
impl<'i, T: Observe> Index<usize> for VecOb<'i, T> {
    type Output = T::Target<'i>;
    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        obs.entry(index).or_insert_with(|| value.observe())
    }
}

impl<'i, T: Observe> IndexMut<usize> for VecOb<'i, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.obs.get() };
        obs.entry(index).or_insert_with(|| value.observe())
    }
}
