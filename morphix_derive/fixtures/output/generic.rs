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
    struct FooObserver<'ob, T, S: ?Sized, N = Zero>
    where
        T: Observe + 'ob,
    {
        __ptr: ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        pub a: DefaultObserver<'ob, T>,
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> Default for FooObserver<'ob, T, S, N>
    where
        T: Observe,
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
    impl<'ob, T, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, T, S, N>
    where
        T: Observe,
    {
        type Target = ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, T, S, N>
    where
        T: Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, T, S> ::morphix::helper::Assignable for FooObserver<'ob, T, S>
    where
        T: Observe,
    {
        type Depth = Succ<Zero>;
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> Observer<'ob> for FooObserver<'ob, T, S, N>
    where
        T: Observe,
        S: AsDerefMut<N, Target = Foo<T>> + 'ob,
        N: Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        type OuterDepth = Zero;
        fn observe(value: &'ob mut S) -> Self {
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
    impl<'ob, T, S: ?Sized, N> SerializeObserver<'ob> for FooObserver<'ob, T, S, N>
    where
        Foo<T>: ::serde::Serialize,
        T: Observe,
        DefaultObserver<'ob, T>: SerializeObserver<'ob>,
        S: AsDerefMut<N, Target = Foo<T>> + 'ob,
        N: Unsigned,
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
        type Observer<'ob, S, N> = FooObserver<'ob, T, S, N>
        where
            T: 'ob,
            Self: 'ob,
            N: Unsigned,
            S: AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
