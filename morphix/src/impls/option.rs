use std::fmt::Debug;
use std::marker::PhantomData;

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Pointer, Unsigned, Zero};
use crate::observe::DefaultSpec;
use crate::{Adapter, Mutation, MutationKind, Observe, Observer};

/// An general observer for [`Option`].
pub struct OptionObserver<'i, O, S: ?Sized, N> {
    ptr: Pointer<S>,
    is_mutated: bool,
    is_initial_some: bool,
    ob: Option<O>,
    phantom: PhantomData<&'i mut N>,
}

impl<'i, O, S: ?Sized, N> Default for OptionObserver<'i, O, S, N> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: Pointer::default(),
            is_mutated: false,
            is_initial_some: false,
            ob: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, O, S: ?Sized, N> std::ops::Deref for OptionObserver<'i, O, S, N> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, O, S: ?Sized, N> std::ops::DerefMut for OptionObserver<'i, O, S, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_mutated = true;
        self.ob = None;
        &mut self.ptr
    }
}

impl<'i, O, S: ?Sized, N> Assignable for OptionObserver<'i, O, S, N> {}

impl<'i, O, S: ?Sized, N, T> Observer for OptionObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Option<T>>,
    O: Observer<UpperDepth = Zero, Head = T>,
    T: Serialize + 'i,
{
    type UpperDepth = N;
    type LowerDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &mut Self::Head) -> Self {
        Self {
            ptr: Pointer::new(value),
            is_mutated: false,
            is_initial_some: (*value).as_deref().is_some(),
            ob: value.as_deref_mut().as_mut().map(O::observe),
            phantom: PhantomData,
        }
    }

    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        if this.is_mutated && (this.is_initial_some || (*this.ptr).as_deref().is_some()) {
            Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value((*this.ptr).as_deref())?),
            }))
        } else if let Some(mut ob) = this.ob.take() {
            Observer::collect(&mut ob)
        } else {
            Ok(None)
        }
    }
}

impl<'i, O, S: ?Sized, N, T> OptionObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target = Option<T>>,
    O: Observer<UpperDepth = Zero, Head = T>,
    T: 'i,
{
    fn __insert(&mut self, value: T) {
        self.is_mutated = true;
        let inserted = (*self.ptr).as_deref_mut().insert(value);
        self.ob = Some(O::observe(inserted));
    }

    pub fn as_mut(&mut self) -> Option<&mut O> {
        if self.as_deref().is_some() && self.ob.is_none() {
            self.ob = (*self.ptr).as_deref_mut().as_mut().map(O::observe);
        }
        self.ob.as_mut()
    }

    pub fn insert(&mut self, value: T) -> &mut O {
        self.__insert(value);
        self.ob.as_mut().unwrap()
    }

    #[inline]
    pub fn get_or_insert(&mut self, value: T) -> &mut O {
        self.get_or_insert_with(|| value)
    }

    #[inline]
    pub fn get_or_insert_default(&mut self) -> &mut O
    where
        T: Default,
    {
        self.get_or_insert_with(Default::default)
    }

    pub fn get_or_insert_with<F>(&mut self, f: F) -> &mut O
    where
        F: FnOnce() -> T,
    {
        if self.as_deref().is_none() {
            self.__insert(f());
        }
        self.ob.as_mut().unwrap()
    }
}

impl<'i, O, S: ?Sized, N> Debug for OptionObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target: Debug>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OptionObserver").field(&(*self.ptr).as_deref()).finish()
    }
}

impl<'i, O, S: ?Sized, N, U: ?Sized> PartialEq<U> for OptionObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target: PartialEq<U>>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        (*self.ptr).as_deref().eq(other)
    }
}

impl<'i, O, S: ?Sized, N, U: ?Sized> PartialOrd<U> for OptionObserver<'i, O, S, N>
where
    N: Unsigned,
    S: AsDerefMut<N, Target: PartialOrd<U>>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (*self.ptr).as_deref().partial_cmp(other)
    }
}

impl<T> Observe for Option<T>
where
    T: Observe + OptionObserveImpl<T, T::Spec>,
{
    type Observer<'i, S, N>
        = <T as OptionObserveImpl<T, T::Spec>>::Observer<'i, S, N>
    where
        Self: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Self> + ?Sized + 'i;

    type Spec = T::Spec;
}

/// Helper trait for selecting an appropriate observer for [`Option<T>`].
#[doc(hidden)]
pub trait OptionObserveImpl<T: Observe, Spec> {
    type Observer<'i, S, N>: Observer<Head = S, UpperDepth = N>
    where
        T: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Option<T>> + ?Sized + 'i;
}

impl<T> OptionObserveImpl<T, DefaultSpec> for T
where
    T: Observe<Spec = DefaultSpec>,
{
    type Observer<'i, S, N>
        = OptionObserver<'i, T::Observer<'i, T, Zero>, S, N>
    where
        T: 'i,
        N: Unsigned,
        S: AsDerefMut<N, Target = Option<T>> + ?Sized + 'i;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::helper::ObserveExt;
    use crate::impls::string::StringObserver;
    use crate::observe::{DefaultSpec, GeneralObserver, ShallowObserver};
    use crate::{JsonAdapter, Observer};

    #[derive(Debug, Serialize, Default)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'i, S, N>
            = ShallowObserver<'i, S, N>
        where
            Self: 'i,
            N: Unsigned,
            S: AsDerefMut<N, Target = Self> + ?Sized + 'i;

        type Spec = DefaultSpec;
    }

    #[test]
    fn no_change_returns_none() {
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        assert!(Observer::collect::<JsonAdapter>(&mut ob).unwrap().is_none());

        let mut opt = Some(Number(1));
        let mut ob = opt.__observe();
        assert!(Observer::collect::<JsonAdapter>(&mut ob).unwrap().is_none());
    }

    #[test]
    fn deref_triggers_replace() {
        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        **ob = None;
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(null)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        **ob = Some(Number(42));
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(42)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        **ob = Some(Number(42));
        **ob = None;
        assert!(Observer::collect::<JsonAdapter>(&mut ob).unwrap().is_none());

        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        **ob = None;
        **ob = Some(Number(42));
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(42)));
    }

    #[test]
    fn insert_returns_observer() {
        let mut opt: Option<String> = None;
        let mut ob = opt.__observe();
        let s: &mut StringObserver<'_, _, _> = ob.insert(String::from("99")); // assert type
        *s += "9";
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!("999")));
    }

    #[test]
    fn as_mut_tracks_inner() {
        let mut opt = Some(String::from("foo"));
        let mut ob = opt.__observe();
        *ob.as_mut().unwrap() += "bar";
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn get_or_insert() {
        // get_or_insert
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert(Number(5)).0 = 6;
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(6)));

        // get_or_insert_default
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_default().0 = 77;
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(77)));

        // get_or_insert_with
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_with(|| Number(88)).0 = 99;
        let mutation = Observer::collect::<JsonAdapter>(&mut ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(99)));
    }

    #[test]
    fn specialization() {
        let mut opt = Some(0i32);
        let _ob: GeneralObserver<'_, _, _, _> = opt.__observe(); // assert type

        let mut opt = Some(Number(0));
        let _ob: OptionObserver<'_, _, _, _> = opt.__observe(); // assert type
    }
}
