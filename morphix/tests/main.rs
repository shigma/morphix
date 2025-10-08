use morphix::{JsonAdapter, Mutation, MutationKind, Observe, observe};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Debug, PartialEq, Observe)]
pub struct Foo {
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

    let mutation = observe!(JsonAdapter, |mut foo| {
        foo.bar.baz += 1;
        foo.qux.push(' ');
        foo.qux += "world";
    })
    .unwrap();

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

    assert_eq!(
        foo,
        Foo {
            bar: Bar { baz: 43 },
            qux: "hello world".to_string(),
        }
    );
}
