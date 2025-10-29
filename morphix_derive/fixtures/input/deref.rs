use std::ops::{Deref, DerefMut};

use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
struct Foo {
    #[observe(deref)]
    a: Bar,
    b: i32,
}

impl Deref for Foo {
    type Target = Bar;

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
    a: i32,
}
