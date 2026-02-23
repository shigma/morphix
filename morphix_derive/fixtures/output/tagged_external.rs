#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Foo<S, T, U>
where
    T: Clone,
{
    A(S),
    B(u32, U),
    #[serde(rename_all = "UPPERCASE")]
    #[serde(rename = "OwO")]
    C { #[serde(skip)] bar: Option<T>, #[serde(rename = "QwQ")] qux: Qux },
    D,
    E(),
    F {},
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S, T, U, _S: ?Sized, N = ::morphix::helper::Zero>
    where
        T: Clone,
        U: ::morphix::Observe + 'ob,
    {
        __ptr: ::morphix::helper::Pointer<_S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __initial: FooObserverInitial,
        __variant: FooObserverVariant<'ob, S, T, U>,
    }
    #[derive(Clone, Copy)]
    pub enum FooObserverInitial {
        D,
        E,
        F,
        __None,
    }
    impl FooObserverInitial {
        fn new<S, T, U>(value: &Foo<S, T, U>) -> Self
        where
            T: Clone,
        {
            match value {
                Foo::D => FooObserverInitial::D,
                Foo::E() => FooObserverInitial::E,
                Foo::F {} => FooObserverInitial::F,
                _ => FooObserverInitial::__None,
            }
        }
    }
    pub enum FooObserverVariant<'ob, S, T, U>
    where
        T: Clone,
        U: ::morphix::Observe + 'ob,
    {
        A(::morphix::helper::Pointer<S>),
        B(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, U>,
        ),
        C {
            bar: ::morphix::helper::Pointer<Option<T>>,
            qux: ::morphix::observe::DefaultObserver<'ob, Qux>,
        },
        __None,
    }
    impl<'ob, S, T, U> FooObserverVariant<'ob, S, T, U>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        fn observe(value: &Foo<S, T, U>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::helper::Pointer::new(v0)),
                Foo::B(v0, v1) => {
                    Self::B(
                        ::morphix::observe::Observer::observe(v0),
                        ::morphix::observe::Observer::observe(v1),
                    )
                }
                Foo::C { bar, qux } => {
                    Self::C {
                        bar: ::morphix::helper::Pointer::new(bar),
                        qux: ::morphix::observe::Observer::observe(qux),
                    }
                }
                _ => Self::__None,
            }
        }
        unsafe fn refresh(&mut self, value: &Foo<S, T, U>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::helper::Pointer::set(u0, v0);
                    }
                    (Self::B(u0, u1), Foo::B(v0, v1)) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1);
                    }
                    (Self::C { bar: u0, qux: u1 }, Foo::C { bar: v0, qux: v1 }) => {
                        ::morphix::helper::Pointer::set(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1);
                    }
                    (Self::__None, _) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush<A: ::morphix::Adapter>(
            &mut self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error>
        where
            ::morphix::observe::DefaultObserver<
                'ob,
                U,
            >: ::morphix::observe::SerializeObserver<'ob>,
        {
            match self {
                Self::A(_) => Ok(::morphix::Mutations::new()),
                Self::B(u0, u1) => {
                    let mutations_0 = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u0)?;
                    let mutations_1 = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u1)?;
                    let mut mutations = ::morphix::Mutations::with_capacity(
                        mutations_0.len() + mutations_1.len(),
                    );
                    mutations.insert2("b", 0usize, mutations_0);
                    mutations.insert2("b", 1usize, mutations_1);
                    Ok(mutations)
                }
                Self::C { qux, .. } => {
                    let mutations_qux = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(qux)?;
                    let mut mutations = ::morphix::Mutations::with_capacity(
                        mutations_qux.len(),
                    );
                    mutations.insert2("OwO", "QwQ", mutations_qux);
                    Ok(mutations)
                }
                Self::__None => Ok(::morphix::Mutations::new()),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::std::ops::Deref
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        type Target = ::morphix::helper::Pointer<_S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::std::ops::DerefMut
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            self.__variant = FooObserverVariant::__None;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::morphix::helper::AsNormalized
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        U: ::morphix::Observe,
    {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, S, T, U, _S, N>
    where
        T: Clone,
        S: 'ob,
        Option<T>: 'ob,
        U: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<N, Target = Foo<S, T, U>> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = _S;
        type InnerDepth = N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::helper::Pointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __initial: FooObserverInitial::__None,
                __variant: FooObserverVariant::__None,
            }
        }
        fn observe(value: &_S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __initial: FooObserverInitial::new(__value),
                __variant: FooObserverVariant::observe(__value),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &_S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref();
            unsafe { this.__variant.refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, U, _S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, S, T, U, _S, N>
    where
        Foo<S, T, U>: ::serde::Serialize,
        T: Clone,
        S: 'ob,
        Option<T>: 'ob,
        U: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<N, Target = Foo<S, T, U>> + 'ob,
        N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            U,
        >: ::morphix::observe::SerializeObserver<'ob>,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            let __value = this.__ptr.as_deref();
            let __initial = this.__initial;
            this.__initial = FooObserverInitial::new(__value);
            if !this.__mutated {
                return this.__variant.flush::<A>();
            }
            this.__mutated = false;
            this.__variant = FooObserverVariant::__None;
            match (__initial, __value) {
                (FooObserverInitial::D, Foo::D)
                | (FooObserverInitial::E, Foo::E())
                | (FooObserverInitial::F, Foo::F {}) => Ok(::morphix::Mutations::new()),
                _ => {
                    Ok(
                        ::morphix::MutationKind::Replace(A::serialize_value(__value)?)
                            .into(),
                    )
                }
            }
        }
    }
    #[automatically_derived]
    impl<S, T, U> ::morphix::Observe for Foo<S, T, U>
    where
        Self: ::serde::Serialize,
        T: Clone,
        U: ::morphix::Observe,
    {
        type Observer<'ob, _S, N> = FooObserver<'ob, S, T, U, _S, N>
        where
            Self: 'ob,
            U: 'ob,
            N: ::morphix::helper::Unsigned,
            _S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Qux {}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::builtin::Snapshot for Qux {
    type Snapshot = ();
    #[inline]
    fn to_snapshot(&self) {}
    #[inline]
    fn eq_snapshot(&self, snapshot: &()) -> bool {
        true
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for Qux {
    type Observer<'ob, S, N> = ::morphix::builtin::NoopObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
