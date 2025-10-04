use std::ops::{AddAssign, Deref, DerefMut};
use std::sync::{Arc, Mutex};

use serde::Serializer;

use crate::adapter::Adapter;
use crate::adapter::observe::ObserveAdapter;
use crate::batch::Batch;
use crate::change::{Change, Operation};

/// Trait for observing changes.
pub trait Observe {
    type Target<'i>
    where
        Self: 'i;

    fn observe<'i>(&'i mut self, ctx: &Context) -> Self::Target<'i>;

    fn serialize_at<S: Serializer>(&self, _change: Change<ObserveAdapter>) -> Result<S::Ok, S::Error> {
        todo!()
    }
}

/// Context for observing changes.
#[derive(Debug, Default)]
pub struct Context {
    path: Vec<String>,
    batch: Arc<Mutex<Batch<ObserveAdapter>>>,
}

impl Context {
    /// Create a root context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a sub-context at a sub-path.
    pub fn extend(&self, part: &str) -> Self {
        let mut path = self.path.clone();
        path.push(part.to_string());
        Self {
            path,
            batch: self.batch.clone(),
        }
    }

    /// Collect changes and errors.
    pub fn collect<A: Adapter>(self, value: &impl Observe) -> Result<Option<Change<A>>, A::Error> {
        if let Some(v) = self.batch.lock().unwrap().dump() {
            Ok(Some(A::from_observe(value, v)?))
        } else {
            Ok(None)
        }
    }
}

/// An observable value.
pub struct Ob<'i, T> {
    pub value: &'i mut T,
    pub ctx: Context,
}

impl<'i, T> Deref for Ob<'i, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'i, T> DerefMut for Ob<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let mut batch = self.ctx.batch.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: self.ctx.path.iter().rev().cloned().collect(),
            operation: Operation::Replace(()),
        });
        self.value
    }
}

impl<'i> Ob<'i, String> {
    pub fn add_assign(&mut self, s: &str) {
        self.push_str(s);
    }

    pub fn push(&mut self, c: char) {
        let mut batch = self.ctx.batch.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: self.ctx.path.iter().rev().cloned().collect(),
            operation: Operation::Append(self.len()),
        });
        self.value.push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let mut batch = self.ctx.batch.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: self.ctx.path.iter().rev().cloned().collect(),
            operation: Operation::Append(self.len()),
        });
        self.value.push_str(s);
    }
}

impl<'i> AddAssign<&str> for Ob<'i, String> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<'i, T> Ob<'i, Vec<T>> {
    pub fn push(&mut self, value: T) {
        let mut batch = self.ctx.batch.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: self.ctx.path.iter().rev().cloned().collect(),
            operation: Operation::Append(self.len()),
        });
        self.value.push(value);
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        let other = other.into_iter().collect::<Vec<_>>();
        if other.is_empty() {
            return;
        }
        let mut batch = self.ctx.batch.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: self.ctx.path.iter().rev().cloned().collect(),
            operation: Operation::Append(self.len()),
        });
        self.value.extend(other);
    }
}
