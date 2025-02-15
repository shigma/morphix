use serde::{Serialize, Deserialize};
use umili::{observe, Delta, Ob, Observe};

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

    let diff = observe!(|mut foo| {
        foo.bar.baz += 1;
        foo.qux += " world";
    });

    assert_eq!(diff, vec![
        Delta::set("bar/baz", 43),
        Delta::append("qux", " world"),
    ]);

    assert_eq!(foo, Foo { bar: Bar { baz: 43 }, qux: "hello world".to_string() });
}
