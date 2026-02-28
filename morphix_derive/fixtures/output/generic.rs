#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(bound = "S: Serialize, U: Serialize")]
pub struct Foo<'a, S, T, U, const N: usize> {
    #[serde(serialize_with = "serialize_mut_array")]
    a: &'a mut [S; N],
    #[serde(skip)]
    pub b: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<U>,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<
        'ob,
        'a,
        S,
        T,
        U,
        const N: usize,
        _S: ?Sized,
        _N = ::morphix::helper::Zero,
    >
    where
        &'a mut [S; N]: ::morphix::Observe + 'ob,
        Option<U>: ::morphix::Observe + 'ob,
    {
        a: ::morphix::observe::DefaultObserver<'ob, &'a mut [S; N]>,
        pub b: ::morphix::helper::Pointer<Option<T>>,
        pub c: ::morphix::observe::DefaultObserver<'ob, Option<U>>,
        __ptr: ::morphix::helper::Pointer<_S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut _N>,
    }
    #[automatically_derived]
    impl<'ob, 'a, S, T, U, const N: usize, _S: ?Sized, _N> ::std::ops::Deref
    for FooObserver<'ob, 'a, S, T, U, N, _S, _N>
    where
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
    {
        type Target = ::morphix::helper::Pointer<_S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'a, S, T, U, const N: usize, _S: ?Sized, _N> ::std::ops::DerefMut
    for FooObserver<'ob, 'a, S, T, U, N, _S, _N>
    where
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<
        'ob,
        'a,
        S,
        T,
        U,
        const N: usize,
        _S: ?Sized,
        _N,
    > ::morphix::helper::QuasiObserver for FooObserver<'ob, 'a, S, T, U, N, _S, _N>
    where
        _S: ::morphix::helper::AsDeref<_N>,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
        _N: ::morphix::helper::Unsigned,
    {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = _N;
    }
    #[automatically_derived]
    impl<'ob, 'a, S, T, U, const N: usize, _S: ?Sized, _N> ::morphix::observe::Observer
    for FooObserver<'ob, 'a, S, T, U, N, _S, _N>
    where
        Option<T>: 'ob,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<_N, Target = Foo<'a, S, T, U, N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::helper::Pointer::uninit(),
                c: ::morphix::observe::Observer::uninit(),
                __ptr: ::morphix::helper::Pointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            }
        }
        fn observe(value: &_S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref();
            Self {
                a: ::morphix::observe::Observer::observe(&__value.a),
                b: ::morphix::helper::Pointer::new(&__value.b),
                c: ::morphix::observe::Observer::observe(&__value.c),
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn refresh(this: &mut Self, value: &_S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.a, &__value.a);
                ::morphix::helper::Pointer::set(&this.b, &__value.b);
                ::morphix::observe::Observer::refresh(&mut this.c, &__value.c);
            }
        }
    }
    #[automatically_derived]
    impl<
        'ob,
        'a,
        S,
        T,
        U,
        const N: usize,
        _S: ?Sized,
        _N,
    > ::morphix::observe::SerializeObserver for FooObserver<'ob, 'a, S, T, U, N, _S, _N>
    where
        Foo<'a, S, T, U, N>: ::serde::Serialize,
        Option<T>: 'ob,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
        _S: ::morphix::helper::AsDerefMut<_N, Target = Foo<'a, S, T, U, N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            &'a mut [S; N],
        >: ::morphix::observe::SerializeObserver,
        ::morphix::observe::DefaultObserver<
            'ob,
            Option<U>,
        >: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            if this.__mutated {
                this.__mutated = false;
                let __inner = ::morphix::observe::ObserverExt::inner_ref(&*this);
                return Ok(
                    ::morphix::MutationKind::Replace(A::serialize_value(__inner)?).into(),
                );
            }
            let mutations_a = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.a)?;
            let mut mutations_c = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.c)?;
            let __inner = ::morphix::observe::ObserverExt::inner_ref(&*this);
            if !mutations_c.is_empty() && Option::is_none(&__inner.c) {
                mutations_c = ::morphix::MutationKind::Delete.into();
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_a.len() + mutations_c.len(),
            );
            mutations.insert("a", mutations_a);
            mutations.insert("c", mutations_c);
            Ok(mutations)
        }
    }
    #[automatically_derived]
    impl<'a, S, T, U, const N: usize> ::morphix::Observe for Foo<'a, S, T, U, N>
    where
        Self: ::serde::Serialize,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
    {
        type Observer<'ob, _S, _N> = FooObserver<'ob, 'a, S, T, U, N, _S, _N>
        where
            Self: 'ob,
            &'a mut [S; N]: 'ob,
            Option<U>: 'ob,
            _N: ::morphix::helper::Unsigned,
            _S: ::morphix::helper::AsDerefMut<_N, Target = Self> + ?Sized + 'ob;
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
