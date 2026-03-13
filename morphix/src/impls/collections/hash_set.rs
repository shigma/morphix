use std::borrow::Borrow;
use std::collections::hash_set::ExtractIf;
use std::collections::{HashSet, TryReserveError};
use std::hash::Hash;

use crate::Observe;
use crate::builtin::{ShallowObserver, Snapshot};
use crate::helper::macros::{default_impl_ref_observe, delegate_methods};
use crate::helper::{AsDerefMut, QuasiObserver, Unsigned};
use crate::observe::DefaultSpec;

impl<T> Observe for HashSet<T> {
    type Observer<'ob, S, D>
        = ShallowObserver<'ob, S, D>
    where
        Self: 'ob,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob;

    type Spec = DefaultSpec;
}

default_impl_ref_observe! {
    impl [T] RefObserve for HashSet<T>;
}

impl<T> Snapshot for HashSet<T>
where
    T: Snapshot,
    T::Snapshot: Eq + Hash,
{
    type Snapshot = HashSet<T::Snapshot>;

    fn to_snapshot(&self) -> Self::Snapshot {
        self.iter().map(|item| item.to_snapshot()).collect()
    }

    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.len() == snapshot.len() && self.iter().all(|item| snapshot.contains(&item.to_snapshot()))
    }
}

impl<'ob, S: ?Sized, D, T> ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashSet<T>>,
    T: Eq + Hash,
{
    delegate_methods! { untracked_mut() as HashSet =>
        pub fn reserve(&mut self, additional: usize);
        pub fn try_reserve(&mut self, additional: usize) -> Result<(), TryReserveError>;
        pub fn shrink_to_fit(&mut self);
        pub fn shrink_to(&mut self, min_capacity: usize);
    }

    delegate_methods! { tracked_mut() as HashSet =>
        pub fn insert(&mut self, value: T) -> bool;
        pub fn replace(&mut self, value: T) -> Option<T>;
        pub fn remove<Q>(&mut self, value: &Q) -> bool
        where T: Borrow<Q>, Q: Hash + Eq + ?Sized;
        pub fn take<Q>(&mut self, value: &Q) -> Option<T>
        where T: Borrow<Q>, Q: Hash + Eq + ?Sized;
        pub fn retain<F>(&mut self, f: F)
        where F: FnMut(&T) -> bool;
        pub fn extract_if<F>(&mut self, pred: F) -> ExtractIf<'_, T, F>
        where F: FnMut(&T) -> bool;
    }

    /// See [`HashSet::clear`].
    pub fn clear(&mut self) {
        if (*self).untracked_ref().is_empty() {
            self.untracked_mut().clear()
        } else {
            self.tracked_mut().clear()
        }
    }
}

impl<'ob, S: ?Sized, D, T, U> Extend<U> for ShallowObserver<'ob, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = HashSet<T>>,
    HashSet<T>: Extend<U>,
{
    fn extend<I: IntoIterator<Item = U>>(&mut self, iter: I) {
        self.tracked_mut().extend(iter);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use serde_json::Value;

    use crate::adapter::Json;
    use crate::helper::QuasiObserver;
    use crate::observe::{ObserveExt, SerializeObserverExt};
    use crate::{Mutation, MutationKind};

    fn is_replace(mutation: &Option<Mutation<Value>>) -> bool {
        match mutation {
            Some(m) => m.path.is_empty() && matches!(m.kind, MutationKind::Replace(_)),
            None => false,
        }
    }

    #[test]
    fn no_change() {
        let mut set = HashSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn insert_triggers_replace() {
        let mut set = HashSet::from([1, 2]);
        let mut ob = set.__observe();
        ob.insert(3);
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn remove_existing_triggers_replace() {
        let mut set = HashSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        assert!(ob.remove(&2));
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn clear_empty_no_mutation() {
        let mut set: HashSet<i32> = HashSet::new();
        let mut ob = set.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn clear_non_empty_triggers_replace() {
        let mut set = HashSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.clear();
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn double_flush() {
        let mut set = HashSet::from([1, 2]);
        let mut ob = set.__observe();
        ob.insert(3);
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn reserve_no_mutation() {
        let mut set = HashSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.reserve(100);
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn extend_triggers_replace() {
        let mut set = HashSet::from([1]);
        let mut ob = set.__observe();
        ob.extend([2, 3, 4]);
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn deref_mut_triggers_replace() {
        let mut set = HashSet::from([1, 2]);
        let mut ob = set.__observe();
        **ob = HashSet::from([10, 20, 30]);
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn retain_triggers_replace() {
        let mut set = HashSet::from([1, 2, 3, 4]);
        let mut ob = set.__observe();
        ob.retain(|&x| x % 2 == 0);
        assert_eq!(*ob.untracked_ref(), HashSet::from([2, 4]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn extract_if_triggers_replace() {
        let mut set = HashSet::from([1, 2, 3, 4]);
        let mut ob = set.__observe();
        let extracted: HashSet<_> = ob.extract_if(|&x| x % 2 == 0).collect();
        assert_eq!(extracted, HashSet::from([2, 4]));
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn take_triggers_replace() {
        let mut set = HashSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        assert_eq!(ob.take(&2), Some(2));
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }

    #[test]
    fn replace_triggers_replace() {
        let mut set = HashSet::from([1, 2, 3]);
        let mut ob = set.__observe();
        ob.replace(2);
        let Json(mutation) = ob.flush().unwrap();
        assert!(is_replace(&mutation));
    }
}
