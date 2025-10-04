use morphix::{Change, Context, JsonAdapter, Ob, Observe, Operation};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Foo {
    pub bar: Bar,
    pub qux: String,
}

pub struct FooOb<'i> {
    pub bar: Ob<'i, Bar>,
    pub qux: Ob<'i, String>,
}

impl Observe for Foo {
    type Target<'i> = FooOb<'i>;

    fn observe(&mut self, ctx: &morphix::Context) -> Self::Target<'_> {
        FooOb {
            bar: morphix::Ob {
                value: &mut self.bar,
                ctx: ctx.extend("bar"),
            },
            qux: morphix::Ob {
                value: &mut self.qux,
                ctx: ctx.extend("qux"),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Bar {
    pub baz: i32,
}

pub struct BarOb<'i> {
    pub baz: Ob<'i, i32>,
}

impl Observe for Bar {
    type Target<'i> = BarOb<'i>;

    fn observe(&mut self, ctx: &morphix::Context) -> Self::Target<'_> {
        BarOb {
            baz: morphix::Ob {
                value: &mut self.baz,
                ctx: ctx.extend("baz"),
            },
        }
    }
}

#[test]
fn main() {
    let mut foo = Foo {
        bar: Bar { baz: 42 },
        qux: "hello".to_string(),
    };

    let change = {
        let ctx = Context::new();
        let mut foo = foo.observe(&ctx);
        foo.bar.baz += 1;
        *foo.qux += " world";
        ctx.collect::<JsonAdapter>()
    }
    .unwrap();

    assert_eq!(
        change,
        Some(Change {
            path_rev: vec![],
            operation: Operation::Batch(vec![
                Change {
                    path_rev: vec!["bar".to_string(), "baz".to_string()],
                    operation: Operation::Replace(json!(43)),
                },
                Change {
                    path_rev: vec!["qux".to_string()],
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
