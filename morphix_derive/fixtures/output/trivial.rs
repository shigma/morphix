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
        type Observer<'morphix, __S, __N> = ::morphix::observe::ShallowObserver<
            'morphix,
            __S,
            __N,
        >
        where
            Self: 'morphix,
            __N: ::morphix::helper::Unsigned,
            __S: ::morphix::helper::AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
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
        type Observer<'morphix, __S, __N> = ::morphix::observe::SnapshotObserver<
            'morphix,
            __S,
            __N,
        >
        where
            Self: 'morphix,
            __N: ::morphix::helper::Unsigned,
            __S: ::morphix::helper::AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;
        type Spec = ::morphix::observe::SnapshotSpec;
    }
};
