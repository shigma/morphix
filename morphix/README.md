# morphix

Mutate and observe Rust data structures.

## Basic Usage

```rust
use serde::Serialize;
use serde_json::json;
use morphix::{Change, JsonAdapter, Observe, Operation, observe};

// 1. Define any data structure with `#[derive(Observe)]`.
#[derive(Serialize, PartialEq, Debug, Observe)]
struct Foo {
    pub bar: Bar,
    pub qux: String,
}

#[derive(Serialize, PartialEq, Debug, Observe)]
struct Bar {
    pub baz: i32,
}

let mut foo = Foo {
    bar: Bar { baz: 42 },
    qux: "hello".to_string(),
};

// 2. Use `observe!` to mutate and observe the data structure.
let change = observe!(JsonAdapter, |mut foo| {
    foo.bar.baz += 1;
    foo.qux.push(' ');
    foo.qux += "world";
})
.unwrap();

// 3. See the changes.
assert_eq!(
    change,
    Some(Change {
        path_rev: vec![],
        operation: Operation::Batch(vec![
            Change {
                path_rev: vec!["baz".into(), "bar".into()],
                operation: Operation::Replace(json!(43)),
            },
            Change {
                path_rev: vec!["qux".into()],
                operation: Operation::Append(json!(" world")),
            },
        ]),
    }),
);

// 4. The original data structure is also mutated.
assert_eq!(
    foo,
    Foo {
        bar: Bar { baz: 43 },
        qux: "hello world".to_string(),
    },
);
```
