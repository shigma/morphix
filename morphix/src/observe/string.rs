use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use serde::{Serialize, Serializer};

use crate::observe::{Mutation, MutationObserver};
use crate::{Adapter, Change, Observe, Observer, Operation};

/// An observer for [String] that tracks both replacements and appends.
///
/// `StringObserver` provides special handling for string append operations,
/// distinguishing them from complete replacements for efficiency.
///
/// ## Supported Operations
///
/// The following mutations are tracked as `Append`:
///
/// - [String::add_assign] (using `+=`)
/// - [String::push]
/// - [String::push_str]
///
/// [`String`]: std::string::String
/// [`String::add_assign`]: std::ops::AddAssign
/// [`String::push`]: std::string::String::push
/// [`String::push_str`]: std::string::String::push_str
pub struct StringObserver<'i> {
    ptr: *mut String,
    mutation: Option<Mutation>,
    phantom: PhantomData<&'i mut String>,
}

impl<'i> Deref for StringObserver<'i> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i> DerefMut for StringObserver<'i> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        self.as_mut()
    }
}

impl<'i> Observer<'i, String> for StringObserver<'i> {
    #[inline]
    fn observe(value: &'i mut String) -> Self {
        Self {
            ptr: value as *mut String,
            mutation: None,
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(mut this: Self) -> Result<Option<Change<A>>, A::Error> {
        Ok(if let Some(mutation) = Self::mutation(&mut this).take() {
            Some(Change {
                path_rev: vec![],
                operation: match mutation {
                    Mutation::Replace => Operation::Replace(A::new_replace(&*this)?),
                    Mutation::Append(start_index) => Operation::Append(A::new_append(&*this, start_index)?),
                },
            })
        } else {
            None
        })
    }
}

impl<'i> MutationObserver<'i, String> for StringObserver<'i> {
    fn mutation(this: &mut Self) -> &mut Option<Mutation> {
        &mut this.mutation
    }
}

impl Observe for String {
    type Observer<'i>
        = StringObserver<'i>
    where
        Self: 'i;

    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        self[start_index..].serialize(serializer)
    }
}

impl<'i> StringObserver<'i> {
    #[inline]
    fn as_mut(&mut self) -> &mut String {
        unsafe { &mut *self.ptr }
    }

    pub fn push(&mut self, c: char) {
        Self::mark_append(self, self.len());
        self.as_mut().push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.as_mut().push_str(s);
    }
}

impl<'i> AddAssign<&str> for StringObserver<'i> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
