use std::{cell::RefCell, ops::AddAssign, rc::Rc};

use serde::{Serialize, Deserialize};
use umili::{Delta, Observer};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Foo {
    pub bar: Bar,
    pub qux: String,
}

// 由 derive macro 生成
impl Foo {
    pub fn as_mut(&mut self, prefix: &str, diff: &Rc<RefCell<Vec<Delta>>>) -> FooMut {
        FooMut {
            bar: Observer {
                value: &mut self.bar,
                path: prefix.to_string() + "bar",
                diff: diff.clone(),
            },
            qux: Observer {
                value: &mut self.qux,
                path: prefix.to_string() + "qux",
                diff: diff.clone(),
            },
        }
    }

    pub fn observe<'i, F: FnOnce(FooMut<'i>)>(&'i mut self, f: F) -> Vec<Delta> {
        let diff = Rc::new(RefCell::new(vec![]));
        f(self.as_mut("", &diff));
        diff.take()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Bar {
    pub baz: i32,
}

// 由 derive macro 生成
impl Bar {
    pub fn as_mut(&mut self, prefix: &str, diff: &Rc<RefCell<Vec<Delta>>>) -> BarMut {
        BarMut {
            baz: Observer {
                value: &mut self.baz,
                path: prefix.to_string() + "baz",
                diff: diff.clone(),
            },
        }
    }

    pub fn observe<'i, F: FnOnce(BarMut<'i>)>(&'i mut self, f: F) -> Vec<Delta> {
        let diff = Rc::new(RefCell::new(vec![]));
        f(self.as_mut("", &diff));
        diff.take()
    }
}

// 由 derive macro 生成
pub struct FooMut<'i> {
    pub bar: Observer<'i, Bar>,
    pub qux: Observer<'i, String>,
}

// 由 derive macro 生成
pub struct BarMut<'i> {
    pub baz: Observer<'i, i32>,
}

// 由 derive macro 生成
impl<'i> Observer<'i, Bar> {
    #[inline]
    pub fn borrow(&mut self) -> BarMut {
        let prefix = self.path.to_string() + "/";
        self.value.as_mut(&prefix, &self.diff)
    }
}

fn main() {
    let mut foo = Foo { bar: Bar { baz: 42 }, qux: "hello".to_string() };

    // 由 macro 展开
    // let diff = observe!(foo, {
    //     foo.bar.baz += 1;
    //     foo.qux += " world";
    // });
    let diff = foo.observe(|mut foo| {
        foo.bar.borrow().baz.borrow_mut().add_assign(1);
        foo.qux.borrow_mut().add_assign(" world");
    });

    println!("{:?}", diff);
    println!("{:?}", foo);
}
