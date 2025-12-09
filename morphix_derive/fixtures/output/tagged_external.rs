#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Foo<S, T>
where
    T: Clone,
{
    A(u32),
    B(u32, S),
    #[serde(rename_all = "UPPERCASE")]
    #[serde(rename = "OwO")]
    C { bar: T, #[serde(rename = "QwQ")] qux: Qux },
    D,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S, T, _S: ?Sized, N = ::morphix::helper::Zero>
    where
        T: Clone,
        S: ::morphix::Observe + 'ob,
        T: ::morphix::Observe + 'ob,
    {
        __ptr: ::morphix::observe::ObserverPointer<_S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        __variant: ::std::mem::MaybeUninit<FooObserverVariant<'ob, S, T>>,
    }
    pub enum FooObserverVariant<'ob, S, T>
    where
        T: Clone,
        S: ::morphix::Observe + 'ob,
        T: ::morphix::Observe + 'ob,
    {
        A(::morphix::observe::DefaultObserver<'ob, u32>),
        B(
            ::morphix::observe::DefaultObserver<'ob, u32>,
            ::morphix::observe::DefaultObserver<'ob, S>,
        ),
        C {
            bar: ::morphix::observe::DefaultObserver<'ob, T>,
            qux: ::morphix::observe::DefaultObserver<'ob, Qux>,
        },
        D,
    }
    impl<'ob, S, T> FooObserverVariant<'ob, S, T>
    where
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
    {
        fn observe(value: &'ob mut Foo<S, T>) -> Self {
            match value {
                Foo::A(v0) => Self::A(::morphix::observe::Observer::observe(v0)),
                Foo::B(v0, v1) => {
                    Self::B(
                        ::morphix::observe::Observer::observe(v0),
                        ::morphix::observe::Observer::observe(v1),
                    )
                }
                Foo::C { bar, qux } => {
                    Self::C {
                        bar: ::morphix::observe::Observer::observe(bar),
                        qux: ::morphix::observe::Observer::observe(qux),
                    }
                }
                Foo::D => Self::D,
            }
        }
        unsafe fn refresh(&mut self, value: &mut Foo<S, T>) {
            unsafe {
                match (self, value) {
                    (Self::A(u0), Foo::A(v0)) => {
                        ::morphix::observe::Observer::refresh(u0, v0)
                    }
                    (Self::B(u0, u1), Foo::B(v0, v1)) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1)
                    }
                    (Self::C { bar: u0, qux: u1 }, Foo::C { bar: v0, qux: v1 }) => {
                        ::morphix::observe::Observer::refresh(u0, v0);
                        ::morphix::observe::Observer::refresh(u1, v1)
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
                S,
            >: ::morphix::observe::SerializeObserver<'ob>,
            ::morphix::observe::DefaultObserver<
                'ob,
                T,
            >: ::morphix::observe::SerializeObserver<'ob>,
        {
            match self {
                Self::A(u0) => {
                    match ::morphix::observe::SerializeObserver::flush::<A>(u0) {
                        Ok(Some(mut mutation)) => {
                            mutation.path.push("a".into());
                            Ok(Some(mutation))
                        }
                        result => result,
                    }
                }
                Self::B(u0, u1) => {
                    let mut mutations = ::std::vec::Vec::with_capacity(2usize);
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u0)? {
                        mutation.path.push("0".into());
                        mutation.path.push("b".into());
                        mutations.push(mutation);
                    }
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(u1)? {
                        mutation.path.push("1".into());
                        mutation.path.push("b".into());
                        mutations.push(mutation);
                    }
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
                Self::C { bar, qux } => {
                    let mut mutations = ::std::vec::Vec::with_capacity(2usize);
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(bar)? {
                        mutation.path.push("BAR".into());
                        mutation.path.push("OwO".into());
                        mutations.push(mutation);
                    }
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                        A,
                    >(qux)? {
                        mutation.path.push("QwQ".into());
                        mutation.path.push("OwO".into());
                        mutations.push(mutation);
                    }
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
                Self::D => Ok(None),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, _S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, S, T, _S, N>
    where
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
    {
        type Target = ::morphix::observe::ObserverPointer<_S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, _S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, S, T, _S, N>
    where
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, _S: ?Sized, N> ::morphix::helper::AsNormalized
    for FooObserver<'ob, S, T, _S, N>
    where
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
    {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S, T, _S: ?Sized, N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, S, T, _S, N>
    where
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<N, Target = Foo<S, T>> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = _S;
        type InnerDepth = N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                __variant: ::std::mem::MaybeUninit::uninit(),
            }
        }
        fn observe(value: &'ob mut _S) -> Self {
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
        unsafe fn refresh(this: &mut Self, value: &mut _S) {
            ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
            let __value = value.as_deref_mut();
            unsafe { this.__variant.assume_init_mut().refresh(__value) }
        }
    }
    #[automatically_derived]
    impl<'ob, S, T, _S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, S, T, _S, N>
    where
        Foo<S, T>: ::serde::Serialize,
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<N, Target = Foo<S, T>> + 'ob,
        N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            S,
        >: ::morphix::observe::SerializeObserver<'ob>,
        ::morphix::observe::DefaultObserver<
            'ob,
            T,
        >: ::morphix::observe::SerializeObserver<'ob>,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
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
    impl<S, T> ::morphix::Observe for Foo<S, T>
    where
        Self: ::serde::Serialize,
        T: Clone,
        S: ::morphix::Observe,
        T: ::morphix::Observe,
    {
        type Observer<'ob, _S, N> = FooObserver<'ob, S, T, _S, N>
        where
            Self: 'ob,
            S: 'ob,
            T: 'ob,
            N: ::morphix::helper::Unsigned,
            _S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
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
