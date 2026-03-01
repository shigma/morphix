use std::borrow::Cow;
use std::collections::TryReserveError;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{AddAssign, Bound, Deref, DerefMut, Range, RangeBounds};
use std::string::Drain;

use crate::helper::macros::{default_impl_ref_observe, untracked_methods};
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverExt, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations, Observe};

/// Observer implementation for [`String`].
pub struct StringObserver<'ob, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    mutation: Option<TruncateAppend>,
    phantom: PhantomData<&'ob mut D>,
}

struct TruncateAppend {
    pub append_index: usize, // byte index
    pub truncate_len: usize, // char count
}

impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D> {
    #[inline]
    fn __mark_replace(&mut self) {
        self.mutation = None;
    }
}

impl<'ob, S: ?Sized, D> Deref for StringObserver<'ob, S, D> {
    type Target = Pointer<S>;

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

impl<'ob, S: ?Sized, D> QuasiObserver for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
{
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;
}

impl<'ob, S: ?Sized, D> Observer for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            mutation: None,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            mutation: Some(TruncateAppend {
                append_index: value.as_deref().len(),
                truncate_len: 0,
            }),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        Pointer::set(this, value);
    }
}

impl<'ob, S: ?Sized, D> SerializeObserver for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let len = (*this.ptr).as_deref().len();
        let Some(truncate_append) = this.mutation.replace(TruncateAppend {
            append_index: len,
            truncate_len: 0,
        }) else {
            return Ok(MutationKind::Replace(A::serialize_value((*this.ptr).as_deref())?).into());
        };
        let TruncateAppend {
            append_index,
            truncate_len,
        } = truncate_append;
        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(truncate_len));
        }
        #[cfg(feature = "append")]
        if len > append_index {
            mutations.extend(MutationKind::Append(A::serialize_value(
                &(*this.ptr).as_deref()[append_index..],
            )?));
        }
        Ok(mutations)
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
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn __append_index(&mut self) -> usize {
        match &mut self.mutation {
            Some(m) => m.append_index,
            None => 0,
        }
    }

    untracked_methods! { String =>
        pub fn push(&mut self, c: char);
        pub fn push_str(&mut self, s: &str);
        pub fn extend_from_within<R>(&mut self, src: R)
        where { R: RangeBounds<usize> };
    }

    /// See [`String::insert`].
    #[inline]
    pub fn insert(&mut self, idx: usize, ch: char) {
        if idx >= self.__append_index() {
            self.untracked_mut().insert(idx, ch)
        } else {
            self.observed_mut().insert(idx, ch)
        }
    }

    /// See [`String::insert_str`].
    #[inline]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        if idx >= self.__append_index() {
            self.untracked_mut().insert_str(idx, string)
        } else {
            self.observed_mut().insert_str(idx, string)
        }
    }
}

#[cfg(any(feature = "truncate", feature = "append"))]
impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn __mark_truncate(&mut self, range: Range<usize>) {
        let count = (*self).observed_ref()[range.clone()].chars().count();
        let mutation = self.mutation.as_mut().unwrap();
        mutation.truncate_len += count;
        mutation.append_index = range.start;
    }

    /// See [`String::clear`].
    #[inline]
    pub fn clear(&mut self) {
        if self.__append_index() == 0 {
            self.untracked_mut().clear()
        } else {
            self.observed_mut().clear()
        }
    }

    /// See [`String::remove`].
    pub fn remove(&mut self, idx: usize) -> char {
        let char = self.untracked_mut().remove(idx);
        let append_index = self.__append_index();
        if idx >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && idx + char.len_utf8() == append_index {
            let mutation = self.mutation.as_mut().unwrap();
            mutation.truncate_len += 1;
            mutation.append_index = idx;
        } else {
            self.__mark_replace();
        }
        char
    }

    /// See [`String::pop`].
    pub fn pop(&mut self) -> Option<char> {
        let char = self.untracked_mut().pop()?;
        let append_index = self.__append_index();
        let len = (*self).observed_ref().len();
        if len >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && len + char.len_utf8() == append_index {
            let mutation = self.mutation.as_mut().unwrap();
            mutation.truncate_len += 1;
            mutation.append_index = len;
        } else {
            self.__mark_replace();
        }
        Some(char)
    }

    /// See [`String::truncate`].
    pub fn truncate(&mut self, len: usize) {
        let append_index = self.__append_index();
        if len >= append_index {
            return self.untracked_mut().truncate(len);
        }
        if cfg!(not(feature = "truncate")) || len == 0 {
            return self.observed_mut().truncate(len);
        }
        self.__mark_truncate(len..append_index);
        self.untracked_mut().truncate(len)
    }

    /// See [`String::split_off`].
    pub fn split_off(&mut self, at: usize) -> String {
        let append_index = self.__append_index();
        if at >= append_index {
            return self.untracked_mut().split_off(at);
        }
        if cfg!(not(feature = "truncate")) || at == 0 {
            return self.observed_mut().split_off(at);
        }
        self.__mark_truncate(at..append_index);
        self.untracked_mut().split_off(at)
    }

    /// See [`String::drain`].
    pub fn drain<R>(&mut self, range: R) -> Drain<'_>
    where
        R: RangeBounds<usize>,
    {
        let append_index = self.__append_index();
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= append_index {
            return self.untracked_mut().drain(range);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return self.observed_mut().drain(range);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).observed_ref().len(),
        };
        if end_index < append_index {
            return self.observed_mut().drain(range);
        }
        self.__mark_truncate(start_index..append_index);
        self.observed_mut().drain(range)
    }

    /// See [`String::replace_range`].
    pub fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        let append_index = self.__append_index();
        let start_index = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        if start_index >= append_index {
            return self.untracked_mut().replace_range(range, replace_with);
        }
        if cfg!(not(feature = "truncate")) || start_index == 0 {
            return self.observed_mut().replace_range(range, replace_with);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).observed_ref().len(),
        };
        if end_index < append_index {
            return self.observed_mut().replace_range(range, replace_with);
        }
        self.__mark_truncate(start_index..append_index);
        self.untracked_mut().replace_range(range, replace_with);
    }
}

impl<'ob, S: ?Sized, D> AddAssign<&str> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn add_assign(&mut self, rhs: &str) {
        #[cfg(feature = "append")]
        self.push_str(rhs);
        #[cfg(not(feature = "append"))]
        self.observed_mut().add_assign(rhs);
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
        self.untracked_mut().extend(other);
    }
}

impl<'ob, S: ?Sized, D> Debug for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StringObserver").field(&self.observed_ref()).finish()
    }
}

impl<'ob, S: ?Sized, D> Display for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Display,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.observed_ref(), f)
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for String);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? S, D> PartialEq<$ty> for StringObserver<'ob, S, D>
            where
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.observed_ref().eq(other)
                }
            }
        )*
    };
}

generic_impl_partial_eq! {
    impl PartialEq<String> for String;
    impl ['a, U: ?Sized] PartialEq<&'a U> for String;
    impl ['a, U: ToOwned + ?Sized] PartialEq<Cow<'a, U>> for String;
    impl PartialEq<std::path::Path> for String;
    impl PartialEq<std::path::PathBuf> for String;
}

impl<'ob, S1, S2, D1, D2> PartialEq<StringObserver<'ob, S2, D2>> for StringObserver<'ob, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialEq<S2::Target>,
{
    #[inline]
    fn eq(&self, other: &StringObserver<'ob, S2, D2>) -> bool {
        self.observed_ref().eq(other.observed_ref())
    }
}

impl<'ob, S, D> Eq for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
{
}

impl<'ob, S, D> PartialOrd<String> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialOrd<String>,
{
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other)
    }
}

impl<'ob, S1, S2, D1, D2> PartialOrd<StringObserver<'ob, S2, D2>> for StringObserver<'ob, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1>,
    S2: AsDeref<D2>,
    S1::Target: PartialOrd<S2::Target>,
{
    #[inline]
    fn partial_cmp(&self, other: &StringObserver<'ob, S2, D2>) -> Option<std::cmp::Ordering> {
        self.observed_ref().partial_cmp(other.observed_ref())
    }
}

impl<'ob, S, D> Ord for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Ord,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.observed_ref().cmp(other.observed_ref())
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

default_impl_ref_observe! {
    impl RefObserve for String;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::Mutation;
    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_mutation_returns_none() {
        let mut s = String::from("hello");
        let mut ob = s.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn replace_on_deref_mut() {
        let mut s = String::from("hello");
        let mut ob = s.__observe();
        ob.clear();
        ob.push_str("world"); // append after replace should have no effect
        let Json(mutation) = ob.flush().unwrap();
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
        let Json(mutation) = ob.flush().unwrap();
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
        let Json(mutation) = ob.flush().unwrap();
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
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn append_empty_string() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob.push_str("");
        ob += "";
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn replace_after_append() {
        let mut s = String::from("abc");
        let mut ob = s.__observe();
        ob.push_str("def");
        **ob = String::from("xyz");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("xyz")));
    }

    #[test]
    fn truncate() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        ob.truncate("你好".len());
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Truncate(4));
    }

    #[test]
    fn pop_as_truncate() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        ob.pop();
        ob.pop();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Truncate(2));
    }

    #[test]
    fn pop_after_append() {
        let mut s = String::from("你好！");
        let mut ob = s.__observe();
        ob.push_str("世界！");
        ob.pop();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("世界")));
    }

    #[test]
    fn append_after_pop() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        ob.pop();
        ob.push('~');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(
            mutation.unwrap().kind,
            MutationKind::Batch(vec![
                Mutation {
                    path: Default::default(),
                    kind: MutationKind::Truncate(1),
                },
                Mutation {
                    path: Default::default(),
                    kind: MutationKind::Append(json!("~")),
                },
            ])
        );
    }

    #[test]
    fn remove_before_append_index() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        assert_eq!(ob.remove("你好".len()), '，');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("你好世界！")));
    }

    #[test]
    fn remove_at_append_index() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        assert_eq!(ob.remove("你好，世界".len()), '！');
        assert_eq!(ob.remove("你好，世".len()), '界');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Truncate(2));
    }
}
