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
        fn observed_assign<T>(&mut self, value: T)
        where
            Self::Target: ::std::ops::DerefMut<
                Target: ::morphix::helper::AsDerefMut<N, Target = T>,
            >,
        {
            let _ = &mut *self.b;
            *::morphix::helper::AsDerefMut::<
                N,
            >::as_deref_mut(self.as_deref_mut_coinductive().deref_mut()) = value;
        }
    }
    #[automatically_derived]
    impl<'ob, T, O, N> ::morphix::observe::Observer for FooObserver<'ob, O>
    where
        Vec<T>: 'ob,
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDeref<N, Target = Foo<T>>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::observe::Observer::uninit(),
            }
        }
        fn observe(head: &O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(head);
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(head);
            Self {
                a: __inner,
                b: ::morphix::observe::Observer::observe(&__value.b),
            }
        }
        unsafe fn refresh(this: &mut Self, head: &O::Head) {
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(head);
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.a, head);
                ::morphix::observe::Observer::refresh(&mut this.b, &__value.b);
            }
        }
    }
    #[automatically_derived]
    impl<'ob, T, O, N> ::morphix::observe::SerializeObserver for FooObserver<'ob, O>
    where
        Foo<T>: ::morphix::helper::serde::Serialize + 'static,
        Vec<T>: 'ob,
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDeref<N, Target = Foo<T>>,
        N: ::morphix::helper::Unsigned,
        O: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let (mutations_a, is_replace_a) = unsafe {
                ::morphix::observe::SerializeObserver::flush_flatten(&mut this.a)
            };
            let mutations_b = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.b)
            };
            let is_replace = is_replace_a && mutations_b.is_replace();
            if is_replace {
                let head = &**(*this).as_deref_coinductive();
                let value = ::morphix::helper::AsDeref::<N>::as_deref(head);
                return ::morphix::Mutations::replace(value);
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_a.len() + (!mutations_b.is_empty()) as usize,
            );
            mutations.extend(mutations_a);
            mutations.insert("b", mutations_b);
            mutations
        }
        unsafe fn flush_flatten(this: &mut Self) -> (::morphix::Mutations, bool) {
            let (mutations_a, is_replace_a) = unsafe {
                ::morphix::observe::SerializeObserver::flush_flatten(&mut this.a)
            };
            let mutations_b = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.b)
            };
            let is_replace = is_replace_a && mutations_b.is_replace();
            let mut mutations = ::morphix::Mutations::with_capacity(
                mutations_a.len() + (!mutations_b.is_empty()) as usize,
            );
            mutations.extend(mutations_a);
            mutations.insert("b", mutations_b);
            (mutations, is_replace)
        }
    }
    #[automatically_derived]
    impl<T> ::morphix::Observe for Foo<T>
    where
        Self: ::morphix::helper::serde::Serialize,
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
        fn observed_assign<T>(&mut self, value: T)
        where
            Self::Target: ::std::ops::DerefMut<
                Target: ::morphix::helper::AsDerefMut<N, Target = T>,
            >,
        {
            let _ = &mut *self.1;
            *::morphix::helper::AsDerefMut::<
                N,
            >::as_deref_mut(self.as_deref_mut_coinductive().deref_mut()) = value;
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::Observer for BarObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDeref<N, Target = Bar>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self(
                ::morphix::observe::Observer::uninit(),
                ::morphix::observe::Observer::uninit(),
            )
        }
        fn observe(head: &O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(head);
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(head);
            Self(__inner, ::morphix::observe::Observer::observe(&__value.1))
        }
        unsafe fn refresh(this: &mut Self, head: &O::Head) {
            let __value = ::morphix::helper::AsDeref::<N>::as_deref(head);
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.0, head);
                ::morphix::observe::Observer::refresh(&mut this.1, &__value.1);
            }
        }
    }
    #[automatically_derived]
    impl<'ob, O, N> ::morphix::observe::SerializeObserver for BarObserver<'ob, O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDeref<N, Target = Bar>,
        N: ::morphix::helper::Unsigned,
        O: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let mutations_0 = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.0)
            };
            let mutations_1 = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.1)
            };
            let is_replace = mutations_0.is_replace() && mutations_1.is_replace();
            if is_replace {
                let head = &**(*this).as_deref_coinductive();
                let value = ::morphix::helper::AsDeref::<N>::as_deref(head);
                return ::morphix::Mutations::replace(value);
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_0.is_empty()) as usize + (!mutations_1.is_empty()) as usize,
            );
            mutations.insert(0usize, mutations_0);
            mutations.insert(1usize, mutations_1);
            mutations
        }
        unsafe fn flush_flatten(this: &mut Self) -> (::morphix::Mutations, bool) {
            let mutations_0 = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.0)
            };
            let mutations_1 = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.1)
            };
            let is_replace = mutations_0.is_replace() && mutations_1.is_replace();
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_0.is_empty()) as usize + (!mutations_1.is_empty()) as usize,
            );
            mutations.insert(0usize, mutations_0);
            mutations.insert(1usize, mutations_1);
            (mutations, is_replace)
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
        fn observed_assign<T>(&mut self, value: T)
        where
            Self::Target: ::std::ops::DerefMut<
                Target: ::morphix::helper::AsDerefMut<N, Target = T>,
            >,
        {
            *::morphix::helper::AsDerefMut::<
                N,
            >::as_deref_mut(self.as_deref_mut_coinductive().deref_mut()) = value;
        }
    }
    #[automatically_derived]
    impl<O, N> ::morphix::observe::Observer for QuxObserver<O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDeref<N, Target = Qux>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self(::morphix::observe::Observer::uninit())
        }
        fn observe(head: &O::Head) -> Self {
            let __inner = ::morphix::observe::Observer::observe(head);
            Self(__inner)
        }
        unsafe fn refresh(this: &mut Self, head: &O::Head) {
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.0, head);
            }
        }
    }
    #[automatically_derived]
    impl<O, N> ::morphix::observe::SerializeObserver for QuxObserver<O>
    where
        O: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<N>>,
        O::Head: ::morphix::helper::AsDeref<N, Target = Qux>,
        N: ::morphix::helper::Unsigned,
        O: ::morphix::observe::SerializeObserver,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let (mutations_0, is_replace_0) = unsafe {
                ::morphix::observe::SerializeObserver::flush_flatten(&mut this.0)
            };
            let is_replace = is_replace_0;
            if is_replace {
                let head = &**(*this).as_deref_coinductive();
                let value = ::morphix::helper::AsDeref::<N>::as_deref(head);
                return ::morphix::Mutations::replace(value);
            }
            let mut mutations = ::morphix::Mutations::with_capacity(mutations_0.len());
            mutations.extend(mutations_0);
            mutations
        }
        unsafe fn flush_flatten(this: &mut Self) -> (::morphix::Mutations, bool) {
            let (mutations_0, is_replace_0) = unsafe {
                ::morphix::observe::SerializeObserver::flush_flatten(&mut this.0)
            };
            let is_replace = is_replace_0;
            let mut mutations = ::morphix::Mutations::with_capacity(mutations_0.len());
            mutations.extend(mutations_0);
            (mutations, is_replace)
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
