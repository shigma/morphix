use morphix::adapter::Json;
use morphix::{Mutation, MutationKind, Observe, observe};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Debug, PartialEq, Observe)]
#[morphix(derive(Debug))]
struct Foo<T> {
    bar: Bar,
    qux: T,
}

#[derive(Serialize, Debug, PartialEq, Observe)]
#[morphix(derive(Debug))]
struct Bar {
    baz: i32,
    #[serde(skip_serializing_if = "String::is_empty")]
    skip: String,
}

#[test]
fn main() {
    let mut foo = Foo {
        bar: Bar {
            baz: 42,
            skip: "test".into(),
        },
        qux: "hello".to_string(),
    };

    let Json(mutation) = observe!(foo => {
        foo.bar.baz += 1;
        foo.bar.skip.clear();
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
                    path: vec!["bar".into()].into(),
                    kind: MutationKind::Batch(vec![
                        Mutation {
                            path: vec!["baz".into()].into(),
                            kind: MutationKind::Replace(json!(43)),
                        },
                        Mutation {
                            path: vec!["skip".into()].into(),
                            kind: MutationKind::Delete,
                        },
                    ]),
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
            bar: Bar {
                baz: 43,
                skip: String::new()
            },
            qux: "hello world".to_string(),
        }
    );
}
