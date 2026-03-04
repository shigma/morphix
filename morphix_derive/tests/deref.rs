use std::collections::HashMap;
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

#[derive(Serialize, Observe)]
struct FlatMap {
    #[serde(flatten)]
    map: HashMap<String, i32>,
    b: u32,
}

fn sorted_mutations(mutation: Option<Mutation<serde_json::Value>>) -> Vec<Mutation<serde_json::Value>> {
    let Some(mutation) = mutation else {
        return vec![];
    };
    let mut batch = match mutation.kind {
        MutationKind::Batch(batch) => batch,
        _ => vec![mutation],
    };
    batch.sort_by(|a, b| a.path.cmp(&b.path));
    batch
}

#[test]
fn flat_map_no_change() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {}).unwrap();
    assert!(mutation.is_none());
}

#[test]
fn flat_map_granular_insert() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        f.map.insert("y".into(), 2);
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: vec!["y".into()].into(),
            kind: MutationKind::Replace(json!(2)),
        })
    );
}

#[test]
fn flat_map_granular_remove() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1), ("y".into(), 2)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        f.map.remove("y");
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: vec!["y".into()].into(),
            kind: MutationKind::Delete,
        })
    );
}

#[test]
fn flat_map_b_only() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        f.b = 20;
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: vec!["b".into()].into(),
            kind: MutationKind::Replace(json!(20)),
        })
    );
}

#[test]
fn flat_map_map_and_b_no_collapse() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        f.map.insert("x".into(), 99);
        f.b = 20;
    })
    .unwrap();
    // Map insert is granular (is_replace=false), so no collapse despite b also changing
    let batch = sorted_mutations(mutation);
    assert_eq!(batch.len(), 2);
    assert_eq!(
        batch[0],
        Mutation {
            path: vec!["b".into()].into(),
            kind: MutationKind::Replace(json!(20)),
        }
    );
    assert_eq!(
        batch[1],
        Mutation {
            path: vec!["x".into()].into(),
            kind: MutationKind::Replace(json!(99)),
        }
    );
}

#[test]
fn flat_map_map_replace_b_unchanged() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        f.map.insert("x".into(), 99);
    })
    .unwrap();
    // Only map reports Replace, b unchanged → per-field mutation
    assert_eq!(
        mutation,
        Some(Mutation {
            path: vec!["x".into()].into(),
            kind: MutationKind::Replace(json!(99)),
        })
    );
}

#[test]
fn flat_map_deref_mut_full_replace() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        f = FlatMap { map: HashMap::from([("y".into(), 2)]), b: 20 };
    })
    .unwrap();
    // Full outer replace → whole-struct Replace
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Replace(json!({"y": 2, "b": 20})),
        })
    );
}

#[test]
fn flat_map_map_deref_mut_with_new_keys() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        *f.map = HashMap::from([("y".into(), 2)]);
    })
    .unwrap();
    // Map replaced via deref_mut (is_replace=true), b unchanged → no collapse
    // Map flatten produces: delete "x", replace "y"
    let batch = sorted_mutations(mutation);
    assert_eq!(batch.len(), 2);
    assert_eq!(
        batch[0],
        Mutation {
            path: vec!["x".into()].into(),
            kind: MutationKind::Delete,
        }
    );
    assert_eq!(
        batch[1],
        Mutation {
            path: vec!["y".into()].into(),
            kind: MutationKind::Replace(json!(2)),
        }
    );
}

#[test]
fn flat_map_map_deref_mut_and_b_collapse() {
    let mut f = FlatMap {
        map: HashMap::from([("x".into(), 1)]),
        b: 10,
    };
    let Json(mutation) = observe!(f => {
        *f.map = HashMap::from([("x".into(), 99)]);
        f.b = 20;
    })
    .unwrap();
    // Both map (is_replace=true) and b report Replace → collapse
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Replace(json!({"x": 99, "b": 20})),
        })
    );
}
