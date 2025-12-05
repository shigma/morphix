use std::collections::TryReserveError;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{AddAssign, Bound, Deref, DerefMut, RangeBounds};
use std::string::Drain;

use crate::helper::macros::untracked_methods;
use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe};

/// Observer implementation for [`String`].
///
/// `StringObserver` provides special handling for string append operations, distinguishing them
/// from complete replacements for efficiency.
pub struct StringObserver<'ob, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    mutation: Option<TruncateAppend>,
    phantom: PhantomData<&'ob mut D>,
}

struct TruncateAppend {
    pub start_index: usize,  // byte index
    pub truncate_len: usize, // char count
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
            mutation: Some(TruncateAppend {
                start_index: value.as_deref().len(),
                truncate_len: 0,
            }),
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
        let Some(TruncateAppend {
            start_index,
            truncate_len,
        }) = this.mutation.replace(TruncateAppend {
            start_index: len,
            truncate_len: 0,
        })
        else {
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref())?),
            }));
        };
        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Truncate(truncate_len),
            }));
        }
        #[cfg(feature = "append")]
        if len > start_index {
            return Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Append(A::serialize_value(&this.as_deref()[start_index..])?),
            }));
        }
        Ok(None)
    }
}

impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'ob,
{
    #[inline]
    fn __start_index(&mut self) -> usize {
        match &mut self.mutation {
            Some(m) => m.start_index,
            None => 0,
        }
    }
}

impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    untracked_methods! { String =>
        pub fn reserve(&mut self, additional: usize);
        pub fn reserve_exact(&mut self, additional: usize);
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn try_reserve_exact(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn shrink_to_fit(&mut self);
        pub fn shrink_to(&mut self, min_capacity: usize);
    }
}

#[cfg(feature = "append")]
impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'ob,
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

    /// See [`String::extend_from_within`].
    #[inline]
    pub fn extend_from_within<R>(&mut self, src: R)
    where
        R: RangeBounds<usize>,
    {
        Observer::as_inner(self).extend_from_within(src);
    }

    /// See [`String::insert`].
    #[inline]
    pub fn insert(&mut self, idx: usize, ch: char) {
        if idx >= self.__start_index() {
            Observer::as_inner(self).insert(idx, ch)
        } else {
            Observer::track_inner(self).insert(idx, ch)
        }
    }

    /// See [`String::insert_str`].
    #[inline]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        if idx >= self.__start_index() {
            Observer::as_inner(self).insert_str(idx, string)
        } else {
            Observer::track_inner(self).insert_str(idx, string)
        }
    }
}

#[cfg(any(feature = "truncate", feature = "append"))]
impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'ob,
{
    /// See [`String::remove`].
    #[inline]
    pub fn remove(&mut self, idx: usize) -> char {
        if idx >= self.__start_index() {
            Observer::as_inner(self).remove(idx)
        } else {
            Observer::track_inner(self).remove(idx)
        }
    }

    /// See [`String::clear`].
    #[inline]
    pub fn clear(&mut self) {
        if self.__start_index() == 0 {
            Observer::as_inner(self).clear()
        } else {
            Observer::track_inner(self).clear()
        }
    }

    /// See [`String::pop`].
    pub fn pop(&mut self) -> Option<char> {
        let start_index = self.__start_index();
        let len = self.as_deref().len();
        if len > start_index {
            return Observer::as_inner(self).pop();
        }
        if len == 0 {
            return None;
        }
        if len == 1 || cfg!(not(feature = "truncate")) {
            return Observer::track_inner(self).pop();
        }
        let char = Observer::as_inner(self).pop().unwrap();
        let mutation = self.mutation.as_mut().unwrap();
        mutation.truncate_len += 1;
        mutation.start_index -= char.len_utf8();
        Some(char)
    }

    /// See [`String::truncate`].
    pub fn truncate(&mut self, len: usize) {
        let start_index = self.__start_index();
        if len >= start_index {
            return Observer::as_inner(self).truncate(len);
        }
        if len == 0 || cfg!(not(feature = "truncate")) {
            return Observer::track_inner(self).truncate(len);
        }
        let count = self.as_deref()[len..start_index].chars().count();
        let mutation = self.mutation.as_mut().unwrap();
        mutation.truncate_len += count;
        mutation.start_index = len;
        Observer::as_inner(self).truncate(len)
    }

    /// See [`String::split_off`].
    pub fn split_off(&mut self, at: usize) -> String {
        let start_index = self.__start_index();
        if at >= start_index {
            return Observer::as_inner(self).split_off(at);
        }
        if at == 0 || cfg!(not(feature = "truncate")) {
            return Observer::track_inner(self).split_off(at);
        }
        let count = self.as_deref()[at..start_index].chars().count();
        let mutation = self.mutation.as_mut().unwrap();
        mutation.truncate_len += count;
        mutation.start_index = at;
        Observer::as_inner(self).split_off(at)
    }

    /// See [`String::drain`].
    pub fn drain<R>(&mut self, range: R) -> Drain<'_>
    where
        R: RangeBounds<usize>,
    {
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= self.__start_index() {
            Observer::as_inner(self).drain(range)
        } else {
            Observer::track_inner(self).drain(range)
        }
    }

    /// See [`String::replace_range`].
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        let start_index = self.__start_index();
        let start_bound = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_bound >= start_index {
            return Observer::as_inner(self).replace_range(range, replace_with);
        }
        if start_bound == 0 || cfg!(not(feature = "truncate")) {
            return Observer::track_inner(self).replace_range(range, replace_with);
        }
        let end_bound = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.as_deref().len(),
        };
        if end_bound < start_index {
            return Observer::track_inner(self).replace_range(range, replace_with);
        }
        let count = self.as_deref()[start_bound..start_index].chars().count();
        let mutation = self.mutation.as_mut().unwrap();
        mutation.truncate_len += count;
        mutation.start_index = start_bound;
        Observer::as_inner(self).replace_range(range, replace_with);
    }
}

impl<'ob, S: ?Sized, D> AddAssign<&str> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String> + 'ob,
{
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        #[cfg(feature = "append")]
        self.push_str(rhs);
        #[cfg(not(feature = "append"))]
        Observer::track_inner(self).add_assign(rhs);
    }
}

#[cfg(feature = "append")]
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
