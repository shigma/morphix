#![doc = include_str!("../README.md")]

use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use serde::Serialize;

mod batch;
mod change;
mod delta;
mod error;

pub use crate::change::Change;
pub use crate::delta::{Delta, DeltaKind, DeltaState};
pub use crate::error::Error;

#[cfg(feature = "derive")]
extern crate umili_derive;

#[cfg(feature = "derive")]
pub use umili_derive::{observe, Observe};

/// Trait for observing changes.
pub trait Observe {
    type Target<'i> where Self: 'i;

    fn observe<'i>(&'i mut self, ctx: &Context) -> Self::Target<'i>;
}

/// Context for observing changes.
#[derive(Debug, Default)]
pub struct Context {
    prefix: String,
    mutation: Rc<RefCell<Mutation>>,
}

impl Context {
    /// Create a root context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a sub-context for a sub-path.
    pub fn extend(&self, path: &str) -> Self {
        Self {
            prefix: self.prefix.clone() + "/" + path,
            mutation: self.mutation.clone(),
        }
    }

    /// Collect changes and errors.
    pub fn collect(self) -> Result<Vec<Change>, Vec<serde_json::Error>> {
        self.mutation.take().collect()
    }
}

#[derive(Debug, Default)]
struct Mutation {
    changes: Vec<Change>,
    errors: Vec<serde_json::Error>,
}

impl Mutation {
    fn push(&mut self, result: Result<Change, serde_json::Error>) {
        match result {
            Ok(change) => self.changes.push(change),
            Err(error) => self.errors.push(error),
        }
    }

    fn collect(self) -> Result<Vec<Change>, Vec<serde_json::Error>> {
        match self.errors.len() {
            0 => Ok(self.changes),
            _ => Err(self.errors),
        }
    }
}

/// An observable value.
pub struct Ob<'i, T: Clone + Serialize + PartialEq> {
    pub value: &'i mut T,
    pub ctx: Context,
}

impl<'i, T: Clone + Serialize + PartialEq> Ob<'i, T> {
    pub fn borrow_mut<'j>(&'j mut self) -> Ref<'j, T> {
        Ref {
            old_value: None,
            value: self.value,
            path: &self.ctx.prefix[1..],
            mutation: self.ctx.mutation.clone(),
        }
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Deref for Ob<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq + Observe> Ob<'i, T> {
    #[inline]
    pub fn borrow(&mut self) -> T::Target<'_> {
        self.value.observe(&self.ctx)
    }
}

/// Reference to an observable value.
pub struct Ref<'i, T: Clone + Serialize + PartialEq> {
    old_value: Option<T>,
    value: &'i mut T,
    path: &'i str,
    mutation: Rc<RefCell<Mutation>>,
}

#[cfg(feature = "append")]
impl<'i> Ref<'i, String> {
    pub fn add_assign(&mut self, s: &str) {
        self.push_str(s);
    }

    pub fn push(&mut self, c: char) {
        self.mutation.borrow_mut().push(Change::append(self.path, c));
        self.value.push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.mutation.borrow_mut().push(Change::append(self.path, s));
        self.value.push_str(s);
    }
}

#[cfg(feature = "append")]
impl<'i, T: Clone + Serialize + PartialEq> Ref<'i, Vec<T>> {
    pub fn push(&mut self, value: T) {
        self.mutation.borrow_mut().push(Change::append(self.path, vec![&value]));
        self.value.push(value);
    }

    pub fn extend<I: IntoIterator<Item = T>>(&mut self, other: I) {
        let other = other.into_iter().collect::<Vec<_>>();
        if other.is_empty() {
            return;
        }
        self.mutation.borrow_mut().push(Change::append(self.path, &other));
        self.value.extend(other);
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Deref for Ref<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq> DerefMut for Ref<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.old_value.is_none() {
            self.old_value.replace(self.value.clone());
        }
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Drop for Ref<'i, T> {
    fn drop(&mut self) {
        if let Some(old_value) = self.old_value.take()
            && old_value != *self.value {
                self.mutation.borrow_mut().push(Change::set(self.path, &self.value));
            }
    }
}
