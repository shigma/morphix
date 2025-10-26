use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
struct Foo<T> {
    a: T,
}
#[rustfmt::skip]
const _: () = {
    #[allow(private_interfaces)]
    struct FooObserver<'morphix, T, __S: ?Sized, __N>
    where
        T: ::morphix::Observe + 'morphix,
    {
        __ptr: ::morphix::observe::ObserverPointer<__S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
        pub a: ::morphix::helper::DefaultObserver<'morphix, T>,
    }
    #[automatically_derived]
    impl<T> ::morphix::Observe for Foo<T>
    where
        T: ::morphix::Observe,
        Self: ::serde::Serialize,
    {
        type Observer<'morphix, __S, __N> = FooObserver<'morphix, T, __S, __N>
        where
            T: 'morphix,
            Self: 'morphix,
            __N: ::morphix::helper::Unsigned,
            __S: ::morphix::helper::AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::DefaultSpec;
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> Default for FooObserver<'morphix, T, __S, __N>
    where
        T: ::morphix::Observe,
        ::morphix::helper::DefaultObserver<'morphix, T>: Default,
    {
        fn default() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::default(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> ::std::ops::Deref
    for FooObserver<'morphix, T, __S, __N>
    where
        T: ::morphix::Observe,
    {
        type Target = ::morphix::observe::ObserverPointer<__S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> ::std::ops::DerefMut
    for FooObserver<'morphix, T, __S, __N>
    where
        T: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> ::morphix::helper::Assignable
    for FooObserver<'morphix, T, __S, __N>
    where
        T: ::morphix::Observe,
    {
        type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> ::morphix::observe::Observer<'morphix>
    for FooObserver<'morphix, T, __S, __N>
    where
        T: ::morphix::Observe,
        Foo<T>: ::serde::Serialize,
        __N: ::morphix::helper::Unsigned,
        __S: ::morphix::helper::AsDerefMut<__N, Target = Foo<T>> + 'morphix,
    {
        type Head = __S;
        type UpperDepth = __N;
        type LowerDepth = ::morphix::helper::Zero;
        fn observe(value: &'morphix mut __S) -> Self {
            let __ptr = ::morphix::observe::ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: ::morphix::observe::ObserveExt::observe(&mut __value.a),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> ::morphix::observe::SerializeObserver<'morphix>
    for FooObserver<'morphix, T, __S, __N>
    where
        T: ::morphix::Observe,
        Foo<T>: ::serde::Serialize,
        __N: ::morphix::helper::Unsigned,
        __S: ::morphix::helper::AsDerefMut<__N, Target = Foo<T>> + 'morphix,
        ::morphix::helper::DefaultObserver<
            'morphix,
            T,
        >: ::morphix::observe::SerializeObserver<'morphix>,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A>>,
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
            let mut mutations = ::std::vec::Vec::new();
            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                A,
            >(&mut this.a)? {
                mutation.path.push(stringify!(a).into());
                mutations.push(mutation);
            }
            Ok(::morphix::Mutation::coalesce(mutations))
        }
    }
};
