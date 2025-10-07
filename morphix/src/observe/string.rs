use std::ops::AddAssign;

use crate::{Ob, Observe, Observer, Operation};

#[derive(Default)]
pub struct StringObserverInner;

pub type StringObserver<'i> = Ob<'i, String, StringObserverInner>;

impl Observe for String {
    type Target<'i>
        = StringObserver<'i>
    where
        Self: 'i;
}

impl<'i> StringObserver<'i> {
    pub fn push(&mut self, c: char) {
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        Self::record(self, Operation::Append(self.len()));
        Self::get_mut(self).push_str(s);
    }
}

impl<'i> AddAssign<&str> for StringObserver<'i> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
