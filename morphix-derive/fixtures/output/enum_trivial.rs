#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum Foo {
    A,
    B(),
    C {},
}
#[rustfmt::skip]
const _: () = {
    #[::std::prelude::v1::derive()]
    pub struct FooObserver<'ob, S: ?Sized, N = ::morphix::helper::Zero> {
        __ptr: ::morphix::helper::Pointer<S>,
        __initial: FooObserverInitial,
        __phantom: ::std::marker::PhantomData<&'ob mut N>,
    }
    #[derive(Clone, Copy)]
    pub enum FooObserverInitial {
        A,
        B,
        C,
        __None,
    }
    impl FooObserverInitial {
        fn new(value: &Foo) -> Self {
            match value {
                Foo::A => FooObserverInitial::A,
                Foo::B() => FooObserverInitial::B,
                Foo::C {} => FooObserverInitial::C,
            }
        }
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
        fn invalidate(this: &mut Self) {}
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::Observer for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDerefMut<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        fn uninit() -> Self {
            Self {
                __ptr: ::morphix::helper::Pointer::uninit(),
                __phantom: ::std::marker::PhantomData,
                __initial: FooObserverInitial::__None,
            }
        }
        fn observe(head: &mut S) -> Self {
            let __value = head.as_deref_mut();
            Self {
                __initial: FooObserverInitial::new(__value),
                __ptr: ::morphix::helper::Pointer::from(head),
                __phantom: ::std::marker::PhantomData,
            }
        }
        unsafe fn refresh(this: &mut Self, head: &mut S) {
            ::morphix::helper::Pointer::set(this, &mut *head);
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::morphix::observe::SerializeObserver
    for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        unsafe fn flush(this: &mut Self) -> ::morphix::Mutations {
            let __value = this.__ptr.as_deref();
            let __initial = this.__initial;
            this.__initial = FooObserverInitial::new(__value);
            match (__initial, __value) {
                (FooObserverInitial::A, Foo::A)
                | (FooObserverInitial::B, Foo::B())
                | (FooObserverInitial::C, Foo::C {}) => ::morphix::Mutations::new(),
                _ => ::morphix::Mutations::replace(__value),
            }
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
    impl<'ob, S: ?Sized, N> ::std::cmp::PartialEq for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn eq(&self, other: &Self) -> bool {
            self.as_deref().eq(other.as_deref())
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::Eq for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {}
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::PartialOrd for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn partial_cmp(
            &self,
            other: &Self,
        ) -> ::std::option::Option<::std::cmp::Ordering> {
            self.as_deref().partial_cmp(other.as_deref())
        }
    }
    #[automatically_derived]
    impl<'ob, S: ?Sized, N> ::std::cmp::Ord for FooObserver<'ob, S, N>
    where
        S: ::morphix::helper::AsDeref<N, Target = Foo>,
        N: ::morphix::helper::Unsigned,
    {
        #[inline]
        fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
            self.as_deref().cmp(other.as_deref())
        }
    }
};
#[rustfmt::skip]
#[derive(Serialize)]
pub enum Bar {
    A,
    B(),
    C {},
}
#[rustfmt::skip]
const _: () = {
    pub enum BarSnapshot {
        A,
        B(),
        C {},
    }
    #[automatically_derived]
    impl ::morphix::builtin::Snapshot for Bar {
        type Snapshot = BarSnapshot;
        #[inline]
        fn to_snapshot(&self) -> Self::Snapshot {
            match self {
                Self::A => BarSnapshot::A,
                Self::B() => BarSnapshot::B(),
                Self::C {} => BarSnapshot::C {},
            }
        }
        #[inline]
        #[allow(clippy::match_like_matches_macro)]
        fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
            match (self, snapshot) {
                (Self::A, BarSnapshot::A) => true,
                (Self::B(), BarSnapshot::B()) => true,
                (Self::C {}, BarSnapshot::C {}) => true,
                _ => false,
            }
        }
    }
};
#[rustfmt::skip]
#[automatically_derived]
impl ::morphix::Observe for Bar
where
    Self: ::morphix::builtin::Snapshot,
{
    type Observer<'ob, S, N> = ::morphix::builtin::SnapshotObserver<'ob, S, N>
    where
        Self: 'ob,
        N: ::morphix::helper::Unsigned,
        S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
    type Spec = ::morphix::observe::SnapshotSpec;
}
