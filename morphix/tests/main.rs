use morphix::{Change, JsonAdapter, Observe, Operation, observe};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Debug, PartialEq, Observe)]
struct Foo {
    bar: Bar,
    qux: String,
}

#[derive(Serialize, Debug, PartialEq, Observe)]
struct Bar {
    baz: i32,
}

#[test]
fn main() {
    let mut foo = Foo {
        bar: Bar { baz: 42 },
        qux: "hello".to_string(),
    };

    let change: Option<Change<JsonAdapter>> = observe!(|mut foo| {
        foo.bar.baz += 1;
        foo.qux += " world";
    })
    .unwrap();

    assert_eq!(
        change,
        Some(Change {
            path_rev: vec![],
            operation: Operation::Batch(vec![
                Change {
                    path_rev: vec!["bar".into(), "baz".into()],
                    operation: Operation::Replace(json!(43)),
                },
                Change {
                    path_rev: vec!["qux".into()],
                    operation: Operation::Append(json!(" world")),
                },
            ]),
        }),
    );

    assert_eq!(
        foo,
        Foo {
            bar: Bar { baz: 43 },
            qux: "hello world".to_string()
        }
    );
}
