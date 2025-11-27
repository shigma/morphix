#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Foo<T> {
    a: T,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, T, S: ?Sized, N = ::morphix::helper::Zero>
    where
        T: ::morphix::Observe + 'ob,
    {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        pub a: ::morphix::observe::DefaultObserver<'ob, T>,
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> ::std::default::Default for FooObserver<'ob, T, S, N>
    where
        T: ::morphix::Observe,
    {
        fn default() -> Self {
            Self {
                __ptr: ::std::default::Default::default(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
                a: ::std::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, T, S, N>
    where
        T: ::morphix::Observe,
    {
        type Target = ::morphix::observe::ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, T, S, N>
    where
        T: ::morphix::Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, T, S> ::morphix::helper::Assignable for FooObserver<'ob, T, S>
    where
        T: ::morphix::Observe,
    {
        type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> ::morphix::observe::Observer<'ob>
    for FooObserver<'ob, T, S, N>
    where
        T: ::morphix::Observe,
        S: ::morphix::helper::AsDerefMut<N, Target = Foo<T>> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type InnerDepth = N;
        type OuterDepth = ::morphix::helper::Zero;
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
    impl<'ob, T, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for FooObserver<'ob, T, S, N>
    where
        Foo<T>: ::serde::Serialize,
        T: ::morphix::Observe,
        ::morphix::observe::DefaultObserver<
            'ob,
            T,
        >: ::morphix::observe::SerializeObserver<'ob>,
        S: ::morphix::helper::AsDerefMut<N, Target = Foo<T>> + 'ob,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
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
            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<
                A,
            >(&mut this.a)? {
                mutation.path.push("a".into());
                return Ok(Some(mutation));
            }
            Ok(None)
        }
    }
    #[automatically_derived]
    impl<'ob, T, S: ?Sized, N> ::std::fmt::Debug for FooObserver<'ob, T, S, N>
    where
        T: ::morphix::Observe,
        ::morphix::observe::DefaultObserver<'ob, T>: ::std::fmt::Debug,
    {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("FooObserver").field("a", &self.a).finish()
        }
    }
    #[automatically_derived]
    impl<T> ::morphix::Observe for Foo<T>
    where
        T: ::morphix::Observe,
        Self: ::serde::Serialize,
    {
        type Observer<'ob, S, N> = FooObserver<'ob, T, S, N>
        where
            T: 'ob,
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
