#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
pub enum Foo {
    A,
    B(),
    C {},
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __initial: FooObserverInitial,
    }
    #[derive(Clone, Copy)]
    pub enum FooObserverInitial {
        A,
        B,
        C,
        __None,
    }
    impl FooObserverInitial {
        fn new(value: &Foo) -> Self {
            match value {
                Foo::A => FooObserverInitial::A,
                Foo::B() => FooObserverInitial::B,
                Foo::C {} => FooObserverInitial::C,
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, S, N> {
        type Target = ::morphix::observe::ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::helper::AsNormalized for FooObserver<'ob, S, N> {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer<'ob> for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::uninit(),
                __phantom: ::std::marker::PhantomData,
                __initial: FooObserverInitial::__None,
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::observe::ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __phantom: ::std::marker::PhantomData,
                __initial: FooObserverInitial::new(__value),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            let __value = this.__ptr.as_deref();
            let __initial = this.__initial;
            this.__initial = FooObserverInitial::new(__value);
            match (__initial, __value) {
                (FooObserverInitial::A, Foo::A)
                | (FooObserverInitial::B, Foo::B())
                | (FooObserverInitial::C, Foo::C {}) => Ok(::morphix::Mutations::new()),
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
    impl ::morphix::Observe for Foo {
        type Observer<'ob, S, N> = FooObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
