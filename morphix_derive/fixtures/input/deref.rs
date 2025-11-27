// Add leading colons to std imports to avoid rustfmt inserting newlines
use ::std::ops::{Deref, DerefMut};
#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub struct Foo {
    #[serde(flatten)]
    #[morphix(deref)]
    a: Qux,
    b: i32,
}

impl Deref for Foo {
    type Target = Qux;

    fn deref(&self) -> &Self::Target {
        &self.a
    }
}

impl DerefMut for Foo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.a
    }
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub struct Bar {
    #[morphix(deref, shallow)]
    a: Qux,
    b: i32,
}

impl Deref for Bar {
    type Target = Qux;

    fn deref(&self) -> &Self::Target {
        &self.a
    }
}

impl DerefMut for Bar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.a
    }
}

#[rustfmt::skip]
#[derive(Serialize, Observe)]
pub struct Qux {
    a: i32,
}
