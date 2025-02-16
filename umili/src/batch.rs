use std::collections::BTreeMap;

use crate::change::{append, concat_path, split_path, Change};
use crate::error::Error;

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

    pub fn load(&mut self, mut change: Change, prefix: &str) -> Result<(), Error> {
        let mut batch = self;
        let mut parts = split_path(change.path());
        if let Some(Change::SET { v, .. }) = &mut batch.change {
            change.apply(v, prefix)?;
            return Ok(())
        }
        let mut prefix = prefix.to_string();
        while let Some(key) = parts.pop() {
            prefix += key;
            prefix += "/";
            batch = batch.children.entry(key.to_string()).or_default();
            if let Some(Change::SET { v, .. }) = &mut batch.change {
                *change.path_mut() = parts.join("/");
                change.apply(v, &prefix)?;
                return Ok(())
            }
        }
        match change {
            Change::SET { .. } => {
                batch.change = Some(change);
                batch.children.clear();
            },
            #[cfg(feature = "append")]
            Change::APPEND { p, v } => {
                match &mut batch.change {
                    Some(Change::APPEND { v: lhs, .. }) => {
                        if !append(lhs, v) {
                            prefix.pop();
                            return Err(Error::OperationError { path: prefix })
                        }
                    },
                    Some(_) => unreachable!(),
                    None => batch.change = Some(Change::APPEND { p, v }),
                }
            },
            Change::BATCH { v, .. } => {
                for change in v {
                    batch.load(change, &prefix)?;
                }
            },
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
        batch.load(Change::set("foo/bar", 1).unwrap(), "").unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", 1).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::set("foo/bar", 1).unwrap(), "").unwrap();
        batch.load(Change::set("foo/bar", 2).unwrap(), "").unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", 2).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::set("foo/bar", json!({"qux": "1"})).unwrap(), "").unwrap();
        batch.load(Change::append("foo/bar/qux", "2").unwrap(), "").unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", json!({"qux": "12"})).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::append("foo/bar/qux", "2").unwrap(), "").unwrap();
        batch.load(Change::set("foo/bar", json!({"qux": "1"})).unwrap(), "").unwrap();
        assert_eq!(batch.dump(), Change::set("foo/bar", json!({"qux": "1"})).unwrap());

        let mut batch = Batch::new();
        batch.load(Change::batch("foo", vec![
            Change::append("bar", "1").unwrap(),
            Change::append("bar", "2").unwrap(),
        ]), "").unwrap();
        assert_eq!(batch.dump(), Change::append("foo/bar", "12").unwrap());

        let mut batch = Batch::new();
        batch.load(Change::append("bar", "2").unwrap(), "").unwrap();
        batch.load(Change::append("qux", "1").unwrap(), "").unwrap();
        assert_eq!(batch.dump(), Change::batch("", vec![
            Change::append("bar", "2").unwrap(),
            Change::append("qux", "1").unwrap(),
        ]));

        let mut batch = Batch::new();
        batch.load(Change::append("foo/bar", "2").unwrap(), "").unwrap();
        batch.load(Change::append("foo/qux", "1").unwrap(), "").unwrap();
        assert_eq!(batch.dump(), Change::batch("foo", vec![
            Change::append("bar", "2").unwrap(),
            Change::append("qux", "1").unwrap(),
        ]));
    }
}
