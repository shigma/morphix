use std::{cell::RefCell, ops::{AddAssign, Deref, DerefMut}, rc::Rc};

use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Foo {
    pub bar: Bar,
    pub qux: String,
}

// 由 derive macro 生成
impl Foo {
    pub fn as_mut(&mut self, prefix: &str, diff: &Rc<RefCell<Vec<Delta>>>) -> FooMut {
        FooMut {
            bar: Observable {
                value: &mut self.bar,
                path: prefix.to_string() + "bar",
                diff: diff.clone(),
            },
            qux: Observable {
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
            baz: Observable {
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
    pub bar: Observable<'i, Bar>,
    pub qux: Observable<'i, String>,
}

// 由 derive macro 生成
pub struct BarMut<'i> {
    pub baz: Observable<'i, i32>,
}

pub struct Observable<'i, T: Clone + Serialize + PartialEq> {
    pub value: &'i mut T,
    pub path: String,
    pub diff: Rc<RefCell<Vec<Delta>>>,
}

impl<'i, T: Clone + Serialize + PartialEq> Deref for Observable<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Observable<'i, T> {
    pub fn borrow_mut(&mut self) -> Ref<T> {
        Ref {
            old_value: None,
            value: self.value,
            path: &self.path,
            diff: self.diff.clone(),
        }
    }
}

// 由 derive macro 生成
impl<'i> Observable<'i, Bar> {
    #[inline]
    pub fn borrow(&mut self) -> BarMut {
        let prefix = self.path.to_string() + "/";
        self.value.as_mut(&prefix, &self.diff)
    }
}

pub struct Ref<'i, T: Clone + Serialize + PartialEq> {
    pub old_value: Option<T>,
    pub value: &'i mut T,
    pub path: &'i str,
    pub diff: Rc<RefCell<Vec<Delta>>>,
}

impl<'i> Ref<'i, String> {
    pub fn add_assign(&mut self, s: &str) {
        self.push_str(s);
    }


    pub fn push(&mut self, c: char) {
        self.diff.borrow_mut().push(Delta::APPEND {
            p: self.path.to_string(),
            v: serde_json::to_value(c).unwrap(),
        });
        self.value.push(c);
    }

    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        self.diff.borrow_mut().push(Delta::APPEND {
            p: self.path.to_string(),
            v: serde_json::to_value(s).unwrap(),
        });
        self.value.push_str(s);
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Ref<'i, Vec<T>> {
    pub fn push(&mut self, value: T) {
        self.diff.borrow_mut().push(Delta::APPEND {
            p: self.path.to_string(),
            v: serde_json::to_value(vec![&value]).unwrap(),
        });
        self.value.push(value);
    }

    pub fn extend(&mut self, other: Vec<T>) { // FIXME iter
        if other.is_empty() {
            return;
        }
        self.diff.borrow_mut().push(Delta::APPEND {
            p: self.path.to_string(),
            v: serde_json::to_value(&other).unwrap(),
        });
        self.value.extend(other);
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Deref for Ref<'i, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq> DerefMut for Ref<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.old_value.is_none() {
            self.old_value.replace(self.value.clone());
        }
        self.value
    }
}

impl<'i, T: Clone + Serialize + PartialEq> Drop for Ref<'i, T> {
    fn drop(&mut self) {
        if let Some(old_value) = self.old_value.take() {
            if old_value != *self.value {
                self.diff.borrow_mut().push(Delta::SET {
                    p: self.path.to_string(),
                    v: serde_json::to_value(&self.value).unwrap(),
                });
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "o")]
pub enum Delta {
    SET { p: String, v: Value },
    APPEND { p: String, v: Value },
    BATCH { p: String, v: Vec<Delta> },
    HISTORY { p: String, v: DeltaTag },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DeltaTag {
    SET,
    APPEND,
    BATCH,
    HISTORY,
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
