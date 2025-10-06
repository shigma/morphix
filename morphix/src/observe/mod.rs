use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::DerefMut;
use std::sync::{Arc, Mutex};

use serde::Serializer;

use crate::adapter::Adapter;
use crate::adapter::observe::ObserveAdapter;
use crate::batch::Batch;
use crate::change::{Change, Operation};

mod ops;
mod string;
mod vec;

/// Trait for observing changes.
pub trait Observe {
    type Target<'i>: Observer<'i, Self>
    where
        Self: 'i;

    fn observe<'i>(&'i mut self, ctx: Option<Context>) -> Self::Target<'i> {
        Self::Target::observe(self, ctx)
    }

    fn serialize_at<S: Serializer>(&self, _serializer: S, _change: &Change<ObserveAdapter>) -> Result<S::Ok, S::Error> {
        todo!()
    }
}

pub trait Observer<'i, T: ?Sized>: DerefMut<Target = T> {
    fn observe(value: &'i mut T, ctx: Option<Context>) -> Self;
}

/// Context for observing changes.
#[derive(Debug, Default)]
pub struct Context {
    path: Vec<Cow<'static, str>>,
    batch: Arc<Mutex<Batch<ObserveAdapter>>>,
}

impl Context {
    /// Create a root context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a sub-context at a sub-path.
    pub fn extend(&self, part: Cow<'static, str>) -> Self {
        let mut path = self.path.clone();
        path.push(part);
        Self {
            path,
            batch: self.batch.clone(),
        }
    }

    /// Collect changes and errors.
    pub fn collect<A: Adapter>(self, value: &impl Observe) -> Result<Option<Change<A>>, A::Error> {
        Ok(match self.batch.lock().unwrap().dump() {
            Some(v) => Some(A::try_from_observe(value, v)?),
            None => None,
        })
    }
}

/// An observable value.
pub struct Ob<'i, T, U: Default = ()> {
    ptr: *mut T,
    ctx: Option<Context>,
    inner: U,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T, U: Default> Observer<'i, T> for Ob<'i, T, U> {
    fn observe(value: &'i mut T, ctx: Option<Context>) -> Self {
        Ob::new(value, ctx)
    }
}

impl<'i, T, U: Default> Ob<'i, T, U> {
    pub fn new(value: &'i mut T, ctx: Option<Context>) -> Self {
        Self {
            ptr: value as *mut T,
            ctx,
            inner: Default::default(),
            phantom: PhantomData,
        }
    }

    pub fn get(this: &Self) -> &T {
        unsafe { &*this.ptr }
    }

    pub fn get_mut(this: &mut Self) -> &mut T {
        unsafe { &mut *this.ptr }
    }

    pub fn record(this: &mut Self, operation: Operation<ObserveAdapter>) {
        let Some(ctx) = &this.ctx else {
            return;
        };
        let mut batch = ctx.batch.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: ctx.path.iter().cloned().rev().collect(),
            operation,
        });
    }
}

macro_rules! impl_observe {
    ($($ty:ty $(=> $target:ty)?),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Target<'i> = Ob<'i, $ty>
                where
                    Self: 'i;

                fn observe<'i>(&'i mut self, ctx: Option<Context>) -> Self::Target<'i> {
                    Ob::new(self, ctx)
                }
            }
        )*
    };
}

impl_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64,
}
