use std::ops::AddAssign;

use serde::{Serialize, Serializer};

use crate::observe::ObInner;
use crate::{Ob, Observe, Observer};

#[derive(Default)]
pub struct StringObserverInner;

impl ObInner for StringObserverInner {}

pub type StringObserver<'i> = Ob<'i, String, StringObserverInner>;

impl Observe for String {
    type Target<'i>
        = StringObserver<'i>
    where
        Self: 'i;

    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        self[start_index..].serialize(serializer)
    }
}

impl<'i> StringObserver<'i> {
    pub fn push(&mut self, c: char) {
        self.mark_append(self.len());
        self.get_mut().push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.mark_append(self.len());
        self.get_mut().push_str(s);
    }
}

impl<'i> AddAssign<&str> for StringObserver<'i> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
