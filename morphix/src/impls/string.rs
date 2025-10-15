use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use crate::helper::Assignable;
use crate::observe::{DefaultSpec, MutationState, StatefulObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, Observer};

/// An observer for [`String`] that tracks both replacements and appends.
///
/// `StringObserver` provides special handling for string append operations, distinguishing them
/// from complete replacements for efficiency.
///
/// ## Supported Operations
///
/// The following mutations are tracked as [`Append`](MutationKind::Append):
///
/// - [String::add_assign](std::ops::AddAssign) (`+=`)
/// - [String::push](std::string::String::push)
/// - [String::push_str](std::string::String::push_str)
#[derive(Default)]
pub struct StringObserver<'i> {
    ptr: *mut String,
    mutation: Option<MutationState>,
    phantom: PhantomData<&'i mut String>,
}

impl<'i> Deref for StringObserver<'i> {
    type Target = String;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i> DerefMut for StringObserver<'i> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        self.__as_mut()
    }
}

impl<'i> Assignable for StringObserver<'i> {}

impl<'i> Observer<'i> for StringObserver<'i> {
    type Spec = DefaultSpec;

    fn inner(this: &Self) -> *mut Self::Target {
        this.ptr
    }

    #[inline]
    fn observe(value: &'i mut String) -> Self {
        Self {
            ptr: value,
            mutation: None,
            phantom: PhantomData,
        }
    }

    unsafe fn collect_unchecked<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
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
    #[inline]
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
    fn __as_mut(&mut self) -> &mut String {
        unsafe { &mut *self.ptr }
    }

    pub fn push(&mut self, c: char) {
        Self::mark_append(self, self.len());
        self.__as_mut().push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__as_mut().push_str(s);
    }
}

impl<'i> AddAssign<&str> for StringObserver<'i> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
