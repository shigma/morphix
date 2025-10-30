// Add leading colons to std imports to avoid rustfmt inserting newlines
use ::std::ops::{Deref, DerefMut};
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
struct Foo {
    #[observe(deref)]
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
struct Bar {
    #[observe(deref, shallow)]
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
struct Qux {
    a: i32,
}
