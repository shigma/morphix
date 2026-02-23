use ::std::ops::{Deref, DerefMut};
#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Foo<T> {
    #[serde(flatten)]
    a: Vec<T>,
    b: i32,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, O> {
        a: O,
        b: ::morphix::observe::DefaultObserver<'ob, i32>,
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
    impl<'ob, O, N> ::morphix::helper::QuasiObserver for FooObserver<'ob, O>
    where
        O: ::morphix::helper::QuasiObserver<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Target: ::std::ops::Deref<
            Target: ::morphix::helper::AsDeref<N>
                + ::morphix::helper::AsDeref<::morphix::helper::Succ<N>>,
        >,
        N: ::morphix::helper::Unsigned,
    {
        type OuterDepth = ::morphix::helper::Succ<O::OuterDepth>;
        type InnerDepth = N;
    }
    #[automatically_derived]
    impl<'ob, T, O, N> ::morphix::observe::Observer for FooObserver<'ob, O>
    where
        Vec<T>: 'ob,
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Foo<T>>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::observe::Observer::uninit(),
            }
        }
        fn observe(value: &O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(value);
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(value);
            Self {
                a: __inner,
                b: ::morphix::observe::Observer::observe(&__value.b),
            }
        }
        unsafe fn refresh(this: &mut Self, value: &O::Head) {
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(value);
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.a, value);
                ::morphix::observe::Observer::refresh(&mut this.b, &__value.b);
            }
        }
    }
    #[automatically_derived]
    impl<'ob, T, O, N> ::morphix::observe::SerializeObserver for FooObserver<'ob, O>
    where
        Foo<T>: ::serde::Serialize,
        Vec<T>: 'ob,
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Foo<T>>,
        N: ::morphix::helper::Unsigned,
        O: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            let mutations_a = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.a)?;
            let mutations_b = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.b)?;
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_a.len() + mutations_b.len(),
            );
            mutations.extend(mutations_a);
            mutations.insert("b", mutations_b);
            Ok(mutations)
        }
    }
    #[automatically_derived]
    impl<T> ::morphix::Observe for Foo<T>
    where
        Self: ::serde::Serialize,
        Vec<T>: ::morphix::Observe,
    {
        type Observer<'ob, S, N> = FooObserver<
            'ob,
            ::morphix::observe::DefaultObserver<
                'ob,
                Vec<T>,
                S,
                ::morphix::helper::Succ<N>,
            >,
        >
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
impl<T> Deref for Foo<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Self::Target {
        &self.a
    }
}
impl<T> DerefMut for Foo<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.a
    }
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Bar(Qux, i32);
#[rustfmt::skip]
const _: () = {
    pub struct BarObserver<'ob, O>(O, ::morphix::observe::DefaultObserver<'ob, i32>);
    #[automatically_derived]
    impl<'ob, O> ::std::ops::Deref for BarObserver<'ob, O> {
        type Target = O;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    #[automatically_derived]
    impl<'ob, O> ::std::ops::DerefMut for BarObserver<'ob, O> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::helper::QuasiObserver for BarObserver<'ob, O>
    where
        O: ::morphix::helper::QuasiObserver<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Target: ::std::ops::Deref<
            Target: ::morphix::helper::AsDeref<N>
                + ::morphix::helper::AsDeref<::morphix::helper::Succ<N>>,
        >,
        N: ::morphix::helper::Unsigned,
    {
        type OuterDepth = ::morphix::helper::Succ<O::OuterDepth>;
        type InnerDepth = N;
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::Observer for BarObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Bar>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self(
                ::morphix::observe::Observer::uninit(),
                ::morphix::observe::Observer::uninit(),
            )
        }
        fn observe(value: &O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(value);
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(value);
            Self(__inner, ::morphix::observe::Observer::observe(&__value.1))
        }
        unsafe fn refresh(this: &mut Self, value: &O::Head) {
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(value);
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.0, value);
                ::morphix::observe::Observer::refresh(&mut this.1, &__value.1);
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::SerializeObserver for BarObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Bar>,
        N: ::morphix::helper::Unsigned,
        O: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            let mutations_0 = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.0)?;
            let mutations_1 = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.1)?;
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_0.len() + mutations_1.len(),
            );
            mutations.insert(0usize, mutations_0);
            mutations.insert(1usize, mutations_1);
            Ok(mutations)
        }
    }
    #[automatically_derived]
    impl ::morphix::Observe for Bar
    where
        Qux: ::morphix::Observe,
    {
        type Observer<'ob, S, N> = BarObserver<
            'ob,
            ::morphix::builtin::ShallowObserver<'ob, S, ::morphix::helper::Succ<N>>,
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
        &self.0
    }
}
impl DerefMut for Bar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Qux(pub i32);
#[rustfmt::skip]
const _: () = {
    pub struct QuxObserver<O>(pub O);
    #[automatically_derived]
    impl<O> ::std::ops::Deref for QuxObserver<O> {
        type Target = O;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    #[automatically_derived]
    impl<O> ::std::ops::DerefMut for QuxObserver<O> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    #[automatically_derived]
    impl<O, N> ::morphix::helper::QuasiObserver for QuxObserver<O>
    where
        O: ::morphix::helper::QuasiObserver<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Target: ::std::ops::Deref<
            Target: ::morphix::helper::AsDeref<N>
                + ::morphix::helper::AsDeref<::morphix::helper::Succ<N>>,
        >,
        N: ::morphix::helper::Unsigned,
    {
        type OuterDepth = ::morphix::helper::Succ<O::OuterDepth>;
        type InnerDepth = N;
    }
    #[automatically_derived]
    impl<O, N> ::morphix::observe::Observer for QuxObserver<O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Qux>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self(::morphix::observe::Observer::uninit())
        }
        fn observe(value: &O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(value);
            Self(__inner)
        }
        unsafe fn refresh(this: &mut Self, value: &O::Head) {
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.0, value);
            }
        }
    }
    #[automatically_derived]
    impl<O, N> ::morphix::observe::SerializeObserver for QuxObserver<O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDerefMut<N, Target = Qux>,
        N: ::morphix::helper::Unsigned,
        O: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush_unchecked<A: ::morphix::Adapter>(
            this: &mut Self,
        ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
            let mutations_0 = ::morphix::observe::SerializeObserver::flush::<
                A,
            >(&mut this.0)?;
            let mut mutations = ::morphix::Mutations::with_capacity(mutations_0.len());
            mutations.extend(mutations_0);
            Ok(mutations)
        }
    }
    #[automatically_derived]
    impl ::morphix::Observe for Qux
    where
        i32: ::morphix::Observe,
    {
        type Observer<'ob, S, N> = QuxObserver<
            ::morphix::observe::DefaultObserver<'ob, i32, S, ::morphix::helper::Succ<N>>,
        >
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
impl Deref for Qux {
    type Target = i32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Qux {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
