#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[morphix(shallow)]
pub struct Foo<T> {
    a: T,
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[morphix(snapshot)]
pub struct Bar<T> {
    a: Vec<T>,
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub struct NoopStruct {}
