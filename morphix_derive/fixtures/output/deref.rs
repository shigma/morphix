use std::ops::{Deref, DerefMut};

use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
struct Foo {
    a: Bar,
    b: i32,
}
#[rustfmt::skip]
const _: () = {
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct FooObserver<'morphix, __O>
    where
        i32: Observe + 'morphix,
    {
        __phantom: ::std::marker::PhantomData<&'morphix mut ()>,
        pub a: __O,
        pub b: DefaultObserver<'morphix, i32>,
    }
    #[automatically_derived]
    impl<'morphix, __O> Default for FooObserver<'morphix, __O>
    where
        i32: Observe,
        __O: Default,
        DefaultObserver<'morphix, i32>: Default,
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
    impl<'morphix, __O> ::std::ops::Deref for FooObserver<'morphix, __O>
    where
        i32: Observe,
    {
        type Target = __O;
        fn deref(&self) -> &Self::Target {
            &self.a
        }
    }
    #[automatically_derived]
    impl<'morphix, __O> ::std::ops::DerefMut for FooObserver<'morphix, __O>
    where
        i32: Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.a
        }
    }
    #[automatically_derived]
    impl<'morphix, __O> ::morphix::helper::Assignable for FooObserver<'morphix, __O>
    where
        i32: Observe,
        __O: Observer<'morphix>,
    {
        type Depth = Succ<__O::OuterDepth>;
    }
    #[automatically_derived]
    impl<'morphix, __O, __N> Observer<'morphix> for FooObserver<'morphix, __O>
    where
        i32: Observe,
        __O: Observer<'morphix, InnerDepth = Succ<__N>>,
        __O::Head: AsDerefMut<__N, Target = Foo>,
        __N: Unsigned,
    {
        type Head = __O::Head;
        type InnerDepth = __N;
        type OuterDepth = Succ<__O::OuterDepth>;
        fn observe(value: &'morphix mut __O::Head) -> Self {
            let __inner = Observer::observe(unsafe { &mut *(value as *mut __O::Head) });
            let __value = AsDerefMut::<__N>::as_deref_mut(value);
            Self {
                __phantom: ::std::marker::PhantomData,
                a: __inner,
                b: Observer::observe(&mut __value.b),
            }
        }
    }
    #[automatically_derived]
    impl<'morphix, __O, __N> SerializeObserver<'morphix> for FooObserver<'morphix, __O>
    where
        i32: Observe,
        __O: SerializeObserver<'morphix>,
        DefaultObserver<'morphix, i32>: SerializeObserver<'morphix>,
        __O: Observer<'morphix, InnerDepth = Succ<__N>>,
        __O::Head: AsDerefMut<__N, Target = Foo>,
        __N: Unsigned,
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
    impl Observe for Foo
    where
        i32: Observe,
        Self: ::serde::Serialize,
    {
        type Observer<'morphix, __S, __N> = FooObserver<
            'morphix,
            DefaultObserver<'morphix, Bar, __S, Succ<__N>>,
        >
        where
            i32: 'morphix,
            Self: 'morphix,
            __N: Unsigned,
            __S: AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
impl Deref for Foo {
    type Target = Bar;
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
    a: i32,
}
#[rustfmt::skip]
const _: () = {
    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
    use ::morphix::observe::{
        DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver,
    };
    #[allow(private_interfaces)]
    struct BarObserver<'morphix, __S: ?Sized, __N = Zero>
    where
        i32: Observe + 'morphix,
    {
        __ptr: ObserverPointer<__S>,
        __mutated: bool,
        __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
        pub a: DefaultObserver<'morphix, i32>,
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> Default for BarObserver<'morphix, __S, __N>
    where
        i32: Observe,
        DefaultObserver<'morphix, i32>: Default,
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
    impl<'morphix, __S: ?Sized, __N> ::std::ops::Deref
    for BarObserver<'morphix, __S, __N>
    where
        i32: Observe,
    {
        type Target = ObserverPointer<__S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> ::std::ops::DerefMut
    for BarObserver<'morphix, __S, __N>
    where
        i32: Observe,
    {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'morphix, __S> ::morphix::helper::Assignable for BarObserver<'morphix, __S>
    where
        i32: Observe,
    {
        type Depth = Succ<Zero>;
    }
    #[automatically_derived]
    impl<'morphix, __S: ?Sized, __N> Observer<'morphix>
    for BarObserver<'morphix, __S, __N>
    where
        i32: Observe,
        __S: AsDerefMut<__N, Target = Bar> + 'morphix,
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
    impl<'morphix, __S: ?Sized, __N> SerializeObserver<'morphix>
    for BarObserver<'morphix, __S, __N>
    where
        i32: Observe,
        DefaultObserver<'morphix, i32>: SerializeObserver<'morphix>,
        Bar: ::serde::Serialize,
        __S: AsDerefMut<__N, Target = Bar> + 'morphix,
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
    impl Observe for Bar
    where
        i32: Observe,
        Self: ::serde::Serialize,
    {
        type Observer<'morphix, __S, __N> = BarObserver<'morphix, __S, __N>
        where
            i32: 'morphix,
            Self: 'morphix,
            __N: Unsigned,
            __S: AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
