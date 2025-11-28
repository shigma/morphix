#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[serde(untagged)]
pub enum Foo {
    A(u32, u32),
    B {
        bar: String,
    },
    C,
}
