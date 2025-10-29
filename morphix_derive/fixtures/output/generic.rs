use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
struct Foo<T> {
    a: T,
}
#[rustfmt::skip]
const _: () = {
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct FooObserver<'morphix, T, __S: ?Sized, __N = Zero>
    where
        T: Observe + 'morphix,
    {
        __ptr: ObserverPointer<__S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
        pub a: DefaultObserver<'morphix, T>,
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> Default for FooObserver<'morphix, T, __S, __N>
    where
        T: Observe,
        DefaultObserver<'morphix, T>: Default,
    {
        fn default() -> Self {
            Self {
                __ptr: ObserverPointer::default(),
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
        T: Observe,
    {
        type Target = ObserverPointer<__S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> ::std::ops::DerefMut
    for FooObserver<'morphix, T, __S, __N>
    where
        T: Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S> ::morphix::helper::Assignable
    for FooObserver<'morphix, T, __S>
    where
        T: Observe,
    {
        type Depth = Succ<Zero>;
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> Observer<'morphix>
    for FooObserver<'morphix, T, __S, __N>
    where
        T: Observe,
        __S: AsDerefMut<__N, Target = Foo<T>> + 'morphix,
        __N: Unsigned,
    {
        type Head = __S;
        type InnerDepth = __N;
        type OuterDepth = Zero;
        fn observe(value: &'morphix mut __S) -> Self {
            let __ptr = ObserverPointer::new(value);
            let __value = value.as_deref_mut();
            Self {
                __ptr,
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: Observer::observe(&mut __value.a),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, T, __S: ?Sized, __N> SerializeObserver<'morphix>
    for FooObserver<'morphix, T, __S, __N>
    where
        T: Observe,
        DefaultObserver<'morphix, T>: SerializeObserver<'morphix>,
        Foo<T>: ::serde::Serialize,
        __S: AsDerefMut<__N, Target = Foo<T>> + 'morphix,
        __N: Unsigned,
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
            if let Some(mut mutation) = SerializeObserver::collect::<A>(&mut this.a)? {
                mutation.path.push(stringify!(a).into());
                mutations.push(mutation);
            }
            Ok(::morphix::Mutation::coalesce(mutations))
        }
    }
    #[automatically_derived]
    impl<T> Observe for Foo<T>
    where
        T: Observe,
        Self: ::serde::Serialize,
    {
        type Observer<'morphix, __S, __N> = FooObserver<'morphix, T, __S, __N>
        where
            T: 'morphix,
            Self: 'morphix,
            __N: Unsigned,
            __S: AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
