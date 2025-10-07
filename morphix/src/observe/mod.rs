use std::ops::DerefMut;

use serde::{Serialize, Serializer};

use crate::Change;
use crate::adapter::Adapter;

mod raw;
mod string;
mod vec;

pub use raw::RawOb;

/// Trait for observing changes.
pub trait Observe: Serialize {
    type Target<'i>: Observer<'i, Self>
    where
        Self: 'i;

    #[inline]
    fn observe<'i>(&'i mut self) -> Self::Target<'i> {
        Self::Target::observe(self)
    }

    #[inline]
    #[expect(unused_variables)]
    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        unimplemented!()
    }
}

pub trait Observer<'i, T: ?Sized>: DerefMut<Target = T> {
    fn observe(value: &'i mut T) -> Self;

    fn collect<A: Adapter>(this: &mut Self) -> Result<Option<Change<A>>, A::Error>
    where
        T: Observe<Target<'i> = Self> + 'i;
}

#[derive(Clone, Copy)]
pub enum Mutation {
    Replace,
    Append(usize),
}

pub trait MutationObserver<'i, T>: Observer<'i, T> {
    fn mutation(this: &mut Self) -> &mut Option<Mutation>;

    fn mark_replace(this: &mut Self) {
        *Self::mutation(this) = Some(Mutation::Replace);
    }

    fn mark_append(this: &mut Self, start_index: usize) {
        let mutation = Self::mutation(this);
        if mutation.is_some() {
            return;
        }
        *mutation = Some(Mutation::Append(start_index));
    }
}
