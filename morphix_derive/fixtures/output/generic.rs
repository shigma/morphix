#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
#[serde(bound = "T: Serialize")]
pub struct Foo<'a, T, U, const N: usize> {
    #[serde(serialize_with = "serialize_mut_array")]
    a: &'a mut [T; N],
    #[serde(skip)]
    pub b: U,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<
        'ob,
        'a,
        T,
        U,
        const N: usize,
        S: ?Sized,
        _N = ::morphix::helper::Zero,
    >
    where
        &'a mut [T; N]: ::morphix::Observe + 'ob,
    {
        a: ::morphix::observe::DefaultObserver<'ob, &'a mut [T; N]>,
        pub b: ::morphix::helper::Pointer<U>,
        __ptr: ::morphix::helper::Pointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut _N>,
    }
    #[automatically_derived]
    impl<'ob, 'a, T, U, const N: usize, S: ?Sized, _N> ::std::ops::Deref
    for FooObserver<'ob, 'a, T, U, N, S, _N>
    where
        &'a mut [T; N]: ::morphix::Observe,
    {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'a, T, U, const N: usize, S: ?Sized, _N> ::std::ops::DerefMut
    for FooObserver<'ob, 'a, T, U, N, S, _N>
    where
        &'a mut [T; N]: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, 'a, T, U, const N: usize, S: ?Sized, _N> ::morphix::helper::AsNormalized
    for FooObserver<'ob, 'a, T, U, N, S, _N>
    where
        &'a mut [T; N]: ::morphix::Observe,
    {
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, 'a, T, U, const N: usize, S: ?Sized, _N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, 'a, T, U, N, S, _N>
    where
        U: 'ob,
        &'a mut [T; N]: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<'a, T, U, N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = _N;
        fn uninit() -> Self {
            Self {
                a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::helper::Pointer::uninit(),
                __ptr: ::morphix::helper::Pointer::uninit(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            }
        }
        fn observe(value: &'ob mut S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                a: ::morphix::observe::Observer::observe(&mut __value.a),
                b: ::morphix::helper::Pointer::new(&mut __value.b),
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn refresh(this: &mut Self, value: &mut S) {
            ::morphix::helper::Pointer::set(this, value);
            let __value = value.as_deref_mut();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.a, &mut __value.a);
                ::morphix::helper::Pointer::set(&this.b, &mut __value.b);
            }
        }
    }
    #[automatically_derived]
    impl<
        'ob,
        'a,
        T,
        U,
        const N: usize,
        S: ?Sized,
        _N,
    > ::morphix::observe::SerializeObserver<'ob> for FooObserver<'ob, 'a, T, U, N, S, _N>
    where
        Foo<'a, T, U, N>: ::serde::Serialize,
        U: 'ob,
        &'a mut [T; N]: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<_N, Target = Foo<'a, T, U, N>> + 'ob,
        _N: ::morphix::helper::Unsigned,
        ::morphix::observe::DefaultObserver<
            'ob,
            &'a mut [T; N],
        >: ::morphix::observe::SerializeObserver<'ob>,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            if this.__mutated {
                this.__mutated = false;
                return Ok(
                    ::morphix::MutationKind::Replace(
                            A::serialize_value(this.as_deref())?,
                        )
                        .into(),
                );
            }
            let mutations_a = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.a)?;
            let mut mutations = ::morphix::Mutations::with_capacity(mutations_a.len());
            mutations.insert("a", mutations_a);
            Ok(mutations)
        }
    }
    #[automatically_derived]
    impl<'a, T, U, const N: usize> ::morphix::Observe for Foo<'a, T, U, N>
    where
        Self: ::serde::Serialize,
        &'a mut [T; N]: ::morphix::Observe,
    {
        type Observer<'ob, S, _N> = FooObserver<'ob, 'a, T, U, N, S, _N>
        where
            Self: 'ob,
            &'a mut [T; N]: 'ob,
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
