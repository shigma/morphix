use ::std::collections::HashMap;
use ::std::fmt::Display;
#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Debug, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Foo {
    r#a: i32,
    #[serde(rename = "bar")]
    b: String,
    #[serde(flatten)]
    c: HashMap<String, i32>,
}
#[rustfmt::skip]
const _: () = {
    pub struct FooObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        r#a: ::morphix::observe::DefaultObserver<'ob, i32>,
        b: ::morphix::observe::DefaultObserver<'ob, String>,
        c: ::morphix::observe::DefaultObserver<'ob, HashMap<String, i32>>,
        __ptr: ::morphix::helper::Pointer<S>,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for FooObserver<'ob, S, N> {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for FooObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            ::std::ptr::from_mut(self).expose_provenance();
            ::morphix::helper::QuasiObserver::invalidate(&mut self.__ptr);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.r#a);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.b);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.c);
            &mut self.__ptr
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::helper::QuasiObserver for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N>,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = N;
        fn invalidate(this: &mut Self) {
            ::morphix::helper::QuasiObserver::invalidate(&mut this.r#a);
            ::morphix::helper::QuasiObserver::invalidate(&mut this.b);
            ::morphix::helper::QuasiObserver::invalidate(&mut this.c);
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                r#a: ::morphix::observe::Observer::uninit(),
                b: ::morphix::observe::Observer::uninit(),
                c: ::morphix::observe::Observer::uninit(),
                __ptr: ::morphix::helper::Pointer::uninit(),
                __phantom: ::std::marker::PhantomData,
            }
        }
        fn observe(head: &mut S) -> Self {
            let __value = head.as_deref_mut();
            let r#a = ::morphix::observe::Observer::observe(&mut __value.r#a);
            let b = ::morphix::observe::Observer::observe(&mut __value.b);
            let c = ::morphix::observe::Observer::observe(&mut __value.c);
            Self {
                r#a,
                b,
                c,
                __ptr: ::morphix::helper::Pointer::new(head),
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn refresh(this: &mut Self, head: &mut S) {
            let __value = head.as_deref_mut();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.r#a, &mut __value.r#a);
                ::morphix::observe::Observer::refresh(&mut this.b, &mut __value.b);
                ::morphix::observe::Observer::refresh(&mut this.c, &mut __value.c);
            }
            ::morphix::helper::Pointer::set(this, head);
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver
    for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let mutations_a = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.r#a)
            };
            let mutations_b = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.b)
            };
            let (mutations_c, is_replace_c) = unsafe {
                ::morphix::observe::SerializeObserver::flat_flush(&mut this.c)
            };
            let is_replace = mutations_a.is_replace() && mutations_b.is_replace()
                && is_replace_c;
            if is_replace {
                let value = ::morphix::helper::QuasiObserver::untracked_ref(&*this);
                return ::morphix::Mutations::replace(value);
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_a.is_empty()) as usize + (!mutations_b.is_empty()) as usize
                    + mutations_c.len(),
            );
            mutations.insert("A", mutations_a);
            mutations.insert("bar", mutations_b);
            mutations.extend(mutations_c);
            mutations
        }
        unsafe fn flat_flush(this: &mut Self) -> (::morphix::Mutations, bool) {
            let mutations_a = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.r#a)
            };
            let mutations_b = unsafe {
                ::morphix::observe::SerializeObserver::flush(&mut this.b)
            };
            let (mutations_c, is_replace_c) = unsafe {
                ::morphix::observe::SerializeObserver::flat_flush(&mut this.c)
            };
            let is_replace = mutations_a.is_replace() && mutations_b.is_replace()
                && is_replace_c;
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_a.is_empty()) as usize + (!mutations_b.is_empty()) as usize
                    + mutations_c.len(),
            );
            mutations.insert("A", mutations_a);
            mutations.insert("bar", mutations_b);
            mutations.extend(mutations_c);
            (mutations, is_replace)
        }
    }
    #[automatically_derived]
    impl ::morphix::Observe for Foo {
        type Observer<'ob, S, N> = FooObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::fmt::Debug for FooObserver<'ob, S, N> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_struct("FooObserver")
                .field("a", &self.r#a)
                .field("b", &self.b)
                .field("c", &self.c)
                .finish()
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::fmt::Display for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let inner = ::morphix::helper::QuasiObserver::untracked_ref(self);
            ::std::fmt::Display::fmt(inner, f)
        }
    }
};
impl Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Foo {{ a: {}, b: {} }}", self.a, self.b)
    }
}
#[rustfmt::skip]
#[derive(Serialize)]
pub struct Bar(i32);
#[rustfmt::skip]
pub struct BarObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero>(
    ::morphix::observe::DefaultObserver<'ob, i32>,
    ::morphix::helper::Pointer<S>,
    ::std::marker::PhantomData<&'ob mut N>,
);
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::std::ops::Deref for BarObserver<'ob, S, N> {
    type Target = ::morphix::helper::Pointer<S>;
    fn deref(&self) -> &Self::Target {
        &self.1
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for BarObserver<'ob, S, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        ::std::ptr::from_mut(self).expose_provenance();
        ::morphix::helper::QuasiObserver::invalidate(&mut self.1);
        ::morphix::helper::QuasiObserver::invalidate(&mut self.0);
        &mut self.1
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::morphix::helper::QuasiObserver for BarObserver<'ob, S, N>
where
    S: ::morphix::helper::AsDeref<N>,
    N: ::morphix::helper::Unsigned,
{
    type Head = S;
    type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
    type InnerDepth = N;
    fn invalidate(this: &mut Self) {
        ::morphix::helper::QuasiObserver::invalidate(&mut this.0);
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::morphix::observe::Observer for BarObserver<'ob, S, N>
where
    S: ::morphix::helper::AsDerefMut<N, Target = Bar>,
    N: ::morphix::helper::Unsigned,
{
    fn uninit() -> Self {
        Self(
            ::morphix::observe::Observer::uninit(),
            ::morphix::helper::Pointer::uninit(),
            ::std::marker::PhantomData,
        )
    }
    fn observe(head: &mut S) -> Self {
        let __value = head.as_deref_mut();
        let observer_0 = ::morphix::observe::Observer::observe(&mut __value.0);
        Self(
            observer_0,
            ::morphix::helper::Pointer::new(head),
            ::std::marker::PhantomData,
        )
    }
    unsafe fn refresh(this: &mut Self, head: &mut S) {
        let __value = head.as_deref_mut();
        unsafe {
            ::morphix::observe::Observer::refresh(&mut this.0, &mut __value.0);
        }
        ::morphix::helper::Pointer::set(this, head);
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver for BarObserver<'ob, S, N>
where
    S: ::morphix::helper::AsDerefMut<N, Target = Bar>,
    N: ::morphix::helper::Unsigned,
{
    unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
        unsafe { ::morphix::observe::SerializeObserver::flush(&mut this.0) }
    }
    unsafe fn flat_flush(this: &mut Self) -> (::morphix::Mutations, bool) {
        unsafe { ::morphix::observe::SerializeObserver::flat_flush(&mut this.0) }
    }
}
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for Bar {
    type Observer<'ob, S, N> = BarObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::DefaultSpec;
}
#[rustfmt::skip]
#[derive(PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Baz(i32, String);
#[rustfmt::skip]
const _: () = {
    pub struct BazObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero>(
        ::morphix::observe::DefaultObserver<'ob, i32>,
        ::morphix::observe::DefaultObserver<'ob, String>,
        ::morphix::helper::Pointer<S>,
        ::std::marker::PhantomData<&'ob mut N>,
    );
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::Deref for BazObserver<'ob, S, N> {
        type Target = ::morphix::helper::Pointer<S>;
        fn deref(&self) -> &Self::Target {
            &self.2
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::ops::DerefMut for BazObserver<'ob, S, N> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            ::std::ptr::from_mut(self).expose_provenance();
            ::morphix::helper::QuasiObserver::invalidate(&mut self.2);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.0);
            ::morphix::helper::QuasiObserver::invalidate(&mut self.1);
            &mut self.2
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::helper::QuasiObserver for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N>,
        N: ::morphix::helper::Unsigned,
    {
        type Head = S;
        type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        type InnerDepth = N;
        fn invalidate(this: &mut Self) {
            ::morphix::helper::QuasiObserver::invalidate(&mut this.0);
            ::morphix::helper::QuasiObserver::invalidate(&mut this.1);
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self(
                ::morphix::observe::Observer::uninit(),
                ::morphix::observe::Observer::uninit(),
                ::morphix::helper::Pointer::uninit(),
                ::std::marker::PhantomData,
            )
        }
        fn observe(head: &mut S) -> Self {
            let __value = head.as_deref_mut();
            let observer_0 = ::morphix::observe::Observer::observe(&mut __value.0);
            let observer_1 = ::morphix::observe::Observer::observe(&mut __value.1);
            Self(
                observer_0,
                observer_1,
                ::morphix::helper::Pointer::new(head),
                ::std::marker::PhantomData,
            )
        }
        unsafe fn refresh(this: &mut Self, head: &mut S) {
            let __value = head.as_deref_mut();
            unsafe {
                ::morphix::observe::Observer::refresh(&mut this.0, &mut __value.0);
                ::morphix::observe::Observer::refresh(&mut this.1, &mut __value.1);
            }
            ::morphix::helper::Pointer::set(this, head);
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver
    for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz>,
        N: ::morphix::helper::Unsigned,
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
                let value = ::morphix::helper::QuasiObserver::untracked_ref(&*this);
                return ::morphix::Mutations::replace(value);
            }
            let mut mutations = ::morphix::Mutations::with_capacity(
                (!mutations_0.is_empty()) as usize + (!mutations_1.is_empty()) as usize,
            );
            mutations.insert(0usize, mutations_0);
            mutations.insert(1usize, mutations_1);
            mutations
        }
        unsafe fn flat_flush(this: &mut Self) -> (::morphix::Mutations, bool) {
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
    impl ::morphix::Observe for Baz {
        type Observer<'ob, S, N> = BazObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::fmt::Debug for BazObserver<'ob, S, N> {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            f.debug_tuple("BazObserver").field(&self.0).field(&self.1).finish()
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::PartialEq for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            let lhs = ::morphix::helper::QuasiObserver::untracked_ref(self);
            let rhs = ::morphix::helper::QuasiObserver::untracked_ref(other);
            lhs.eq(rhs)
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::Eq for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz>,
        N: ::morphix::helper::Unsigned,
    {}
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::PartialOrd for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Self,
        ) -> ::std::option::Option<::std::cmp::Ordering> {
            let lhs = ::morphix::helper::QuasiObserver::untracked_ref(self);
            let rhs = ::morphix::helper::QuasiObserver::untracked_ref(other);
            lhs.partial_cmp(rhs)
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::Ord for BazObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Baz>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
            let lhs = ::morphix::helper::QuasiObserver::untracked_ref(self);
            let rhs = ::morphix::helper::QuasiObserver::untracked_ref(other);
            lhs.cmp(rhs)
        }
    }
};
