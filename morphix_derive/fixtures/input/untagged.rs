#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[serde(untagged)]
pub enum Foo {
    A(u32),
    B(u32, u32),
    C {
        bar: String,
    },
    D,
}
