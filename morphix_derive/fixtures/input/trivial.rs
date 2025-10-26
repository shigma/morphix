use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[observe(shallow)]
struct Foo<T> {
    a: T,
}

#[rustfmt::skip]
#[derive(Serialize, Observe, Clone, PartialEq)]
#[observe(snapshot)]
struct Bar<T> {
    a: T,
}
