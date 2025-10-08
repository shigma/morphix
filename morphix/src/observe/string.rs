use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use crate::observe::{MutationState, StatefulObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, Observer};

/// An observer for [`String`] that tracks both replacements and appends.
///
/// `StringObserver` provides special handling for string append operations, distinguishing them
/// from complete replacements for efficiency.
///
/// ## Supported Operations
///
/// The following mutations are tracked as `Append`:
///
/// - [String::add_assign](std::ops::AddAssign) (`+=`)
/// - [String::push](std::string::String::push)
/// - [String::push_str](std::string::String::push_str)
pub struct StringObserver<'i> {
    ptr: *mut String,
    mutation: Option<MutationState>,
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

impl<'i> Observer<'i> for StringObserver<'i> {
    #[inline]
    fn observe(value: &'i mut String) -> Self {
        Self {
            ptr: value as *mut String,
            mutation: None,
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
        Ok(if let Some(mutation) = Self::mutation_state(&mut this).take() {
            Some(Mutation {
                path_rev: vec![],
                operation: match mutation {
                    MutationState::Replace => MutationKind::Replace(A::serialize_value(&*this)?),
                    MutationState::Append(start_index) => {
                        MutationKind::Append(A::serialize_value(&this[start_index..])?)
                    }
                },
            })
        } else {
            None
        })
    }
}

impl<'i> StatefulObserver<'i> for StringObserver<'i> {
    fn mutation_state(this: &mut Self) -> &mut Option<MutationState> {
        &mut this.mutation
    }
}

impl Observe for String {
    type Observer<'i>
        = StringObserver<'i>
    where
        Self: 'i;
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
