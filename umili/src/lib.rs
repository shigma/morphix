use std::mem::replace;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::cell::RefCell;

use serde::Serialize;
use serde_json::Error;

pub mod change;
pub mod delta;

pub use crate::change::Change;
pub use crate::delta::{Delta, DeltaHistory, DeltaKind};

#[cfg(feature = "derive")]
extern crate umili_derive;

#[cfg(feature = "derive")]
pub use umili_derive::{observe, Observe};

pub trait Observe {
    type Target<'i> where Self: 'i;

    fn observe<'i>(&'i mut self, prefix: &str, mutation: &Rc<RefCell<Mutation>>) -> Self::Target<'i>;
}

#[derive(Debug, Default)]
pub struct Mutation {
    changes: Vec<Change>,
    errors: Vec<Error>,
}

impl Mutation {
    pub fn new() -> Rc<RefCell<Self>> {
        Default::default()
    }

    pub fn push(&mut self, result: Result<Change, Error>) {
        match result {
            Ok(change) => self.changes.push(change),
            Err(error) => self.errors.push(error),
        }
    }

    pub fn collect(&mut self) -> Result<Vec<Change>, Vec<Error>> {
        match self.errors.len() {
            0 => Ok(replace(&mut self.changes, vec![])),
            _ => Err(replace(&mut self.errors, vec![])),
        }
    }
}

pub struct Ob<'i, T: Clone + Serialize + PartialEq> {
    pub value: &'i mut T,
    pub path: String,
    pub mutation: Rc<RefCell<Mutation>>,
}

impl<'i, T: Clone + Serialize + PartialEq> Deref for Ob<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Ob<'i, T> {
    pub fn borrow_mut(&mut self) -> Ref<T> {
        Ref {
            old_value: None,
            value: self.value,
            path: &self.path,
            mutation: self.mutation.clone(),
        }
    }
}

impl<'i, T: Clone + Serialize + PartialEq + Observe> Ob<'i, T> {
    #[inline]
    pub fn borrow(&mut self) -> T::Target<'_> {
        let prefix = self.path.to_string() + "/";
        self.value.observe(&prefix, &self.mutation)
    }
}

pub struct Ref<'i, T: Clone + Serialize + PartialEq> {
    pub old_value: Option<T>,
    pub value: &'i mut T,
    pub path: &'i str,
    pub mutation: Rc<RefCell<Mutation>>,
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
        if let Some(old_value) = self.old_value.take() {
            if old_value != *self.value {
                self.mutation.borrow_mut().push(Change::set(self.path, &self.value));
            }
        }
    }
}
