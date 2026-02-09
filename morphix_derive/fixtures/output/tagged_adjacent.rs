#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(tag = "type", content = "data")]
pub enum Foo<'i> {
    A(u32),
    B(u32, u32),
    C { bar: &'i mut String },
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, 'i, S: ?Sized, N = ::morphix::helper::Zero>
    where
        &'i mut String: ::morphix::Observe + 'ob,
    {
        __ptr: ::morphix::helper::Pointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __variant: FooObserverVariant<'ob, 'i>,
    }
    pub enum FooObserverVariant<'ob, 'i>
    where
        &'i mut String: ::morphix::Observe + 'ob,
    {
        A(::morphix::observe::DefaultObserver<'ob, u32>),
        B(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, u32>,
        ),
        C { bar: ::morphix::observe::DefaultObserver<'ob, &'i mut String> },
        __None,
    }
    impl<'ob, 'i> FooObserverVariant<'ob, 'i>
    where
        &'i mut String: ::morphix::Observe,
    {
        fn observe(value: &'ob mut Foo<'i>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::observe::Observer::observe(v0)),
                Foo::B(v0, v1) => {
                    Self::B(
                        ::morphix::observe::Observer::observe(v0),
                        ::morphix::observe::Observer::observe(v1),
                    )
                }
                Foo::C { bar } => {
                    Self::C {
                        bar: ::morphix::observe::Observer::observe(bar),
                    }
                }
            }
        }
        unsafe fn refresh(&mut self, value: &mut Foo<'i>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
                    }
                    (Self::B(u0, u1), Foo::B(v0, v1)) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1)
                    }
                    (Self::C { bar: u0 }, Foo::C { bar: v0 }) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
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
                &'i mut String,
            >: ::morphix::observe::SerializeObserver<'ob>,
        {
            match self {
                Self::A(u0) => {
                    let mut mutations = ::morphix::Mutations::new();
                    mutations
                        .insert(
                            "data",
                            ::morphix::observe::SerializeObserver::flush::<A>(u0)?,
                        );
                    Ok(mutations)
                }
                Self::B(u0, u1) => {
                    let mut mutations = ::morphix::Mutations::new();
                    mutations
                        .insert2(
                            "data",
                            0usize,
                            ::morphix::observe::SerializeObserver::flush::<A>(u0)?,
                        );
                    mutations
                        .insert2(
                            "data",
                            1usize,
                            ::morphix::observe::SerializeObserver::flush::<A>(u1)?,
                        );
                    Ok(mutations)
                }
                Self::C { bar } => {
                    let mut mutations = ::morphix::Mutations::new();
                    mutations
                        .insert2(
                            "data",
                            "bar",
                            ::morphix::observe::SerializeObserver::flush::<A>(bar)?,
                        );
                    Ok(mutations)
                }
                Self::__None => Ok(::morphix::Mutations::new()),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            self.__variant = FooObserverVariant::__None;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::morphix::helper::AsNormalized
    for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<N, Target = Foo<'i>> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::helper::Pointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: FooObserverVariant::__None,
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: FooObserverVariant::observe(__value),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref_mut();
            unsafe { this.__variant.refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, 'i, S, N>
    where
        Foo<'i>: ::serde::Serialize,
        &'i mut String: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<N, Target = Foo<'i>> + 'ob,
        N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            &'i mut String,
        >: ::morphix::observe::SerializeObserver<'ob>,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            if !this.__mutated {
                return this.__variant.flush::<A>();
            }
            this.__mutated = false;
            this.__variant = FooObserverVariant::__None;
            Ok(
                ::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?)
                    .into(),
            )
        }
    }
    #[automatically_derived]
    impl<'i> ::morphix::Observe for Foo<'i>
    where
        Self: ::serde::Serialize,
        &'i mut String: ::morphix::Observe,
    {
        type Observer<'ob, S, N> = FooObserver<'ob, 'i, S, N>
        where
            Self: 'ob,
            &'i mut String: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
