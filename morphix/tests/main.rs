use morphix::{JsonAdapter, Mutation, MutationKind, Observe, observe};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Debug, PartialEq, Observe)]
pub struct Foo<T> {
    bar: Bar,
    qux: T,
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

    let mutation = observe!(JsonAdapter, |foo| {
        foo.bar.baz += 1;
        foo.qux.push(' ');
        foo.qux += "world";
    })
    .unwrap();

    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
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

    assert_eq!(
        foo,
        Foo {
            bar: Bar { baz: 43 },
            qux: "hello world".to_string(),
        }
    );
}
