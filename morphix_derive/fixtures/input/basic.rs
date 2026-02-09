// Add leading colons to std imports to avoid rustfmt inserting newlines
use ::std::fmt::Display;
#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Debug, Serialize, Observe)]
#[morphix(derive(Debug, Display))]
#[serde(rename_all = "UPPERCASE")]
pub struct Foo {
    r#a: i32,
    #[serde(rename = "bar")]
    b: String,
}

impl Display for Foo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Foo {{ a: {}, b: {} }}", self.a, self.b)
    }
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub struct Bar(i32);

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[morphix(derive(Debug))]
pub struct Baz(i32, String);
