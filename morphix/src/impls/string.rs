use std::borrow::Cow;
use std::collections::TryReserveError;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::ops::{AddAssign, Bound, Deref, DerefMut, Range, RangeBounds};
use std::string::Drain;

use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
use crate::helper::{AsDeref, AsDerefMut, ObserverState, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{MutationKind, Mutations, Observe};

struct StringObserverState {
    pub append_index: usize, // byte index
    pub truncate_len: usize, // char count
}

impl StringObserverState {
    #[inline]
    fn mark_truncate(&mut self, value: &str, new_len: usize) {
        let count = value[new_len..].chars().count();
        self.truncate_len += count;
        self.append_index = new_len;
    }

    #[inline]
    fn mark_replace(&mut self, value: &str) {
        self.mark_truncate(value, 0);
    }
}

impl ObserverState for StringObserverState {
    type Target = String;

    #[inline]
    fn invalidate(this: &mut Self, value: &String) {
        this.mark_replace(value.as_str());
    }
}

/// Observer implementation for [`String`].
pub struct StringObserver<'ob, S: ?Sized, D = Zero> {
    ptr: Pointer<S>,
    mutation: StringObserverState,
    phantom: PhantomData<&'ob mut D>,
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
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
}

impl<'ob, S: ?Sized, D> QuasiObserver for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        ObserverState::invalidate(&mut this.mutation, (*this.ptr).as_deref());
    }
}

impl<'ob, S: ?Sized, D> Observer for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            mutation: StringObserverState {
                append_index: 0,
                truncate_len: 0,
            },
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(mut head: &mut Self::Head) -> Self {
        let mut this = Self {
            ptr: Pointer::new(&mut head),
            mutation: StringObserverState {
                append_index: head.as_deref_mut().len(),
                truncate_len: 0,
            },
            phantom: PhantomData,
        };
        Pointer::register_state::<_, D>(&mut this.ptr, &mut this.mutation);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, mut head: &mut Self::Head) {
        Pointer::set(this, &mut head);
    }
}

impl<'ob, S: ?Sized, D> SerializeObserver for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let value = (*this.ptr).as_deref();
        let len = value.len();
        let append_index = std::mem::replace(&mut this.mutation.append_index, len);
        let truncate_len = std::mem::replace(&mut this.mutation.truncate_len, 0);
        // After invalidate (mark_replace), append_index = 0 and truncate_len > 0,
        // meaning all tracked content was replaced.
        if append_index == 0 && truncate_len > 0 {
            return Mutations::replace(value);
        }
        let mut mutations = Mutations::new();
        #[cfg(feature = "truncate")]
        if truncate_len > 0 {
            mutations.extend(MutationKind::Truncate(truncate_len));
        }
        #[cfg(feature = "append")]
        if len > append_index {
            mutations.extend(Mutations::append(&value[append_index..]));
        }
        mutations
    }
}

impl<'ob, S: ?Sized, D> StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = String>,
{
    delegate_methods! { untracked_mut as String =>
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
        self.mutation.append_index
    }

    delegate_methods! { untracked_mut as String =>
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
            self.tracked_mut().insert(idx, ch)
        }
    }

    /// See [`String::insert_str`].
    #[inline]
    pub fn insert_str(&mut self, idx: usize, string: &str) {
        if idx >= self.__append_index() {
            self.untracked_mut().insert_str(idx, string)
        } else {
            self.tracked_mut().insert_str(idx, string)
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
        let count = (*self).untracked_ref()[range.clone()].chars().count();
        self.mutation.truncate_len += count;
        self.mutation.append_index = range.start;
    }

    /// See [`String::clear`].
    #[inline]
    pub fn clear(&mut self) {
        if self.__append_index() == 0 {
            self.untracked_mut().clear()
        } else {
            self.tracked_mut().clear()
        }
    }

    /// See [`String::remove`].
    pub fn remove(&mut self, idx: usize) -> char {
        let char = self.untracked_mut().remove(idx);
        let append_index = self.__append_index();
        if idx >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && idx + char.len_utf8() == append_index {
            self.mutation.truncate_len += 1;
            self.mutation.append_index = idx;
        } else {
            let value = (*self.ptr).as_deref();
            self.mutation.mark_replace(value);
        }
        char
    }

    /// See [`String::pop`].
    pub fn pop(&mut self) -> Option<char> {
        let char = self.untracked_mut().pop()?;
        let append_index = self.__append_index();
        let len = (*self).untracked_ref().len();
        if len >= append_index {
            // no-op
        } else if cfg!(feature = "truncate") && len + char.len_utf8() == append_index {
            self.mutation.truncate_len += 1;
            self.mutation.append_index = len;
        } else {
            let value = (*self.ptr).as_deref();
            self.mutation.mark_replace(value);
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
            return self.tracked_mut().truncate(len);
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
            return self.tracked_mut().split_off(at);
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
            return self.tracked_mut().drain(range);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).untracked_ref().len(),
        };
        if end_index < append_index {
            return self.tracked_mut().drain(range);
        }
        self.__mark_truncate(start_index..append_index);
        self.tracked_mut().drain(range)
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
            return self.tracked_mut().replace_range(range, replace_with);
        }
        let end_index = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => (*self).untracked_ref().len(),
        };
        if end_index < append_index {
            return self.tracked_mut().replace_range(range, replace_with);
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
        self.tracked_mut().add_assign(rhs);
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
    S: AsDeref<D, Target = String>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("StringObserver").field(&self.untracked_ref()).finish()
    }
}

impl<'ob, S: ?Sized, D> Display for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.untracked_ref(), f)
    }
}

macro_rules! generic_impl_partial_eq {
    ($(impl $([$($gen:tt)*])? PartialEq<$ty:ty> for String);* $(;)?) => {
        $(
            impl<'ob, $($($gen)*,)? S, D> PartialEq<$ty> for StringObserver<'ob, S, D>
            where
                D: Unsigned,
                S: AsDeref<D, Target = String>,
                String: PartialEq<$ty>,
            {
                #[inline]
                fn eq(&self, other: &$ty) -> bool {
                    self.untracked_ref().eq(other)
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
    S1: AsDeref<D1, Target = String>,
    S2: AsDeref<D2, Target = String>,
{
    #[inline]
    fn eq(&self, other: &StringObserver<'ob, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<'ob, S, D> Eq for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
}

impl<'ob, S, D> PartialOrd<String> for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other)
    }
}

impl<'ob, S1, S2, D1, D2> PartialOrd<StringObserver<'ob, S2, D2>> for StringObserver<'ob, S1, D1>
where
    D1: Unsigned,
    D2: Unsigned,
    S1: AsDeref<D1, Target = String>,
    S2: AsDeref<D2, Target = String>,
{
    #[inline]
    fn partial_cmp(&self, other: &StringObserver<'ob, S2, D2>) -> Option<std::cmp::Ordering> {
        self.untracked_ref().partial_cmp(other.untracked_ref())
    }
}

impl<'ob, S, D> Ord for StringObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = String>,
{
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.untracked_ref().cmp(other.untracked_ref())
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
    use morphix_test_utils::*;
    use serde_json::json;

    use crate::adapter::Json;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn no_mutation_returns_none() {
        let mut s = String::from("hello");
        let mut ob = s.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn replace_on_deref_mut() {
        let mut s = String::from("hello");
        let mut ob = s.__observe();
        ob.clear();
        ob.push_str("world"); // append after replace should have no effect
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!("world"))));
    }

    #[test]
    fn append_with_push() {
        let mut s = String::from("a");
        let mut ob = s.__observe();
        ob.push('b');
        ob.push('c');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!("bc"))));
    }

    #[test]
    fn append_with_push_str() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob.push_str("bar");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!("bar"))));
    }

    #[test]
    fn append_with_add_assign() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob += "bar";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!("bar"))));
    }

    #[test]
    fn append_empty_string() {
        let mut s = String::from("foo");
        let mut ob = s.__observe();
        ob.push_str("");
        ob += "";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn replace_after_append() {
        let mut s = String::from("abc");
        let mut ob = s.__observe();
        ob.push_str("def");
        **ob = String::from("xyz");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!("xyz"))));
    }

    #[test]
    fn truncate() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        ob.truncate("你好".len());
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 4)));
    }

    #[test]
    fn pop_as_truncate() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        ob.pop();
        ob.pop();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 2)));
    }

    #[test]
    fn pop_after_append() {
        let mut s = String::from("你好！");
        let mut ob = s.__observe();
        ob.push_str("世界！");
        ob.pop();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(_, json!("世界"))));
    }

    #[test]
    fn append_after_pop() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        ob.pop();
        ob.push('~');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(batch!(_, truncate!(_, 1), append!(_, json!("~")))));
    }

    #[test]
    fn remove_before_append_index() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        assert_eq!(ob.remove("你好".len()), '，');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!("你好世界！"))));
    }

    #[test]
    fn remove_at_append_index() {
        let mut s = String::from("你好，世界！");
        let mut ob = s.__observe();
        assert_eq!(ob.remove("你好，世界".len()), '！');
        assert_eq!(ob.remove("你好，世".len()), '界');
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(truncate!(_, 2)));
    }
}
