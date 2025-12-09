#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(bound = "T: Serialize")]
pub struct Foo<'i, T, const N: usize> {
    #[serde(serialize_with = "serialize_mut_array")]
    a: &'i mut [T; N],
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<
        'ob,
        'i,
        T,
        const N: usize,
        S: ?Sized,
        _N = ::morphix::helper::Zero,
    >
    where
        &'i mut [T; N]: ::morphix::Observe + 'ob,
    {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut _N>,
        pub a: ::morphix::observe::DefaultObserver<'ob, &'i mut [T; N]>,
    }
    #[automatically_derived]
    impl<'ob, 'i, T, const N: usize, S: ?Sized, _N> ::std::ops::Deref
    for FooObserver<'ob, 'i, T, N, S, _N>
    where
        &'i mut [T; N]: ::morphix::Observe,
    {
        type Target = ::morphix::observe::ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, T, const N: usize, S: ?Sized, _N> ::std::ops::DerefMut
    for FooObserver<'ob, 'i, T, N, S, _N>
    where
        &'i mut [T; N]: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'i, T, const N: usize, S: ?Sized, _N> ::morphix::helper::AsNormalized
    for FooObserver<'ob, 'i, T, N, S, _N>
    where
        &'i mut [T; N]: ::morphix::Observe,
    {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, 'i, T, const N: usize, S: ?Sized, _N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, 'i, T, N, S, _N>
    where
        &'i mut [T; N]: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<'i, T, N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = _N;
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: ::morphix::observe::Observer::uninit(),
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::observe::ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: ::morphix::observe::Observer::observe(&mut __value.a),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
            let __value = value.as_deref_mut();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.a, &mut __value.a);
            }
        }
    }
    #[automatically_derived]
    impl<
        'ob,
        'i,
        T,
        const N: usize,
        S: ?Sized,
        _N,
    > ::morphix::observe::SerializeObserver<'ob> for FooObserver<'ob, 'i, T, N, S, _N>
    where
        Foo<'i, T, N>: ::serde::Serialize,
        &'i mut [T; N]: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<'i, T, N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            &'i mut [T; N],
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
            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.a)? {
                mutation.path.push("a".into());
                return Ok(Some(mutation));
            }
            Ok(None)
        }
    }
    #[automatically_derived]
    impl<'i, T, const N: usize> ::morphix::Observe for Foo<'i, T, N>
    where
        Self: ::serde::Serialize,
        &'i mut [T; N]: ::morphix::Observe,
    {
        type Observer<'ob, S, _N> = FooObserver<'ob, 'i, T, N, S, _N>
        where
            Self: 'ob,
            &'i mut [T; N]: 'ob,
            _N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<_N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
fn serialize_mut_array<T, S, const N: usize>(
    a: &&mut [T; N],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: serde::Serializer,
{
    <[_]>::serialize(&**a, serializer)
}
