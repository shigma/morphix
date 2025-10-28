use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe};

enum MutationState {
    Replace,
    Append(usize),
}

/// Observer implementation for [`String`].
///
/// `StringObserver` provides special handling for string append operations, distinguishing them
/// from complete replacements for efficiency.
pub struct StringObserver<'i, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    mutation: Option<MutationState>,
    phantom: PhantomData<&'i mut D>,
}

impl<'i, S: ?Sized, D> StringObserver<'i, S, D> {
    #[inline]
    fn __mark_replace(&mut self) {
        self.mutation = Some(MutationState::Replace);
    }
}

impl<'i, S: ?Sized, D> Default for StringObserver<'i, S, D> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, S: ?Sized, D> Deref for StringObserver<'i, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, S: ?Sized, D> DerefMut for StringObserver<'i, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.__mark_replace();
        &mut self.ptr
    }
}

impl<'i, S> Assignable for StringObserver<'i, S> {
    type Depth = Succ<Zero>;
}

impl<'i, S: ?Sized, D> Observer<'i> for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'i,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            mutation: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, S: ?Sized, D> SerializeObserver<'i> for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'i,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        Ok(if let Some(mutation) = this.mutation.take() {
            Some(Mutation {
                path: Default::default(),
                kind: match mutation {
                    MutationState::Replace => MutationKind::Replace(A::serialize_value(this.as_deref())?),
                    MutationState::Append(start_index) => {
                        MutationKind::Append(A::serialize_value(&this.as_deref()[start_index..])?)
                    }
                },
            })
        } else {
            None
        })
    }
}

impl<'i, S: ?Sized, D> StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn __mark_append(&mut self) {
        if self.mutation.is_some() {
            return;
        }
        self.mutation = Some(MutationState::Append(self.as_deref().len()));
    }

    /// See [`String::push`].
    pub fn push(&mut self, c: char) {
        self.__mark_append();
        Observer::as_inner(self).push(c);
    }

    /// See [`String::push_str`].
    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.__mark_append();
        Observer::as_inner(self).push_str(s);
    }
}

impl<'i, S: ?Sized, D> AddAssign<&str> for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<'i, S: ?Sized, D, U> Extend<U> for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
    String: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        self.__mark_append();
        Observer::as_inner(self).extend(other);
    }
}

impl<'i, S: ?Sized, D> Debug for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StringObserver").field(self.as_deref()).finish()
    }
}

impl<'i, S: ?Sized, D> Display for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_deref(), f)
    }
}

impl<'i, S, D, U: ?Sized> PartialEq<U> for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
    String: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'i, S, D, U: ?Sized> PartialOrd<U> for StringObserver<'i, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
    String: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl Observe for String {
    type Observer<'i, S, D>
        = StringObserver<'i, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::JsonAdapter;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_mutation_returns_none() {
        let mut s = String::from("hello");
        let mut ob = s.observe();
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());
    }

    #[test]
    fn replace_on_deref_mut() {
        let mut s = String::from("hello");
        let mut ob = s.observe();
        ob.clear();
        ob.push_str("world"); // append after replace should have no effect
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!("world")));
    }

    #[test]
    fn append_with_push() {
        let mut s = String::from("a");
        let mut ob = s.observe();
        ob.push('b');
        ob.push('c');
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!("bc")));
    }

    #[test]
    fn append_with_push_str() {
        let mut s = String::from("foo");
        let mut ob = s.observe();
        ob.push_str("bar");
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn append_with_add_assign() {
        let mut s = String::from("foo");
        let mut ob = s.observe();
        ob += "bar";
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn append_empty_string() {
        let mut s = String::from("foo");
        let mut ob = s.observe();
        ob.push_str("");
        ob += "";
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());
    }

    #[test]
    fn replace_after_append() {
        let mut s = String::from("abc");
        let mut ob = s.observe();
        ob.push_str("def");
        **ob = String::from("xyz");
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!("xyz")));
    }
}
