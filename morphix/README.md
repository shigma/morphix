# morphix

[![Crates.io](https://img.shields.io/crates/v/morphix.svg)](https://crates.io/crates/morphix)
[![Documentation](https://docs.rs/morphix/badge.svg)](https://docs.rs/morphix)
 
A Rust library for observing and serializing mutations.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
morphix = { version = "0.1", features = ["json"] }
```

## Basic Usage

```rust
use serde::Serialize;
use serde_json::json;
use morphix::{JsonAdapter, Mutation, MutationKind, Observe, observe};

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

// 2. Use `observe!` to mutate data and track mutations.
let mutation = observe!(JsonAdapter, |mut foo| {
    foo.bar.baz += 1;
    foo.qux.push(' ');
    foo.qux += "world";
}).unwrap();

// 3. Inspect the mutations.
assert_eq!(
    mutation,
    Some(Mutation {
        path_rev: vec![],
        operation: MutationKind::Batch(vec![
            Mutation {
                path_rev: vec!["baz".into(), "bar".into()],
                operation: MutationKind::Replace(json!(43)),
            },
            Mutation {
                path_rev: vec!["qux".into()],
                operation: MutationKind::Append(json!(" world")),
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

## Mutation Types

Morphix recognizes three types of mutations:

### Replace

The most general mutation type, used for any mutation that replaces a value:

```rust ignore
person.age = 35;                        // Replace at .age
person.name = "Bob".into();             // Replace at .name
```

### Append

Optimized for appending to strings and vectors:

```rust ignore
person.name.push_str(" Smith");         // Append to .name
person.hobbies.push("gaming".into());   // Append to .hobbies
```

### Batch

Multiple mutations combined into a single operation.

## Features

- `derive` (default): Enables the Observe derive macro
- `json`: Includes JSON serialization support via `serde_json`
- `yaml`: Includes YAML serialization support via `serde_yml`
