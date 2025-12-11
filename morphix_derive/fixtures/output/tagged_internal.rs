#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Foo<const N: usize> {
    #[serde(serialize_with = "<[_]>::serialize")]
    A([u32; N]),
    C { bar: String, #[serde(flatten)] qux: Qux },
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<
        'ob,
        const N: usize,
        S: ?Sized,
        _N = ::morphix::helper::Zero,
    > {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut _N>,
        __variant: FooObserverVariant<'ob, N>,
    }
    pub enum FooObserverVariant<'ob, const N: usize> {
        A(::morphix::observe::DefaultObserver<'ob, [u32; N]>),
        C {
            bar: ::morphix::observe::DefaultObserver<'ob, String>,
            qux: ::morphix::observe::DefaultObserver<'ob, Qux>,
        },
        __None,
    }
    impl<'ob, const N: usize> FooObserverVariant<'ob, N> {
        fn observe(value: &'ob mut Foo<N>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::observe::Observer::observe(v0)),
                Foo::C { bar, qux } => {
                    Self::C {
                        bar: ::morphix::observe::Observer::observe(bar),
                        qux: ::morphix::observe::Observer::observe(qux),
                    }
                }
            }
        }
        unsafe fn refresh(&mut self, value: &mut Foo<N>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
                    }
                    (Self::C { bar: u0, qux: u1 }, Foo::C { bar: v0, qux: v1 }) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1)
                    }
                    (Self::__None, _) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn flush<A: ::morphix::Adapter>(
            &mut self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        > {
            match self {
                Self::A(u0) => ::morphix::observe::SerializeObserver::flush::<A>(u0),
                Self::C { bar, qux } => {
                    let mut mutations = ::std::vec::Vec::with_capacity(2usize);
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(bar)? {
                        mutation.path.push("bar".into());
                        mutations.push(mutation);
                    }
                    if let Some(mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(qux)? {
                        mutations.push(mutation);
                    }
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
                Self::__None => Ok(None),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::std::ops::Deref
    for FooObserver<'ob, N, S, _N> {
        type Target = ::morphix::observe::ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::std::ops::DerefMut
    for FooObserver<'ob, N, S, _N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            self.__variant = FooObserverVariant::__None;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::helper::AsNormalized
    for FooObserver<'ob, N, S, _N> {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, N, S, _N>
    where
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = _N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: FooObserverVariant::__None,
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::observe::ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: FooObserverVariant::observe(__value),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
            let __value = value.as_deref_mut();
            unsafe { this.__variant.refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, const N: usize, S: ?Sized, _N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, N, S, _N>
    where
        Foo<N>: ::serde::Serialize,
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        > {
            if !this.__mutated {
                return this.__variant.flush::<A>();
            }
            this.__mutated = false;
            this.__variant = FooObserverVariant::__None;
            Ok(
                Some(::morphix::Mutation {
                    path: ::morphix::Path::new(),
                    kind: ::morphix::MutationKind::Replace(
                        A::serialize_value(this.as_deref())?,
                    ),
                }),
            )
        }
    }
    #[automatically_derived]
    impl<const N: usize> ::morphix::Observe for Foo<N>
    where
        Self: ::serde::Serialize,
    {
        type Observer<'ob, S, _N> = FooObserver<'ob, N, S, _N>
        where
            Self: 'ob,
            _N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<_N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Qux {}
#[rustfmt::skip]
const _: () = {
    #[automatically_derived]
    impl ::morphix::Observe for Qux {
        type Observer<'ob, S, N> = ::morphix::observe::NoopObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
