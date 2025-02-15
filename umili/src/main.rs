use std::{cell::RefCell, ops::AddAssign, rc::Rc};

use serde::{Serialize, Deserialize};
use umili::{Delta, Observe, Ob};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Foo {
    pub bar: Bar,
    pub qux: String,
}

// 由 derive macro 生成
impl Observe for Foo {
    type Target<'i> = FooMut<'i>;

    fn observe(&mut self, prefix: &str, diff: &Rc<RefCell<Vec<Delta>>>) -> FooMut {
        FooMut {
            bar: Ob {
                value: &mut self.bar,
                path: prefix.to_string() + "bar",
                diff: diff.clone(),
            },
            qux: Ob {
                value: &mut self.qux,
                path: prefix.to_string() + "qux",
                diff: diff.clone(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Bar {
    pub baz: i32,
}

// 由 derive macro 生成
impl Observe for Bar {
    type Target<'i> = BarMut<'i>;

    fn observe(&mut self, prefix: &str, diff: &Rc<RefCell<Vec<Delta>>>) -> BarMut {
        BarMut {
            baz: Ob {
                value: &mut self.baz,
                path: prefix.to_string() + "baz",
                diff: diff.clone(),
            },
        }
    }
}

// 由 derive macro 生成
pub struct FooMut<'i> {
    pub bar: Ob<'i, Bar>,
    pub qux: Ob<'i, String>,
}

// 由 derive macro 生成
pub struct BarMut<'i> {
    pub baz: Ob<'i, i32>,
}

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
