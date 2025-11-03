use ::std::ops::{Deref, DerefMut};
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Foo {
    a: Qux,
    b: i32,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, O> {
        __phantom: ::std::marker::PhantomData<&'ob mut ()>,
        pub a: O,
        pub b: ::morphix::observe::DefaultObserver<'ob, i32>,
    }
    #[automatically_derived]
    impl<'ob, O> ::std::default::Default for FooObserver<'ob, O>
    where
        O: ::std::default::Default,
    {
        fn default() -> Self {
            Self {
                __phantom: ::std::marker::PhantomData,
                a: ::std::default::Default::default(),
                b: ::std::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::std::ops::Deref for FooObserver<'ob, O> {
        type Target = O;
        fn deref(&self) -> &Self::Target {
            &self.a
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::std::ops::DerefMut for FooObserver<'ob, O> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.a
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::morphix::helper::Assignable for FooObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<'ob>,
    {
        type Depth = ::morphix::helper::Succ<O::OuterDepth>;
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::Observer<'ob> for FooObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<'ob, InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        type Head = O::Head;
        type InnerDepth = N;
        type OuterDepth = ::morphix::helper::Succ<O::OuterDepth>;
        fn observe(value: &'ob mut O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(unsafe {
                &mut *(value as *mut O::Head)
            });
            let __value = ::morphix::helper::AsDerefMut::<N>::as_deref_mut(value);
            Self {
                __phantom: ::std::marker::PhantomData,
                a: __inner,
                b: ::morphix::observe::Observer::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::SerializeObserver<'ob> for FooObserver<'ob, O>
    where
        O: ::morphix::observe::SerializeObserver<
            'ob,
            InnerDepth = ::morphix::helper::Succ<N>,
        >,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A>>,
            A::Error,
        > {
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
    #[automatically_derived]
    impl ::morphix::Observe for Foo {
        type Observer<'ob, S, N> = FooObserver<
            'ob,
            ::morphix::observe::DefaultObserver<'ob, Qux, S, ::morphix::helper::Succ<N>>,
        >
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
impl Deref for Foo {
    type Target = Qux;
    fn deref(&self) -> &Self::Target {
        &self.a
    }
}
impl DerefMut for Foo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.a
    }
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Bar {
    a: Qux,
    b: i32,
}
#[rustfmt::skip]
const _: () = {
    pub struct BarObserver<'ob, O> {
        __phantom: ::std::marker::PhantomData<&'ob mut ()>,
        pub a: O,
        pub b: ::morphix::observe::DefaultObserver<'ob, i32>,
    }
    #[automatically_derived]
    impl<'ob, O> ::std::default::Default for BarObserver<'ob, O>
    where
        O: ::std::default::Default,
    {
        fn default() -> Self {
            Self {
                __phantom: ::std::marker::PhantomData,
                a: ::std::default::Default::default(),
                b: ::std::default::Default::default(),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::std::ops::Deref for BarObserver<'ob, O> {
        type Target = O;
        fn deref(&self) -> &Self::Target {
            &self.a
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::std::ops::DerefMut for BarObserver<'ob, O> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.a
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::morphix::helper::Assignable for BarObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<'ob>,
    {
        type Depth = ::morphix::helper::Succ<O::OuterDepth>;
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::Observer<'ob> for BarObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<'ob, InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Bar>,
        N: ::morphix::helper::Unsigned,
    {
        type Head = O::Head;
        type InnerDepth = N;
        type OuterDepth = ::morphix::helper::Succ<O::OuterDepth>;
        fn observe(value: &'ob mut O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(unsafe {
                &mut *(value as *mut O::Head)
            });
            let __value = ::morphix::helper::AsDerefMut::<N>::as_deref_mut(value);
            Self {
                __phantom: ::std::marker::PhantomData,
                a: __inner,
                b: ::morphix::observe::Observer::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::SerializeObserver<'ob> for BarObserver<'ob, O>
    where
        O: ::morphix::observe::SerializeObserver<
            'ob,
            InnerDepth = ::morphix::helper::Succ<N>,
        >,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Bar>,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A>>,
            A::Error,
        > {
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
    #[automatically_derived]
    impl ::morphix::Observe for Bar {
        type Observer<'ob, S, N> = BarObserver<
            'ob,
            ::morphix::observe::ShallowObserver<'ob, S, ::morphix::helper::Succ<N>>,
        >
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
impl Deref for Bar {
    type Target = Qux;
    fn deref(&self) -> &Self::Target {
        &self.a
    }
}
impl DerefMut for Bar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.a
    }
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Qux {
    a: i32,
}
#[rustfmt::skip]
const _: () = {
    pub struct QuxObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        __ptr: ::morphix::observe::ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        pub a: ::morphix::observe::DefaultObserver<'ob, i32>,
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::default::Default for QuxObserver<'ob, S, N> {
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
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for QuxObserver<'ob, S, N> {
        type Target = ::morphix::observe::ObserverPointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for QuxObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S> ::morphix::helper::Assignable for QuxObserver<'ob, S> {
        type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer<'ob> for QuxObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Qux> + 'ob,
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
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver<'ob>
    for QuxObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Qux> + 'ob,
        N: ::morphix::helper::Unsigned,
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
    #[automatically_derived]
    impl ::morphix::Observe for Qux {
        type Observer<'ob, S, N> = QuxObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
