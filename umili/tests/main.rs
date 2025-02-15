use std::{cell::RefCell, ops::AddAssign, rc::Rc};

use serde::{Serialize, Deserialize};
use umili::{Delta, Observe, Ob};
use umili_derive::observe;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Observe)]
pub struct Foo {
    pub bar: Bar,
    pub qux: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Observe)]
pub struct Bar {
    pub baz: i32,
}

#[test]
fn main() {
    let mut foo = Foo { bar: Bar { baz: 42 }, qux: "hello".to_string() };

    // 由 macro 展开
    let diff = observe!(|foo| {
        foo.bar.baz += 1;
        foo.qux += " world";
    });

    assert_eq!(diff, vec![
        Delta::SET { p: "bar/baz".into(), v: 43.into() },
        Delta::APPEND { p: "qux".into(), v: " world".into() },
    ]);

    assert_eq!(foo, Foo { bar: Bar { baz: 43 }, qux: "hello world".to_string() });
}
