#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[serde(rename_all = "lowercase")]
pub enum Foo<S, T> where T: Clone {
    A(u32),
    B(u32, S),
    #[serde(rename_all = "UPPERCASE")]
    #[serde(rename = "OwO")]
    C {
        bar: T,
        #[serde(rename = "QwQ")]
        qux: Qux,
    },
    D,
    E(),
    F {},
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub struct Qux {}
