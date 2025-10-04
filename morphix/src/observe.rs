use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use serde::Serialize;

use crate::adapter::{Adapter, MutationAdapter};
use crate::batch::Batch;
use crate::change::{Change, Operation};

/// Trait for observing changes.
pub trait Observe {
    type Target<'i>
    where
        Self: 'i;

    fn observe<'i>(&'i mut self, ctx: &Context) -> Self::Target<'i>;
}

/// Context for observing changes.
#[derive(Debug, Default)]
pub struct Context {
    prefix: String,
    mutations: Arc<Mutex<Batch<MutationAdapter>>>,
}

impl Context {
    /// Create a root context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a sub-context at a sub-path.
    pub fn extend(&self, path: &str) -> Self {
        Self {
            prefix: self.prefix.clone() + "/" + path,
            mutations: self.mutations.clone(),
        }
    }

    /// Collect changes and errors.
    pub fn collect<A: Adapter>(self) -> Result<Option<Change<A>>, A::Error> {
        todo!()
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
        let mut batch = self.ctx.mutations.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: vec![], // &self.ctx.prefix
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
        let mut batch = self.ctx.mutations.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: vec![], // &self.ctx.prefix
            operation: Operation::Append(self.chars().count()),
        });
        self.value.push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let mut batch = self.ctx.mutations.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: vec![], // &self.ctx.prefix
            operation: Operation::Append(self.chars().count()),
        });
        self.value.push_str(s);
    }
}

impl<'i, T: Serialize> Ob<'i, Vec<T>> {
    pub fn push(&mut self, value: T) {
        let mut batch = self.ctx.mutations.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: vec![], // &self.ctx.prefix
            operation: Operation::Append(self.len()),
        });
        self.value.push(value);
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        let other = other.into_iter().collect::<Vec<_>>();
        if other.is_empty() {
            return;
        }
        let mut batch = self.ctx.mutations.lock().unwrap();
        let _ = batch.load(Change {
            path_rev: vec![], // &self.ctx.prefix
            operation: Operation::Append(self.len()),
        });
        self.value.extend(other);
    }
}
