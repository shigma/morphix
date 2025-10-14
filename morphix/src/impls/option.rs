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
        if this.is_mutated && this.is_initial_some != this.is_some() {
            Ok(Some(Mutation {
                path_rev: vec![],
                operation: MutationKind::Replace(A::serialize_value(&*this)?),
            }))
        } else if let Some(ob) = this.ob.take() {
            Observer::collect(ob)
        } else {
            Ok(None)
        }
    }
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

impl<T: Observe + OptionObserve<T, <T as Observe>::Spec>> Observe for Option<T> {
    type Observer<'i>
        = <T as OptionObserve<T, <T as Observe>::Spec>>::Observer<'i>
    where
        Self: 'i;

    type Spec = <T as OptionObserve<T, <T as Observe>::Spec>>::Spec;
}

/// Helper trait for selecting an appropriate observer for [`Option<T>`].
#[doc(hidden)]
pub trait OptionObserve<T: Observe, S> {
    type Observer<'i>: Observer<'i, Target = Option<T>>
    where
        Self: 'i;

    type Spec;
}

impl<T: Observe<Spec = DefaultSpec>> OptionObserve<T, DefaultSpec> for T {
    type Observer<'i>
        = OptionObserver<'i, T::Observer<'i>>
    where
        Self: 'i;

    type Spec = DefaultSpec;
}
