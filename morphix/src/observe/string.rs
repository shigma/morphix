use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use serde::{Serialize, Serializer};

use crate::observe::{Mutation, MutationObserver};
use crate::{Adapter, Change, Observe, Observer, Operation};

pub struct StringOb<'i> {
    ptr: *mut String,
    mutation: Option<Mutation>,
    phantom: PhantomData<&'i mut String>,
}

impl<'i> Deref for StringOb<'i> {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i> DerefMut for StringOb<'i> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::mark_replace(self);
        self.__mut()
    }
}

impl<'i> Observer<'i, String> for StringOb<'i> {
    #[inline]
    fn observe(value: &'i mut String) -> Self {
        Self {
            ptr: value as *mut String,
            mutation: None,
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(this: &mut Self) -> Result<Option<Change<A>>, A::Error> {
        Ok(if let Some(mutation) = Self::mutation(this).take() {
            Some(Change {
                path_rev: vec![],
                operation: match mutation {
                    Mutation::Replace => Operation::Replace(A::new_replace(&**this)?),
                    Mutation::Append(start_index) => Operation::Append(A::new_append(&**this, start_index)?),
                },
            })
        } else {
            None
        })
    }
}

impl<'i> MutationObserver<'i, String> for StringOb<'i> {
    fn mutation(this: &mut Self) -> &mut Option<Mutation> {
        &mut this.mutation
    }
}

impl Observe for String {
    type Target<'i>
        = StringOb<'i>
    where
        Self: 'i;

    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        self[start_index..].serialize(serializer)
    }
}

impl<'i> StringOb<'i> {
    #[inline]
    fn __mut(&mut self) -> &mut String {
        unsafe { &mut *self.ptr }
    }

    pub fn push(&mut self, c: char) {
        Self::mark_append(self, self.len());
        self.__mut().push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        Self::mark_append(self, self.len());
        self.__mut().push_str(s);
    }
}

impl<'i> AddAssign<&str> for StringOb<'i> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
