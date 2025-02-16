use std::collections::BTreeMap;

use crate::change::{append, concat_path, split_path, Change, Error};

#[derive(Debug, Default)]
pub struct Batch {
    /// can only be SET or APPEND
    change: Option<Change>,
    children: BTreeMap<String, Batch>,
}

impl Batch {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(&mut self, mut change: Change) -> Result<(), Error> {
        let mut node = self;
        let mut parts = split_path(change.path());
        if let Some(Change::SET { v, .. }) = &mut node.change {
            change.apply(v)?;
            return Ok(())
        }
        while let Some(part) = parts.pop() {
            node = node.children.entry(part).or_default();
            *change.path_mut() = parts.join("/");
            if let Some(Change::SET { v, .. }) = &mut node.change {
                change.apply(v)?;
                return Ok(())
            }
        }
        match change {
            Change::SET { .. } => {
                node.change = Some(change);
                node.children.clear();
            },
            #[cfg(feature = "append")]
            Change::APPEND { p, v: rhs } => {
                match &mut node.change {
                    Some(Change::APPEND { v: lhs, .. }) => {
                        append(lhs, rhs)?
                    },
                    Some(_) => panic!("invalid append operation"),
                    None => node.change = Some(Change::APPEND { p, v: rhs }),
                }
            },
            Change::BATCH { .. } => unreachable!(),
        }
        Ok(())
    }

    pub fn dump(self) -> Change {
        let mut changes = vec![];
        if let Some(mut change) = self.change {
            *change.path_mut() = String::new();
            changes.push(change);
        }
        for (key, batch) in self.children {
            let mut change = batch.dump();
            *change.path_mut() = concat_path(key, change.path());
            changes.push(change);
        }
        match changes.len() {
            1 => changes.swap_remove(0),
            _ => Change::batch("", changes),
        }
    }
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn batch() {
        let batch = Batch::new();
        assert_eq!(batch.dump(), Change::batch("", vec![]));

        let mut batch = Batch::new();
        batch.load(Change::set("foo/bar", 1).unwrap()).unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", 1).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::set("foo/bar", 1).unwrap()).unwrap();
        batch.load(Change::set("foo/bar", 2).unwrap()).unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", 2).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::set("foo/bar", json!({"qux": "1"})).unwrap()).unwrap();
        batch.load(Change::append("foo/bar/qux", "2").unwrap()).unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", json!({"qux": "12"})).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::append("foo/bar/qux", "2").unwrap()).unwrap();
        batch.load(Change::set("foo/bar", json!({"qux": "1"})).unwrap()).unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", json!({"qux": "1"})).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::append("bar", "2").unwrap()).unwrap();
        batch.load(Change::append("qux", "1").unwrap()).unwrap();
        assert_eq!(batch.dump(), Change::batch("", vec![
            Change::append("bar", "2").unwrap(),
            Change::append("qux", "1").unwrap(),
        ]));

        let mut batch = Batch::new();
        batch.load(Change::append("foo/bar", "2").unwrap()).unwrap();
        batch.load(Change::append("foo/qux", "1").unwrap()).unwrap();
        assert_eq!(batch.dump(), Change::batch("foo", vec![
            Change::append("bar", "2").unwrap(),
            Change::append("qux", "1").unwrap(),
        ]));
    }
}
