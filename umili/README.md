# umili

Mutate and observe Rust data structures.

## Basic Usage

```rust
use serde::{Serialize, Deserialize};
use umili::{observe, Change, Observe};

// 1. Define any data structure with `#[derive(Observe)]`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Observe)]
pub struct Foo {
    pub bar: Bar,
    pub qux: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Observe)]
pub struct Bar {
    pub baz: i32,
}

let mut foo = Foo { bar: Bar { baz: 42 }, qux: "hello".to_string() };

// 2. Use `observe!` to mutate and observe the data structure.
let diff = observe!(|mut foo| {
    foo.bar.baz += 1;
    foo.qux += " world";
}).unwrap();

// 3. See the changes.
assert_eq!(diff, vec![
    Change::set("bar/baz", 43).unwrap(),
    Change::append("qux", " world").unwrap(),
]);

assert_eq!(foo, Foo { bar: Bar { baz: 43 }, qux: "hello world".to_string() });
```
