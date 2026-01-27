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
    #[automatically_derived]
    impl<T> ::morphix::Observe for Foo<T> {
        type Observer<'ob, S, N> = ::morphix::builtin::ShallowObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize, Clone, PartialEq)]
pub struct Bar<T> {
    a: T,
}
#[rustfmt::skip]
const _: () = {
    #[automatically_derived]
    impl<T> ::morphix::Observe for Bar<T>
    where
        Self: ::std::clone::Clone + ::std::cmp::PartialEq,
    {
        type Observer<'ob, S, N> = ::morphix::builtin::SnapshotObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::SnapshotSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize)]
pub struct NoopStruct {}
#[rustfmt::skip]
const _: () = {
    #[automatically_derived]
    impl ::morphix::Observe for NoopStruct {
        type Observer<'ob, S, N> = ::morphix::builtin::NoopObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
