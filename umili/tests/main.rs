use std::{cell::RefCell, ops::AddAssign, rc::Rc};

use serde::{Serialize, Deserialize};
use umili::{Delta, Observe, Ob};

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
    // let diff = observe!(foo, {
    //     foo.bar.baz += 1;
    //     foo.qux += " world";
    // });
    let diff = foo.with_observe(|mut foo| {
        foo.bar.borrow().baz.borrow_mut().add_assign(1);
        foo.qux.borrow_mut().add_assign(" world");
    });

    println!("{:?}", diff);
    println!("{:?}", foo);
}
