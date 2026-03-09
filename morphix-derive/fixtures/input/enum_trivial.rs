#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(PartialEq, Eq, PartialOrd, Ord, Serialize, Observe)]
#[morphix(derive(PartialEq, Eq, PartialOrd, Ord))]
pub enum Foo {
    A,
    B(),
    C {},
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[morphix(snapshot)]
pub enum Bar {
    A,
    B(),
    C {},
}
