use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{AddAssign, Deref, DerefMut};

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe};

/// Observer implementation for [`String`].
///
/// `StringObserver` provides special handling for string append operations, distinguishing them
/// from complete replacements for efficiency.
pub struct StringObserver<'ob, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    mutation: Option<usize>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D> {
    #[inline]
    fn __mark_replace(&mut self) {
        self.mutation = None;
    }
}

impl<'ob, S: ?Sized, D> Deref for StringObserver<'ob, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, S: ?Sized, D> DerefMut for StringObserver<'ob, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.__mark_replace();
        &mut self.ptr
    }
}

impl<'ob, S> Assignable for StringObserver<'ob, S> {
    type Depth = Succ<Zero>;
}

impl<'ob, S: ?Sized, D> Observer<'ob> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'ob,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            mutation: None,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            mutation: Some(value.as_deref().len()),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        ObserverPointer::set(Self::as_ptr(this), value);
    }
}

impl<'ob, S: ?Sized, D> SerializeObserver<'ob> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'ob,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        let len = this.as_deref().len();
        Ok(if let Some(initial_len) = this.mutation.replace(len) {
            if len > initial_len {
                Some(Mutation {
                    path: Default::default(),
                    kind: MutationKind::Append(A::serialize_value(&this.as_deref()[initial_len..])?),
                })
            } else {
                None
            }
        } else {
            Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref())?),
            })
        })
    }
}

impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    /// See [`String::push`].
    #[inline]
    pub fn push(&mut self, c: char) {
        Observer::as_inner(self).push(c);
    }

    /// See [`String::push_str`].
    #[inline]
    pub fn push_str(&mut self, s: &str) {
        Observer::as_inner(self).push_str(s);
    }
}

impl<'ob, S: ?Sized, D> AddAssign<&str> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}

impl<'ob, S: ?Sized, D, U> Extend<U> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
    String: Extend<U>,
{
    #[inline]
    fn extend<I: IntoIterator<Item = U>>(&mut self, other: I) {
        Observer::as_inner(self).extend(other);
    }
}

impl<'ob, S: ?Sized, D> Debug for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StringObserver").field(self.as_deref()).finish()
    }
}

impl<'ob, S: ?Sized, D> Display for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.as_deref(), f)
    }
}

impl<'ob, S, D, U: ?Sized> PartialEq<U> for StringObserver<'ob, S, D>
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

impl<'ob, S, D, U: ?Sized> PartialOrd<U> for StringObserver<'ob, S, D>
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
    type Observer<'ob, S, D>
        = StringObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_mutation_returns_none() {
        let mut s = String::from("hello");
        let mut ob = s.__observe();
        let Json(mutation) = ob.collect().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn replace_on_deref_mut() {
        let mut s = String::from("hello");
        let mut ob = s.__observe();
        ob.clear();
        ob.push_str("world"); // append after replace should have no effect
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Replace(json!("world"))
            })
        );
    }

    #[test]
    fn append_with_push() {
        let mut s = String::from("a");
        let mut ob = s.__observe();
        ob.push('b');
        ob.push('c');
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("bc"))
            })
        );
    }

    #[test]
    fn append_with_push_str() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob.push_str("bar");
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(
            mutation,
            Some(Mutation {
                path: vec![].into(),
                kind: MutationKind::Append(json!("bar"))
            })
        );
    }

    #[test]
    fn append_with_add_assign() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob += "bar";
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn append_empty_string() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob.push_str("");
        ob += "";
        let Json(mutation) = ob.collect().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn replace_after_append() {
        let mut s = String::from("abc");
        let mut ob = s.__observe();
        ob.push_str("def");
        **ob = String::from("xyz");
        let Json(mutation) = ob.collect().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("xyz")));
    }
}
