use morphix_derive::Observe;
use serde::Serialize;
#[rustfmt::skip]
#[derive(Serialize, Observe)]
struct Foo<T> {
    a: T,
}
