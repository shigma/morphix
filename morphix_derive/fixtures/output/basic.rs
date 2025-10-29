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
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct FooObserver<'morphix, __S: ?Sized, __N = Zero> {
        __ptr: ObserverPointer<__S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
        pub a: DefaultObserver<'morphix, i32>,
        pub b: DefaultObserver<'morphix, String>,
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> Default for FooObserver<'morphix, __S, __N>
    where
        DefaultObserver<'morphix, i32>: Default,
        DefaultObserver<'morphix, String>: Default,
    {
        fn default() -> Self {
            Self {
                __ptr: ObserverPointer::default(),
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
        type Target = ObserverPointer<__S>;
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
        type Depth = Succ<Zero>;
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> Observer<'morphix>
    for FooObserver<'morphix, __S, __N>
    where
        Foo: ::serde::Serialize,
        __N: Unsigned,
        __S: AsDerefMut<__N, Target = Foo> + 'morphix,
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
                b: Observer::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> SerializeObserver<'morphix>
    for FooObserver<'morphix, __S, __N>
    where
        i32: Observe,
        DefaultObserver<'morphix, i32>: SerializeObserver<'morphix>,
        String: Observe,
        DefaultObserver<'morphix, String>: SerializeObserver<'morphix>,
        Foo: ::serde::Serialize,
        __S: AsDerefMut<__N, Target = Foo> + 'morphix,
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
            if let Some(mut mutation) = SerializeObserver::collect::<A>(&mut this.b)? {
                mutation.path.push(stringify!(b).into());
                mutations.push(mutation);
            }
            Ok(::morphix::Mutation::coalesce(mutations))
        }
    }
    #[automatically_derived]
    impl Observe for Foo
    where
        Self: ::serde::Serialize,
    {
        type Observer<'morphix, __S, __N> = FooObserver<'morphix, __S, __N>
        where
            Self: 'morphix,
            __N: Unsigned,
            __S: AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
