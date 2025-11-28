#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(untagged)]
pub enum Foo {
    A(u32, u32),
    B { bar: String },
    C,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __variant: ::std::mem::MaybeUninit<FooObserverVariant<'ob>>,
    }
    pub enum FooObserverVariant<'ob> {
        A(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, u32>,
        ),
        B { bar: ::morphix::observe::DefaultObserver<'ob, String> },
        C,
    }
    impl<'ob> FooObserverVariant<'ob> {
        fn observe(value: &'ob mut Foo) -> Self {
            match value {
                Foo::A(v0, v1) => {
                    Self::A(
                        ::morphix::observe::Observer::observe(v0),
                        ::morphix::observe::Observer::observe(v1),
                    )
                }
                Foo::B { bar } => {
                    Self::B {
                        bar: ::morphix::observe::Observer::observe(bar),
                    }
                }
                Foo::C => Self::C,
            }
        }
        unsafe fn refresh(&mut self, value: &mut Foo) {
            unsafe {
                match (self, value) {
                    (Self::A(u0, u1), Foo::A(v0, v1)) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1)
                    }
                    (Self::B { bar: u0 }, Foo::B { bar: v0 }) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
                    }
                    (Self::C, Foo::C) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn collect<A: ::morphix::Adapter>(
            &mut self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        > {
            match self {
                Self::A(u0, u1) => {
                    let mut mutations = ::std::vec::Vec::with_capacity(2usize);
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                        A,
                    >(u0)? {
                        mutation.path.push("0".into());
                        mutations.push(mutation);
                    }
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                        A,
                    >(u1)? {
                        mutation.path.push("1".into());
                        mutations.push(mutation);
                    }
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
                Self::B { bar } => {
                    ::morphix::observe::SerializeObserver::collect::<A>(bar)
                }
                Self::C => Ok(None),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::default::Default for FooObserver<'ob, S, N> {
        fn default() -> Self {
            Self {
                __ptr: ::std::default::Default::default(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: ::std::mem::MaybeUninit::uninit(),
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
    impl<'ob, S> ::morphix::helper::Assignable for FooObserver<'ob, S> {
        type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer<'ob> for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        type OuterDepth = ::morphix::helper::Zero;
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::observe::ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: ::std::mem::MaybeUninit::new(
                    FooObserverVariant::observe(__value),
                ),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
            let __value = value.as_deref_mut();
            unsafe { this.__variant.assume_init_mut().refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        > {
            if this.__mutated {
                return Ok(
                    Some(::morphix::Mutation {
                        path: ::morphix::Path::new(),
                        kind: ::morphix::MutationKind::Replace(
                            A::serialize_value(this.as_deref())?,
                        ),
                    }),
                );
            }
            unsafe { this.__variant.assume_init_mut() }.collect::<A>()
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
