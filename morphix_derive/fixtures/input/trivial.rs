#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[observe(shallow)]
pub struct Foo<T> {
    a: T,
}

#[rustfmt::skip]
#[derive(Serialize, Observe, Clone, PartialEq)]
#[observe(snapshot)]
pub struct Bar<T> {
    a: T,
}
