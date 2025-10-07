use std::marker::PhantomData;

use serde::{Serialize, Serializer};

use crate::Change;
use crate::adapter::Adapter;
use crate::adapter::observe::ObserveAdapter;
use crate::change::Operation;

mod ops;
mod string;
mod vec;

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

pub trait Observer<'i, T: ?Sized> {
    fn observe(value: &'i mut T) -> Self;

    fn into_inner(self) -> T;

    fn get_ref(&self) -> &T;

    fn get_mut(&mut self) -> &mut T;

    fn op(&mut self) -> &mut Option<Operation<ObserveAdapter>>;

    fn take_op(&mut self) -> Option<Operation<ObserveAdapter>> {
        self.op().take()
    }

    fn mark_replace(&mut self) {
        *self.op() = Some(Operation::Replace(()));
    }

    fn mark_append(&mut self, start_index: usize) {
        if let Some(Operation::Replace(())) = self.op() {
            return;
        }
        *self.op() = Some(Operation::Append(start_index));
    }

    fn collect<A: Adapter>(this: &mut Self) -> Result<Option<Change<A>>, A::Error>
    where
        T: Observe<Target<'i> = Self> + 'i;
}

pub trait ObInner: Default {
    #[expect(unused)]
    fn dump<A: Adapter>(&mut self, changes: &mut Vec<Change<A>>) -> Result<(), A::Error> {
        Ok(())
    }
}

impl ObInner for () {}

/// An observable value.
pub struct Ob<'i, T, U: ObInner = ()> {
    ptr: *mut T,
    operation: Option<Operation<ObserveAdapter>>,
    inner: U,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T, U: ObInner> Observer<'i, T> for Ob<'i, T, U> {
    #[inline]
    fn observe(value: &'i mut T) -> Self {
        Ob::new(value)
    }

    #[inline]
    fn op(&mut self) -> &mut Option<Operation<ObserveAdapter>> {
        &mut self.operation
    }

    #[inline]
    fn into_inner(self) -> T {
        unsafe { std::ptr::read(self.ptr) }
    }

    #[inline]
    fn get_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }

    #[inline]
    fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }

    fn collect<A: Adapter>(this: &mut Self) -> Result<Option<Change<A>>, A::Error>
    where
        T: Observe,
    {
        let mut changes = vec![];
        if let Some(operation) = this.take_op() {
            changes.push(Change {
                path_rev: vec![],
                operation: match operation {
                    Operation::Replace(()) => Operation::Replace(A::new_replace(this.get_ref())?),
                    Operation::Append(start_index) => Operation::Append(A::new_append(this.get_ref(), start_index)?),
                    _ => unreachable!(),
                },
            })
        };
        this.inner.dump(&mut changes)?;
        Ok(match changes.len() {
            0 => None,
            1 => Some(changes.swap_remove(0)),
            _ => Some(Change {
                path_rev: vec![],
                operation: Operation::Batch(changes),
            }),
        })
    }
}

impl<'i, T, U: ObInner> Ob<'i, T, U> {
    pub fn new(value: &'i mut T) -> Self {
        Self {
            ptr: value as *mut T,
            operation: None,
            inner: U::default(),
            phantom: PhantomData,
        }
    }
}

macro_rules! impl_observe {
    ($($ty:ty $(=> $target:ty)?),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Target<'i> = Ob<'i, $ty>
                where
                    Self: 'i;
            }
        )*
    };
}

impl_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool,
}
