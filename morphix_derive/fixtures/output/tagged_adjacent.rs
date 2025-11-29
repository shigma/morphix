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
    D,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, 'i, S: ?Sized, N = ::morphix::helper::Zero>
    where
        &'i mut String: ::morphix::Observe + 'ob,
    {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __variant: ::std::mem::MaybeUninit<FooObserverVariant<'ob, 'i>>,
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
        D,
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
                Foo::D => Self::D,
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
                    (Self::D, Foo::D) => {}
                    _ => panic!("inconsistent state for FooObserver"),
                }
            }
        }
        fn collect<A: ::morphix::Adapter>(
            &mut self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A::Value>>,
            A::Error,
        >
        where
            ::morphix::observe::DefaultObserver<
                'ob,
                &'i mut String,
            >: ::morphix::observe::SerializeObserver<'ob>,
        {
            match self {
                Self::A(u0) => {
                    match ::morphix::observe::SerializeObserver::collect::<A>(u0) {
                        Ok(Some(mut mutation)) => {
                            mutation.path.push("data".into());
                            Ok(Some(mutation))
                        }
                        result => result,
                    }
                }
                Self::B(u0, u1) => {
                    let mut mutations = ::std::vec::Vec::with_capacity(2usize);
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                        A,
                    >(u0)? {
                        mutation.path.push("0".into());
                        mutation.path.push("data".into());
                        mutations.push(mutation);
                    }
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                        A,
                    >(u1)? {
                        mutation.path.push("1".into());
                        mutation.path.push("data".into());
                        mutations.push(mutation);
                    }
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
                Self::C { bar } => {
                    match ::morphix::observe::SerializeObserver::collect::<A>(bar) {
                        Ok(Some(mut mutation)) => {
                            mutation.path.push("bar".into());
                            mutation.path.push("data".into());
                            Ok(Some(mutation))
                        }
                        result => result,
                    }
                }
                Self::D => Ok(None),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S: ?Sized, N> ::std::default::Default for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
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
    impl<'ob, 'i, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, 'i, S, N>
    where
        &'i mut String: ::morphix::Observe,
    {
        type Target = ::morphix::observe::ObserverPointer<S>;
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
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, S> ::morphix::helper::Assignable for FooObserver<'ob, 'i, S>
    where
        &'i mut String: ::morphix::Observe,
    {
        type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
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
