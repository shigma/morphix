use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
struct Foo {
    a: i32,
    b: String,
}
#[rustfmt::skip]
const _: () = {
    #[allow(private_interfaces)]
    struct FooObserver<'morphix, __S: ?Sized, __N = ::morphix::helper::Zero> {
        __ptr: ::morphix::observe::ObserverPointer<__S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
        pub a: ::morphix::observe::DefaultObserver<'morphix, i32>,
        pub b: ::morphix::observe::DefaultObserver<'morphix, String>,
    }
    #[automatically_derived]
    impl ::morphix::Observe for Foo
    where
        Self: ::serde::Serialize,
    {
        type Observer<'morphix, __S, __N> = FooObserver<'morphix, __S, __N>
        where
            Self: 'morphix,
            __N: ::morphix::helper::Unsigned,
            __S: ::morphix::helper::AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::DefaultSpec;
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> Default for FooObserver<'morphix, __S, __N>
    where
        ::morphix::observe::DefaultObserver<'morphix, i32>: Default,
        ::morphix::observe::DefaultObserver<'morphix, String>: Default,
    {
        fn default() -> Self {
            Self {
                __ptr: ::morphix::observe::ObserverPointer::default(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: Default::default(),
                b: Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> ::std::ops::Deref
    for FooObserver<'morphix, __S, __N> {
        type Target = ::morphix::observe::ObserverPointer<__S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> ::std::ops::DerefMut
    for FooObserver<'morphix, __S, __N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            self.__mutated = true;
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, __S> ::morphix::helper::Assignable for FooObserver<'morphix, __S> {
        type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> ::morphix::observe::Observer<'morphix>
    for FooObserver<'morphix, __S, __N>
    where
        Foo: ::serde::Serialize,
        __N: ::morphix::helper::Unsigned,
        __S: ::morphix::helper::AsDerefMut<__N, Target = Foo> + 'morphix,
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
                b: ::morphix::observe::ObserveExt::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> ::morphix::observe::SerializeObserver<'morphix>
    for FooObserver<'morphix, __S, __N>
    where
        Foo: ::serde::Serialize,
        __N: ::morphix::helper::Unsigned,
        __S: ::morphix::helper::AsDerefMut<__N, Target = Foo> + 'morphix,
        ::morphix::observe::DefaultObserver<
            'morphix,
            i32,
        >: ::morphix::observe::SerializeObserver<'morphix>,
        ::morphix::observe::DefaultObserver<
            'morphix,
            String,
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
            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                A,
            >(&mut this.b)? {
                mutation.path.push(stringify!(b).into());
                mutations.push(mutation);
            }
            Ok(::morphix::Mutation::coalesce(mutations))
        }
    }
};
