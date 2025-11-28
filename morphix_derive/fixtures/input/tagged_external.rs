#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub enum Foo<S, T> where T: Clone {
    A(S),
    B {
        bar: T,
    },
    C,
}
