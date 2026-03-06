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
            ::morphix::helper::QuasiObserver::invalidate(&mut self.__ptr);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.a);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.c);
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
        type Head = _S;
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = _N;
        fn invalidate(this: &mut Self) {
            ::morphix::helper::QuasiObserver::invalidate(&mut this.a);
            ::morphix::helper::QuasiObserver::invalidate(&mut this.c);
        }
    }
    #[automatically_derived]
    impl<'ob, 'a, S, T, U, const N: usize, _S: ?Sized, _N> ::morphix::observe::Observer
    for FooObserver<'ob, 'a, S, T, U, N, _S, _N>
    where
        Option<T>: 'ob,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
        _S: ::morphix::helper::AsDeref<_N, Target = Foo<'a, S, T, U, N>>,
        _N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::helper::Pointer::uninit(),
                c: ::morphix::observe::Observer::uninit(),
                __ptr: ::morphix::helper::Pointer::uninit(),
                __phantom: ::std::marker::PhantomData,
            }
        }
        fn observe(head: &_S) -> Self {
            let __ptr = ::morphix::helper::Pointer::new(head);
            let __value = head.as_deref();
            Self {
                a: ::morphix::observe::Observer::observe(&__value.a),
                b: ::morphix::helper::Pointer::new(&__value.b),
                c: ::morphix::observe::Observer::observe(&__value.c),
                __ptr,
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn refresh(this: &mut Self, head: &_S) {
            ::morphix::helper::Pointer::set(this, head);
            let __value = head.as_deref();
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
        Foo<'a, S, T, U, N>: ::morphix::helper::serde::Serialize + 'static,
        Option<T>: 'ob,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
        _S: ::morphix::helper::AsDeref<_N, Target = Foo<'a, S, T, U, N>>,
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
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let mutations_a = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.a)
            };
            let mut mutations_c = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.c)
            };
            let is_replace = mutations_a.is_replace() && mutations_c.is_replace();
            if is_replace {
                let value = ::morphix::helper::QuasiObserver::untracked_ref(&*this);
                return ::morphix::Mutations::replace(value);
            }
            let __inner = ::morphix::helper::QuasiObserver::untracked_ref(&*this);
            if !mutations_c.is_empty() && Option::is_none(&__inner.c) {
                mutations_c = ::morphix::MutationKind::Delete.into();
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_a.is_empty()) as usize + (!mutations_c.is_empty()) as usize,
            );
            mutations.insert("a", mutations_a);
            mutations.insert("c", mutations_c);
            mutations
        }
        unsafe fn flat_flush(this: &mut Self) -> (::morphix::Mutations, bool) {
            let mutations_a = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.a)
            };
            let mut mutations_c = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.c)
            };
            let is_replace = mutations_a.is_replace() && mutations_c.is_replace();
            let __inner = ::morphix::helper::QuasiObserver::untracked_ref(&*this);
            if !mutations_c.is_empty() && Option::is_none(&__inner.c) {
                mutations_c = ::morphix::MutationKind::Delete.into();
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_a.is_empty()) as usize + (!mutations_c.is_empty()) as usize,
            );
            mutations.insert("a", mutations_a);
            mutations.insert("c", mutations_c);
            (mutations, is_replace)
        }
    }
    #[automatically_derived]
    impl<'a, S, T, U, const N: usize> ::morphix::Observe for Foo<'a, S, T, U, N>
    where
        Self: ::morphix::helper::serde::Serialize,
        &'a mut [S; N]: ::morphix::Observe,
        Option<U>: ::morphix::Observe,
    {
        type Observer<'ob, _S, _N> = FooObserver<'ob, 'a, S, T, U, N, _S, _N>
        where
            Self: 'ob,
            &'a mut [S; N]: 'ob,
            Option<U>: 'ob,
            _N: ::morphix::helper::Unsigned,
            _S: ::morphix::helper::AsDeref<_N, Target = Self> + ?Sized + 'ob;
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
