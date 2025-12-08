use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDerefMut, AsNormalized, Succ, Unsigned, Zero};
use crate::observe::{Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe};

/// Observer implementation for [`Option`].
///
/// This observer tracks changes to optional values, including transitions between [`Some`] and
/// [`None`] states. It provides specialized methods for working with options while maintaining
/// change tracking.
pub struct OptionObserver<'ob, O, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    is_mutated: bool,
    is_initial_some: bool,
    ob: Option<O>,
    phantom: PhantomData<&'ob mut D>,
}

impl<'ob, O, S: ?Sized, D> Deref for OptionObserver<'ob, O, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'ob, O, S: ?Sized, D> DerefMut for OptionObserver<'ob, O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_mutated = true;
        self.ob = None;
        &mut self.ptr
    }
}

impl<'ob, O, S: ?Sized, D> AsNormalized for OptionObserver<'ob, O, S, D> {
    type OuterDepth = Succ<Zero>;
}

impl<'ob, O, S: ?Sized, D> Observer<'ob> for OptionObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
    O::Head: Sized,
{
    type InnerDepth = D;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            is_mutated: false,
            is_initial_some: false,
            ob: None,
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        ObserverPointer::set(Self::as_ptr(this), value);
        match (&mut this.ob, value.as_deref_mut()) {
            (Some(inner), Some(value)) => unsafe { Observer::refresh(inner, value) },
            (None, None) => {}
            _ => unreachable!("inconsistent option observer state"),
        }
    }

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            is_mutated: false,
            is_initial_some: value.as_deref().is_some(),
            ob: value.as_deref_mut().as_mut().map(O::observe),
            phantom: PhantomData,
        }
    }
}

impl<'ob, O, S: ?Sized, D> SerializeObserver<'ob> for OptionObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>> + 'ob,
    O: SerializeObserver<'ob, InnerDepth = Zero>,
    O::Head: Serialize + Sized,
{
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        if this.is_mutated && (this.is_initial_some || this.as_deref().is_some()) {
            Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref())?),
            }))
        } else if let Some(mut ob) = this.ob.take() {
            SerializeObserver::flush::<A>(&mut ob)
        } else {
            Ok(None)
        }
    }
}

impl<'ob, O, S: ?Sized, D> OptionObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = Option<O::Head>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
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

impl<'ob, O, S: ?Sized, D> Debug for OptionObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target: Debug>,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OptionObserver").field(&self.as_deref()).finish()
    }
}

impl<'ob, O, S: ?Sized, D, U: ?Sized> PartialEq<U> for OptionObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target: PartialEq<U>>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.as_deref().eq(other)
    }
}

impl<'ob, O, S: ?Sized, D, U: ?Sized> PartialOrd<U> for OptionObserver<'ob, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target: PartialOrd<U>>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.as_deref().partial_cmp(other)
    }
}

spec_impl_observe!(OptionObserveImpl, Option<Self>, Option<T>, OptionObserver);
spec_impl_ref_observe!(OptionRefObserveImpl, Option<Self>, Option<T>);

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::impls::string::StringObserver;
    use crate::observe::{DefaultSpec, GeneralObserver, ObserveExt, SerializeObserverExt, ShallowObserver};

    #[derive(Debug, Serialize, Default)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'ob, S, D>
            = ShallowObserver<'ob, S, D>
        where
            Self: 'ob,
            D: Unsigned,
            S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

        type Spec = DefaultSpec;
    }

    #[test]
    fn no_change_returns_none() {
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());

        let mut opt = Some(Number(1));
        let mut ob = opt.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());
    }

    #[test]
    fn deref_triggers_replace() {
        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        **ob = None;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(null)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        **ob = Some(Number(42));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(42)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        **ob = Some(Number(42));
        **ob = None;
        let Json(mutation) = ob.flush().unwrap();
        assert!(mutation.is_none());

        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        **ob = None;
        **ob = Some(Number(42));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(42)));
    }

    #[test]
    fn insert_returns_observer() {
        let mut opt: Option<String> = None;
        let mut ob = opt.__observe();
        let s: &mut StringObserver<_, _> = ob.insert(String::from("99")); // assert type
        *s += "9";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!("999")));
    }

    #[test]
    fn as_mut_tracks_inner() {
        let mut opt = Some(String::from("foo"));
        let mut ob = opt.__observe();
        *ob.as_mut().unwrap() += "bar";
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn get_or_insert() {
        // get_or_insert
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert(Number(5)).0 = 6;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(6)));

        // get_or_insert_default
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_default().0 = 77;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(77)));

        // get_or_insert_with
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_with(|| Number(88)).0 = 99;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation.unwrap().kind, MutationKind::Replace(json!(99)));
    }

    #[test]
    fn specialization() {
        let mut opt = Some(0i32);
        let _ob: GeneralObserver<_, _, _> = opt.__observe(); // assert type

        let mut opt = Some(Number(0));
        let _ob: OptionObserver<_, _, _> = opt.__observe(); // assert type
    }
}
