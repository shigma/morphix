use std::ops::{Deref, DerefMut};

use morphix::adapter::Json;
use morphix::{Mutation, MutationKind, Observe, observe};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Observe)]
struct VecWrapper(#[morphix(deref)] Vec<i32>);

impl Deref for VecWrapper {
    type Target = Vec<i32>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for VecWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[test]
fn deref_delegates() {
    let mut w = VecWrapper(vec![1, 2, 3]);
    let Json(mutation) = observe!(w => {
        w.push(4);
    })
    .unwrap();
    // Vec push produces Append through the deref observer
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Append(json!([4])),
        })
    );
}

#[test]
fn deref_no_mutation() {
    let mut w = VecWrapper(vec![1, 2, 3]);
    let Json(mutation) = observe!(w => {}).unwrap();
    assert!(mutation.is_none());
}

#[test]
fn deref_vec_replace() {
    let mut w = VecWrapper(vec![1, 2, 3]);
    let Json(mutation) = observe!(w => {
        w.clear();
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Replace(json!([])),
        })
    );
}

#[test]
fn deref_flush_resets() {
    let mut w = VecWrapper(vec![1, 2, 3]);
    let Json(mutation1) = observe!(w => {
        w.push(4);
    })
    .unwrap();
    assert!(mutation1.is_some());

    let Json(mutation2) = observe!(w => {}).unwrap();
    assert!(mutation2.is_none());
}

#[derive(Serialize, Observe)]
struct Inner {
    c: i32,
}

#[derive(Serialize, Observe)]
struct Outer {
    a: i32,
    b: i32,
    #[serde(flatten)]
    #[morphix(deref)]
    inner: Inner,
}

impl Deref for Outer {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Outer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[test]
fn deref_replace_outer() {
    let mut o = Outer {
        a: 1,
        b: 1,
        inner: Inner { c: 2 },
    };
    let Json(mutation) = observe!(o => {
        o.a = 10;
        o = Outer { a: 100, b: 100, inner: Inner { c: 200 } };
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Replace(json!({"a": 100, "b": 100, "c": 200})),
        })
    );
}

#[test]
fn deref_replace_inner() {
    let mut o = Outer {
        a: 1,
        b: 1,
        inner: Inner { c: 2 },
    };
    let Json(mutation) = observe!(o => {
        o.a = 10;
        *o = Inner { c: 200 };
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Batch(vec![
                Mutation {
                    path: vec!["a".into()].into(),
                    kind: MutationKind::Replace(json!(10)),
                },
                Mutation {
                    path: vec!["c".into()].into(),
                    kind: MutationKind::Replace(json!(200)),
                },
            ]),
        })
    );
}
