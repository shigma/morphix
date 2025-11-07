use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe};

/// Observer implementation for [`Option`].
///
/// This observer tracks changes to optional values, including transitions between [`Some`] and
/// [`None`] states. It provides specialized methods for working with options while maintaining
/// change tracking.
pub struct OptionObserver<'i, O, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    is_mutated: bool,
    is_initial_some: bool,
    ob: Option<O>,
    phantom: PhantomData<&'i mut D>,
}

impl<'i, O, S: ?Sized, D> Default for OptionObserver<'i, O, S, D> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            is_mutated: false,
            is_initial_some: false,
            ob: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, O, S: ?Sized, D> Deref for OptionObserver<'i, O, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, O, S: ?Sized, D> DerefMut for OptionObserver<'i, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_mutated = true;
        self.ob = None;
        &mut self.ptr
    }
}

impl<'i, O, S> Assignable for OptionObserver<'i, O, S> {
    type Depth = Succ<Zero>;
}

impl<'i, O, S: ?Sized, D> Observer<'i> for OptionObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>> + 'i,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            is_mutated: false,
            is_initial_some: value.as_deref().is_some(),
            ob: value.as_deref_mut().as_mut().map(O::observe),
            phantom: PhantomData,
        }
    }
}

impl<'i, O, S: ?Sized, D> SerializeObserver<'i> for OptionObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>> + 'i,
    O: SerializeObserver<'i, InnerDepth = Zero>,
    O::Head: Serialize + Sized,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        if this.is_mutated && (this.is_initial_some || this.as_deref().is_some()) {
            Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref())?),
            }))
        } else if let Some(mut ob) = this.ob.take() {
            SerializeObserver::collect(&mut ob)
        } else {
            Ok(None)
        }
    }
}

impl<'i, O, S: ?Sized, D> OptionObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>> + 'i,
    O: Observer<'i, InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn __insert(&mut self, value: O::Head) {
        self.is_mutated = true;
        let inserted = Observer::as_inner(self).insert(value);
        self.ob = Some(O::observe(inserted));
    }

    /// See [`Option::as_mut`].
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut O> {
        if self.as_deref().is_some() && self.ob.is_none() {
            self.ob = Observer::as_inner(self).as_mut().map(O::observe);
        }
        self.ob.as_mut()
    }

    /// See [`Option::insert`].
    #[inline]
    pub fn insert(&mut self, value: O::Head) -> &mut O {
        self.__insert(value);
        self.ob.as_mut().unwrap()
    }

    /// See [`Option::get_or_insert`].
    #[inline]
    pub fn get_or_insert(&mut self, value: O::Head) -> &mut O {
        self.get_or_insert_with(|| value)
    }

    /// See [`Option::get_or_insert_default`].
    #[inline]
    pub fn get_or_insert_default(&mut self) -> &mut O
    where
        O::Head: Default,
    {
        self.get_or_insert_with(Default::default)
    }

    /// See [`Option::get_or_insert_with`].
    #[inline]
    pub fn get_or_insert_with<F>(&mut self, f: F) -> &mut O
    where
        F: FnOnce() -> O::Head,
    {
        if self.as_deref().is_none() {
            self.__insert(f());
        }
        self.ob.as_mut().unwrap()
    }
}

impl<'i, O, S: ?Sized, D> Debug for OptionObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target: Debug>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OptionObserver").field(&self.as_deref()).finish()
    }
}

impl<'i, O, S: ?Sized, D, U: ?Sized> PartialEq<U> for OptionObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target: PartialEq<U>>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'i, O, S: ?Sized, D, U: ?Sized> PartialOrd<U> for OptionObserver<'i, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target: PartialOrd<U>>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

impl<T> Observe for Option<T>
where
    T: Observe + OptionObserveImpl<T::Spec>,
{
    type Observer<'i, S, D>
        = <T as OptionObserveImpl<T::Spec>>::Observer<'i, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = T::Spec;
}

/// Helper trait for selecting appropriate observer implementations for [`Option<T>`].
#[doc(hidden)]
pub trait OptionObserveImpl<Spec> {
    /// The observer type for [`Option<T>`] with the given specification.
    type Observer<'i, S, D>: Observer<'i, Head = S, InnerDepth = D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Option<Self>> + ?Sized + 'i;
}

impl<T> OptionObserveImpl<DefaultSpec> for T
where
    T: Observe<Spec = DefaultSpec>,
{
    type Observer<'i, S, D>
        = OptionObserver<'i, T::Observer<'i, T, Zero>, S, D>
    where
        T: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Option<T>> + ?Sized + 'i;
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::JsonAdapter;
    use crate::impls::string::StringObserver;
    use crate::observe::{DefaultSpec, GeneralObserver, ObserveExt, SerializeObserverExt, ShallowObserver};

    #[derive(Debug, Serialize, Default)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'i, S, D>
            = ShallowObserver<'i, S, D>
        where
            Self: 'i,
            D: Unsigned,
            S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

        type Spec = DefaultSpec;
    }

    #[test]
    fn no_change_returns_none() {
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());

        let mut opt = Some(Number(1));
        let mut ob = opt.__observe();
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());
    }

    #[test]
    fn deref_triggers_replace() {
        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        **ob = None;
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(null)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        **ob = Some(Number(42));
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(42)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        **ob = Some(Number(42));
        **ob = None;
        assert!(ob.collect::<JsonAdapter>().unwrap().is_none());

        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        **ob = None;
        **ob = Some(Number(42));
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(42)));
    }

    #[test]
    fn insert_returns_observer() {
        let mut opt: Option<String> = None;
        let mut ob = opt.__observe();
        let s: &mut StringObserver<_, _> = ob.insert(String::from("99")); // assert type
        *s += "9";
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!("999")));
    }

    #[test]
    fn as_mut_tracks_inner() {
        let mut opt = Some(String::from("foo"));
        let mut ob = opt.__observe();
        *ob.as_mut().unwrap() += "bar";
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn get_or_insert() {
        // get_or_insert
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert(Number(5)).0 = 6;
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(6)));

        // get_or_insert_default
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_default().0 = 77;
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(77)));

        // get_or_insert_with
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_with(|| Number(88)).0 = 99;
        let mutation = ob.collect::<JsonAdapter>().unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(99)));
    }

    #[test]
    fn specialization() {
        let mut opt = Some(0i32);
        let _ob: GeneralObserver<_, _, _> = opt.__observe(); // assert type

        let mut opt = Some(Number(0));
        let _ob: OptionObserver<_, _, _> = opt.__observe(); // assert type
    }
}
