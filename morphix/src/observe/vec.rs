use std::cell::UnsafeCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use crate::{Ob, Observe};

pub struct VecObInner<'i, T: Observe + 'i> {
    obs: UnsafeCell<HashMap<usize, T::Target<'i>>>,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T: Observe> Default for VecObInner<'i, T> {
    fn default() -> Self {
        Self {
            obs: Default::default(),
            phantom: PhantomData,
        }
    }
}

pub type VecOb<'i, T> = Ob<'i, Vec<T>, VecObInner<'i, T>>;

impl<T: Observe> Observe for Vec<T> {
    type Target<'i>
        = VecOb<'i, T>
    where
        Self: 'i;
}

impl<'i, T: Observe> VecOb<'i, T> {
    pub fn push(&mut self, value: T) {
        if let Some(ctx) = &self.ctx {
            println!("append {:?} (VecOb::push)", ctx.path);
        }
        Self::get_mut(self).push(value);
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        let other = other.into_iter().collect::<Vec<_>>();
        if other.is_empty() {
            return;
        }
        if let Some(ctx) = &self.ctx {
            println!("append {:?} (VecOb::extend)", ctx.path);
        }
        Self::get_mut(self).extend(other);
    }
}

impl<'i, T: Observe> Index<usize> for VecOb<'i, T> {
    type Output = T::Target<'i>;
    fn index(&self, index: usize) -> &Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.inner.obs.get() };
        obs.entry(index)
            .or_insert_with(|| value.observe(self.ctx.as_ref().map(|ctx| ctx.extend(index.to_string().into()))))
    }
}

impl<'i, T: Observe> IndexMut<usize> for VecOb<'i, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = unsafe { &mut (&mut *self.ptr)[index] };
        let obs = unsafe { &mut *self.inner.obs.get() };
        obs.entry(index)
            .or_insert_with(|| value.observe(self.ctx.as_ref().map(|ctx| ctx.extend(index.to_string().into()))))
    }
}
