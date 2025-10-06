use std::ops::AddAssign;

use crate::{Ob, Observe, Operation};

#[derive(Default)]
pub struct StringObInner;

pub type StringOb<'i> = Ob<'i, String, StringObInner>;

impl Observe for String {
    type Target<'i>
        = StringOb<'i>
    where
        Self: 'i;
}

impl<'i> StringOb<'i> {
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

impl<'i> AddAssign<&str> for StringOb<'i> {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
