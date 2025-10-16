use std::marker::PhantomData;

use serde::Serialize;

use crate::helper::Assignable;
use crate::observe::DefaultSpec;
use crate::{Adapter, Mutation, MutationKind, Observe, Observer};

/// An general observer for [`Option`].
pub struct OptionObserver<'i, O: Observer<'i, Target: Sized>> {
    ptr: *mut Option<O::Target>,
    is_initial_some: bool,
    is_mutated: bool,
    ob: Option<O>,
    phantom: PhantomData<&'i mut O::Target>,
}

impl<'i, O: Observer<'i, Target: Sized>> Default for OptionObserver<'i, O> {
    #[inline]
    fn default() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            is_initial_some: false,
            is_mutated: false,
            ob: None,
            phantom: PhantomData,
        }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> std::ops::Deref for OptionObserver<'i, O> {
    type Target = Option<O::Target>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> std::ops::DerefMut for OptionObserver<'i, O> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_mutated = true;
        self.ob = None;
        unsafe { &mut *self.ptr }
    }
}

impl<'i, O: Observer<'i, Target: Sized>> Assignable for OptionObserver<'i, O> {}

impl<'i, O: Observer<'i, Target: Serialize + Sized>> Observer<'i> for OptionObserver<'i, O> {
    #[inline]
    fn inner(this: &Self) -> *mut Self::Target {
        this.ptr
    }

    #[inline]
    fn observe(value: &'i mut Option<O::Target>) -> Self {
        Self {
            ptr: value,
            is_initial_some: value.is_some(),
            is_mutated: false,
            ob: value.as_mut().map(O::observe),
            phantom: PhantomData,
        }
    }

    unsafe fn collect_unchecked<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
        if this.is_mutated && (this.is_initial_some || this.is_some()) {
            Ok(Some(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(&*this)?),
            }))
        } else if let Some(ob) = this.ob.take() {
            Observer::collect(ob)
        } else {
            Ok(None)
        }
    }

    type Spec = DefaultSpec;
}

impl<'i, O: Observer<'i, Target: Sized>> OptionObserver<'i, O> {
    fn __insert(&mut self, value: O::Target) {
        self.is_mutated = true;
        let inner = unsafe { &mut *self.ptr };
        self.ob = Some(O::observe(inner.insert(value)));
    }

    pub fn as_mut(&mut self) -> Option<&mut O> {
        if self.is_some() && self.ob.is_none() {
            let inner = unsafe { &mut *self.ptr };
            self.ob = inner.as_mut().map(O::observe);
        }
        self.ob.as_mut()
    }

    pub fn insert(&mut self, value: O::Target) -> &mut O {
        self.__insert(value);
        self.ob.as_mut().unwrap()
    }

    #[inline]
    pub fn get_or_insert(&mut self, value: O::Target) -> &mut O {
        self.get_or_insert_with(|| value)
    }

    #[inline]
    pub fn get_or_insert_default(&mut self) -> &mut O
    where
        O::Target: Default,
    {
        self.get_or_insert_with(Default::default)
    }

    pub fn get_or_insert_with<F>(&mut self, f: F) -> &mut O
    where
        F: FnOnce() -> O::Target,
    {
        if self.is_none() {
            self.__insert(f());
        }
        self.ob.as_mut().unwrap()
    }
}

impl<T, S> Observe for Option<T>
where
    T: Observe + OptionObserveImpl<T, S>,
    for<'i> <T as Observe>::Observer<'i>: Observer<'i, Spec = S>,
{
    type Observer<'i>
        = <T as OptionObserveImpl<T, S>>::Observer<'i>
    where
        Self: 'i;
}

/// Helper trait for selecting an appropriate observer for [`Option<T>`].
#[doc(hidden)]
pub trait OptionObserveImpl<T: Observe, S> {
    type Observer<'i>: Observer<'i, Target = Option<T>>
    where
        Self: 'i;
}

impl<T> OptionObserveImpl<T, DefaultSpec> for T
where
    T: Observe,
    for<'i> <T as Observe>::Observer<'i>: Observer<'i, Spec = DefaultSpec>,
{
    type Observer<'i>
        = OptionObserver<'i, T::Observer<'i>>
    where
        Self: 'i;
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::helper::ObserveExt;
    use crate::impls::string::StringObserver;
    use crate::observe::{GeneralObserver, ShallowObserver};
    use crate::{JsonAdapter, Observer};

    #[derive(Serialize, Default)]
    struct Number(i32);

    impl Observe for Number {
        type Observer<'i>
            = ShallowObserver<'i, Self>
        where
            Self: 'i;
    }

    #[test]
    fn no_change_returns_none() {
        let mut opt: Option<Number> = None;
        let ob = opt.__observe();
        assert!(Observer::collect::<JsonAdapter>(ob).unwrap().is_none());

        let mut opt = Some(Number(1));
        let ob = opt.__observe();
        assert!(Observer::collect::<JsonAdapter>(ob).unwrap().is_none());
    }

    #[test]
    fn deref_triggers_replace() {
        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        *ob = None;
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(null)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        *ob = Some(Number(42));
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(42)));

        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        *ob = Some(Number(42));
        *ob = None;
        assert!(Observer::collect::<JsonAdapter>(ob).unwrap().is_none());

        let mut opt = Some(Number(42));
        let mut ob = opt.__observe();
        *ob = None;
        *ob = Some(Number(42));
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(42)));
    }

    #[test]
    fn insert_returns_observer() {
        let mut opt: Option<String> = None;
        let mut ob = opt.__observe();
        let s: &mut StringObserver<'_> = ob.insert(String::from("99")); // assert type
        *s += "9";
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!("999")));
    }

    #[test]
    fn as_mut_tracks_inner() {
        let mut opt = Some(String::from("foo"));
        let mut ob = opt.__observe();
        *ob.as_mut().unwrap() += "bar";
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Append(json!("bar")));
    }

    #[test]
    fn get_or_insert() {
        // get_or_insert
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert(Number(5)).0 = 6;
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(6)));

        // get_or_insert_default
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_default().0 = 77;
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(77)));

        // get_or_insert_with
        let mut opt: Option<Number> = None;
        let mut ob = opt.__observe();
        ob.get_or_insert_with(|| Number(88)).0 = 99;
        let mutation = Observer::collect::<JsonAdapter>(ob).unwrap().unwrap();
        assert_eq!(mutation.kind, MutationKind::Replace(json!(99)));
    }

    #[test]
    fn specialization() {
        let mut opt = Some(0i32);
        let _ob: GeneralObserver<'_, _, _> = opt.__observe(); // assert type

        let mut opt = Some(Number(0));
        let _ob: OptionObserver<'_, _> = opt.__observe(); // assert type
    }
}
