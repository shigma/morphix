//! Observer implementation for [`BinaryHeap<T>`].

use std::collections::binary_heap::{Drain, PeekMut};
use std::collections::{BinaryHeap, TryReserveError};
use std::ops::{Deref, DerefMut};

use crate::Observe;
use crate::helper::macros::{default_impl_ref_observe, delegate_methods, shallow_observer};
use crate::helper::{AsDerefMut, QuasiObserver, Unsigned};
use crate::observe::DefaultSpec;

shallow_observer! {
    impl [T] BinaryHeapObserver for BinaryHeap<T>;
}

default_impl_ref_observe! {
    impl [T] RefObserve for BinaryHeap<T>;
}

struct Guard<'a, T> {
    old_len: usize,
    mutated: &'a mut bool,
    inner: &'a mut BinaryHeap<T>,
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        if self.old_len != self.inner.len() {
            *self.mutated = true;
        }
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = BinaryHeap<T>;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'ob, S: ?Sized, D, T> BinaryHeapObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BinaryHeap<T>>,
{
    fn nonempty_mut(&mut self) -> &mut BinaryHeap<T> {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut()
        } else {
            self.tracked_mut()
        }
    }

    fn guarded_mut(&mut self) -> Guard<'_, T> {
        let inner = (*self.ptr).as_deref_mut();
        Guard {
            old_len: inner.len(),
            mutated: &mut self.mutated,
            inner,
        }
    }

    delegate_methods! { untracked_mut() as BinaryHeap =>
        pub fn reserve_exact(&mut self, additional: usize);
        pub fn reserve(&mut self, additional: usize);
        pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn shrink_to_fit(&mut self);
        pub fn shrink_to(&mut self, min_capacity: usize);
    }

    delegate_methods! { nonempty_mut() as BinaryHeap =>
        pub fn drain(&mut self) -> Drain<'_, T>;
        pub fn clear(&mut self);
    }
}

impl<'ob, S: ?Sized, D, T> BinaryHeapObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BinaryHeap<T>>,
    T: Ord,
{
    delegate_methods! { tracked_mut() as BinaryHeap =>
        // TODO: need observer here
        pub fn peek_mut(&mut self) -> Option<PeekMut<'_, T>>;
        pub fn push(&mut self, item: T);
    }

    delegate_methods! { guarded_mut() as BinaryHeap =>
        pub fn pop(&mut self) -> Option<T>;
        pub fn append(&mut self, other: &mut BinaryHeap<T>);
        pub fn retain<F>(&mut self, f: F) where F: FnMut(&T) -> bool;
    }
}

impl<'ob, S: ?Sized, D, T, U> Extend<U> for BinaryHeapObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = BinaryHeap<T>>,
    BinaryHeap<T>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, iter: I) {
        self.guarded_mut().extend(iter);
    }
}
