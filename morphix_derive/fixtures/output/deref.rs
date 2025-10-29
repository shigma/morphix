use std::ops::{Deref, DerefMut};
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
struct Foo {
    a: Qux,
    b: i32,
}
#[rustfmt::skip]
const _: () = {
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct FooObserver<'ob, O> {
        __phantom: ::std::marker::PhantomData<&'ob mut ()>,
        pub a: O,
        pub b: DefaultObserver<'ob, i32>,
    }
    #[automatically_derived]
    impl<'ob, O> Default for FooObserver<'ob, O>
    where
        O: Default,
    {
        fn default() -> Self {
            Self {
                __phantom: ::std::marker::PhantomData,
                a: Default::default(),
                b: Default::default(),
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
        O: Observer<'ob>,
    {
        type Depth = Succ<O::OuterDepth>;
    }
    #[automatically_derived]
    impl<'ob, O, N> Observer<'ob> for FooObserver<'ob, O>
    where
        O: Observer<'ob, InnerDepth = Succ<N>>,
        O::Head: AsDerefMut<N, Target = Foo>,
        N: Unsigned,
    {
        type Head = O::Head;
        type InnerDepth = N;
        type OuterDepth = Succ<O::OuterDepth>;
        fn observe(value: &'ob mut O::Head) -> Self {
            let __inner = Observer::observe(unsafe { &mut *(value as *mut O::Head) });
            let __value = AsDerefMut::<N>::as_deref_mut(value);
            Self {
                __phantom: ::std::marker::PhantomData,
                a: __inner,
                b: Observer::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> SerializeObserver<'ob> for FooObserver<'ob, O>
    where
        O: SerializeObserver<'ob, InnerDepth = Succ<N>>,
        O::Head: AsDerefMut<N, Target = Foo>,
        N: Unsigned,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A>>,
            A::Error,
        > {
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
    impl Observe for Foo {
        type Observer<'ob, S, N> = FooObserver<
            'ob,
            DefaultObserver<'ob, Qux, S, Succ<N>>,
        >
        where
            Self: 'ob,
            N: Unsigned,
            S: AsDerefMut<N, Target = Self> + ?Sized + 'ob;
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
struct Bar {
    a: Qux,
    b: i32,
}
#[rustfmt::skip]
const _: () = {
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct BarObserver<'ob, O> {
        __phantom: ::std::marker::PhantomData<&'ob mut ()>,
        pub a: O,
        pub b: DefaultObserver<'ob, i32>,
    }
    #[automatically_derived]
    impl<'ob, O> Default for BarObserver<'ob, O>
    where
        O: Default,
    {
        fn default() -> Self {
            Self {
                __phantom: ::std::marker::PhantomData,
                a: Default::default(),
                b: Default::default(),
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
        O: Observer<'ob>,
    {
        type Depth = Succ<O::OuterDepth>;
    }
    #[automatically_derived]
    impl<'ob, O, N> Observer<'ob> for BarObserver<'ob, O>
    where
        O: Observer<'ob, InnerDepth = Succ<N>>,
        O::Head: AsDerefMut<N, Target = Bar>,
        N: Unsigned,
    {
        type Head = O::Head;
        type InnerDepth = N;
        type OuterDepth = Succ<O::OuterDepth>;
        fn observe(value: &'ob mut O::Head) -> Self {
            let __inner = Observer::observe(unsafe { &mut *(value as *mut O::Head) });
            let __value = AsDerefMut::<N>::as_deref_mut(value);
            Self {
                __phantom: ::std::marker::PhantomData,
                a: __inner,
                b: Observer::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> SerializeObserver<'ob> for BarObserver<'ob, O>
    where
        O: SerializeObserver<'ob, InnerDepth = Succ<N>>,
        O::Head: AsDerefMut<N, Target = Bar>,
        N: Unsigned,
    {
        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<
            ::std::option::Option<::morphix::Mutation<A>>,
            A::Error,
        > {
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
    impl Observe for Bar {
        type Observer<'ob, S, N> = BarObserver<
            'ob,
            ::morphix::observe::ShallowObserver<'ob, S, Succ<N>>,
        >
        where
            Self: 'ob,
            N: Unsigned,
            S: AsDerefMut<N, Target = Self> + ?Sized + 'ob;
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
struct Qux {
    a: i32,
}
#[rustfmt::skip]
const _: () = {
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct QuxObserver<'ob, S: ?Sized, N = Zero> {
        __ptr: ObserverPointer<S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
        pub a: DefaultObserver<'ob, i32>,
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> Default for QuxObserver<'ob, S, N> {
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
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for QuxObserver<'ob, S, N> {
        type Target = ObserverPointer<S>;
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
        type Depth = Succ<Zero>;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> Observer<'ob> for QuxObserver<'ob, S, N>
    where
        S: AsDerefMut<N, Target = Qux> + 'ob,
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
    impl<'ob, S: ?Sized, N> SerializeObserver<'ob> for QuxObserver<'ob, S, N>
    where
        S: AsDerefMut<N, Target = Qux> + 'ob,
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
    impl Observe for Qux {
        type Observer<'ob, S, N> = QuxObserver<'ob, S, N>
        where
            Self: 'ob,
            N: Unsigned,
            S: AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
