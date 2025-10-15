# morphix

[![Crates.io](https://img.shields.io/crates/v/morphix.svg)](https://crates.io/crates/morphix)
[![Documentation](https://docs.rs/morphix/badge.svg)](https://docs.rs/morphix)

A Rust library for observing and serializing mutations.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
morphix = { version = "0.3", features = ["json"] }
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
        path: vec![].into(),
        kind: MutationKind::Batch(vec![
            Mutation {
                path: vec!["bar".into(), "baz".into()].into(),
                kind: MutationKind::Replace(json!(43)),
            },
            Mutation {
                path: vec!["qux".into()].into(),
                kind: MutationKind::Append(json!(" world")),
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

```rs
foo.a.b = 1;        // Replace at .a.b
foo.num *= 2;       // Replace at .num
foo.vec.clear();    // Replace at .vec
```

### Append

Optimized for appending to strings and vectors:

```rs
foo.a.b += "text";          // Append to .a.b
foo.a.b.push_str("text");   // Append to .a.b
foo.vec.push(1);            // Append to .vec
foo.vec.extend(iter);       // Append to .vec
```

### Batch

Multiple mutations combined into a single operation.

## Features

- `derive` (default): Enables the Observe derive macro
- `json`: Includes JSON serialization support via `serde_json`
- `yaml`: Includes YAML serialization support via `serde_yaml_ng`
