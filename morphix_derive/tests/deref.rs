use std::ops::{Deref, DerefMut};

use morphix::adapter::Json;
use morphix::{Mutation, MutationKind, Observe, observe};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize, Observe)]
struct VecWrapper {
    #[serde(flatten)]
    #[morphix(deref)]
    inner: Vec<i32>,
}

impl Deref for VecWrapper {
    type Target = Vec<i32>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for VecWrapper {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[test]
fn deref_delegates_to_inner_observer() {
    let mut w = VecWrapper { inner: vec![1, 2, 3] };
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
    let mut w = VecWrapper { inner: vec![1, 2, 3] };
    let Json(mutation) = observe!(w => {}).unwrap();
    assert!(mutation.is_none());
}

#[derive(Serialize, Observe)]
struct TaggedVec {
    #[serde(flatten)]
    #[morphix(deref)]
    items: Vec<i32>,
    label: String,
}

impl Deref for TaggedVec {
    type Target = Vec<i32>;

    fn deref(&self) -> &Self::Target {
        &self.items
    }
}

impl DerefMut for TaggedVec {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.items
    }
}

#[test]
fn non_deref_alongside_deref() {
    let mut t = TaggedVec {
        items: vec![1],
        label: "test".into(),
    };
    let Json(mutation) = observe!(t => {
        t.push(2);
        t.label.push_str("!");
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Batch(vec![
                Mutation {
                    path: Default::default(),
                    kind: MutationKind::Append(json!([2])),
                },
                Mutation {
                    path: vec!["label".into()].into(),
                    kind: MutationKind::Append(json!("!")),
                },
            ]),
        })
    );
}

#[test]
fn deref_only_label_mutation() {
    let mut t = TaggedVec {
        items: vec![1],
        label: "test".into(),
    };
    let Json(mutation) = observe!(t => {
        t.label.push_str("!");
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: vec!["label".into()].into(),
            kind: MutationKind::Append(json!("!")),
        })
    );
}

#[derive(Serialize, Observe)]
struct IntBox(#[morphix(deref)] i32);

impl Deref for IntBox {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for IntBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[test]
fn deref_single_field_replace() {
    let mut b = IntBox(42);
    let Json(mutation) = observe!(b => {
        *b = 99;
    })
    .unwrap();
    assert_eq!(
        mutation,
        Some(Mutation {
            path: Default::default(),
            kind: MutationKind::Replace(json!(99)),
        })
    );
}

#[test]
fn deref_vec_replace() {
    let mut w = VecWrapper { inner: vec![1, 2, 3] };
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
    let mut w = VecWrapper { inner: vec![1, 2, 3] };
    let Json(mutation1) = observe!(w => {
        w.push(4);
    })
    .unwrap();
    assert!(mutation1.is_some());

    let Json(mutation2) = observe!(w => {}).unwrap();
    assert!(mutation2.is_none());
}
