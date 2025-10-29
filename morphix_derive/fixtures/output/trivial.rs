use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize)]
struct Foo<T> {
    a: T,
}
#[rustfmt::skip]
const _: () = {
    #[automatically_derived]
    impl<T> ::morphix::Observe for Foo<T> {
        type Observer<'ob, S, N> = ::morphix::observe::ShallowObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::DefaultSpec;
    }
};
#[rustfmt::skip]
#[derive(Serialize, Clone, PartialEq)]
struct Bar<T> {
    a: T,
}
#[rustfmt::skip]
const _: () = {
    #[automatically_derived]
    impl<T> ::morphix::Observe for Bar<T>
    where
        Self: ::std::clone::Clone + ::std::cmp::PartialEq,
    {
        type Observer<'ob, S, N> = ::morphix::observe::SnapshotObserver<'ob, S, N>
        where
            Self: 'ob,
            N: ::morphix::helper::Unsigned,
            S: ::morphix::helper::AsDerefMut<N, Target = Self> + ?Sized + 'ob;
        type Spec = ::morphix::observe::SnapshotSpec;
    }
};
