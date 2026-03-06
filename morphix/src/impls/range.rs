use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range, RangeFrom, RangeInclusive, RangeTo};

use serde::Serialize;

use crate::Mutations;
use crate::builtin::Snapshot;
use crate::helper::macros::{spec_impl_observe, spec_impl_ref_observe};
use crate::helper::{AsDeref, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{Observer, SerializeObserver};

/// Observer implementation for [`Range<Idx>`].
pub struct RangeObserver<O, S: ?Sized, D = Zero> {
    /// See [`Range::start`].
    pub start: O,
    /// See [`Range::end`].
    pub end: O,
    ptr: Pointer<S>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> QuasiObserver for RangeObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        O::invalidate(&mut this.start);
        O::invalidate(&mut this.end);
    }
}

impl<O, S: ?Sized, D> Observer for RangeObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Range<O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            start: O::uninit(),
            end: O::uninit(),
            ptr: Pointer::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let value = head.as_deref();
        let mut this = Self {
            start: O::observe(&value.start),
            end: O::observe(&value.end),
            ptr: Pointer::new(head),
            phantom: PhantomData,
        };
        Pointer::register_observer(&mut this.ptr, &mut this.start);
        Pointer::register_observer(&mut this.ptr, &mut this.end);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(this, head);
        let value = head.as_deref();
        unsafe {
            O::refresh(&mut this.start, &value.start);
            O::refresh(&mut this.end, &value.end);
        }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for RangeObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = Range<O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized + 'static,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let mutations_start = unsafe { SerializeObserver::flush(&mut this.start) };
        let mutations_end = unsafe { SerializeObserver::flush(&mut this.end) };
        if mutations_start.is_replace() && mutations_end.is_replace() {
            Mutations::replace((*this).untracked_ref())
        } else {
            let mut mutations = Mutations::new();
            mutations.insert("start", mutations_start);
            mutations.insert("end", mutations_end);
            mutations
        }
    }

    unsafe fn flat_flush(this: &mut Self) -> (Mutations, bool) {
        let mutations_start = unsafe { SerializeObserver::flush(&mut this.start) };
        let mutations_end = unsafe { SerializeObserver::flush(&mut this.end) };
        let is_replace = mutations_start.is_replace() && mutations_end.is_replace();
        let mut mutations = Mutations::new();
        mutations.insert("start", mutations_start);
        mutations.insert("end", mutations_end);
        (mutations, is_replace)
    }
}

impl<T: Snapshot> Snapshot for Range<T> {
    type Snapshot = Range<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.start.to_snapshot()..self.end.to_snapshot()
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.start.eq_snapshot(&snapshot.start) && self.end.eq_snapshot(&snapshot.end)
    }
}

/// Observer implementation for [`RangeFrom<Idx>`].
pub struct RangeFromObserver<O, S: ?Sized, D = Zero> {
    /// See [`RangeFrom::start`].
    pub start: O,
    ptr: Pointer<S>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> QuasiObserver for RangeFromObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        O::invalidate(&mut this.start);
    }
}

impl<O, S: ?Sized, D> Observer for RangeFromObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeFrom<O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            start: O::uninit(),
            ptr: Pointer::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let value = head.as_deref();
        let mut this = Self {
            start: O::observe(&value.start),
            ptr: Pointer::new(head),
            phantom: PhantomData,
        };
        Pointer::register_observer(&mut this.ptr, &mut this.start);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(this, head);
        let value = head.as_deref();
        unsafe {
            O::refresh(&mut this.start, &value.start);
        }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for RangeFromObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeFrom<O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized + 'static,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let mutations_start = unsafe { SerializeObserver::flush(&mut this.start) };
        if mutations_start.is_replace() {
            Mutations::replace((*this).untracked_ref())
        } else {
            let mut mutations = Mutations::new();
            mutations.insert("start", mutations_start);
            mutations
        }
    }

    unsafe fn flat_flush(this: &mut Self) -> (Mutations, bool) {
        let mutations_start = unsafe { SerializeObserver::flush(&mut this.start) };
        let is_replace = mutations_start.is_replace();
        let mut mutations = Mutations::new();
        mutations.insert("start", mutations_start);
        (mutations, is_replace)
    }
}

impl<T: Snapshot> Snapshot for RangeFrom<T> {
    type Snapshot = RangeFrom<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.start.to_snapshot()..
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.start.eq_snapshot(&snapshot.start)
    }
}

/// Observer implementation for [`RangeInclusive<Idx>`].
pub struct RangeInclusiveObserver<O, S: ?Sized, D = Zero> {
    start: O,
    end: O,
    ptr: Pointer<S>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> QuasiObserver for RangeInclusiveObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        O::invalidate(&mut this.start);
        O::invalidate(&mut this.end);
    }
}

impl<O, S: ?Sized, D> Observer for RangeInclusiveObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeInclusive<O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            start: O::uninit(),
            end: O::uninit(),
            ptr: Pointer::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let value = head.as_deref();
        let mut this = Self {
            start: O::observe(value.start()),
            end: O::observe(value.end()),
            ptr: Pointer::new(head),
            phantom: PhantomData,
        };
        Pointer::register_observer(&mut this.ptr, &mut this.start);
        Pointer::register_observer(&mut this.ptr, &mut this.end);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(this, head);
        let value = head.as_deref();
        unsafe {
            O::refresh(&mut this.start, value.start());
            O::refresh(&mut this.end, value.end());
        }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for RangeInclusiveObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeInclusive<O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized + 'static,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let mutations_start = unsafe { SerializeObserver::flush(&mut this.start) };
        let mutations_end = unsafe { SerializeObserver::flush(&mut this.end) };
        if mutations_start.is_replace() && mutations_end.is_replace() {
            Mutations::replace((*this).untracked_ref())
        } else {
            let mut mutations = Mutations::new();
            mutations.insert("start", mutations_start);
            mutations.insert("end", mutations_end);
            mutations
        }
    }

    unsafe fn flat_flush(this: &mut Self) -> (Mutations, bool) {
        let mutations_start = unsafe { SerializeObserver::flush(&mut this.start) };
        let mutations_end = unsafe { SerializeObserver::flush(&mut this.end) };
        let is_replace = mutations_start.is_replace() && mutations_end.is_replace();
        let mut mutations = Mutations::new();
        mutations.insert("start", mutations_start);
        mutations.insert("end", mutations_end);
        (mutations, is_replace)
    }
}

impl<T: Snapshot> Snapshot for RangeInclusive<T> {
    type Snapshot = (T::Snapshot, T::Snapshot);

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        (self.start().to_snapshot(), self.end().to_snapshot())
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.start().eq_snapshot(&snapshot.0) && self.end().eq_snapshot(&snapshot.1)
    }
}

/// Observer implementation for [`RangeTo<Idx>`].
pub struct RangeToObserver<O, S: ?Sized, D = Zero> {
    /// See [`RangeTo::end`].
    pub end: O,
    ptr: Pointer<S>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> QuasiObserver for RangeToObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
{
    type Head = S;
    type OuterDepth = Succ<Zero>;
    type InnerDepth = D;

    #[inline]
    fn invalidate(this: &mut Self) {
        O::invalidate(&mut this.end);
    }
}

impl<O, S: ?Sized, D> Observer for RangeToObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeTo<O::Head>>,
    O: Observer<InnerDepth = Zero>,
    O::Head: Sized,
{
    #[inline]
    fn uninit() -> Self {
        Self {
            end: O::uninit(),
            ptr: Pointer::uninit(),
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(head: &Self::Head) -> Self {
        let value = head.as_deref();
        let mut this = Self {
            end: O::observe(&value.end),
            ptr: Pointer::new(head),
            phantom: PhantomData,
        };
        Pointer::register_observer(&mut this.ptr, &mut this.end);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, head: &Self::Head) {
        Pointer::set(this, head);
        let value = head.as_deref();
        unsafe {
            O::refresh(&mut this.end, &value.end);
        }
    }
}

impl<O, S: ?Sized, D> SerializeObserver for RangeToObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeTo<O::Head>>,
    O: SerializeObserver<InnerDepth = Zero>,
    O::Head: Serialize + Sized + 'static,
{
    unsafe fn flush(this: &mut Self) -> Mutations {
        let mutations_end = unsafe { SerializeObserver::flush(&mut this.end) };
        if mutations_end.is_replace() {
            Mutations::replace((*this).untracked_ref())
        } else {
            let mut mutations = Mutations::new();
            mutations.insert("end", mutations_end);
            mutations
        }
    }

    unsafe fn flat_flush(this: &mut Self) -> (Mutations, bool) {
        let mutations_end = unsafe { SerializeObserver::flush(&mut this.end) };
        let is_replace = mutations_end.is_replace();
        let mut mutations = Mutations::new();
        mutations.insert("end", mutations_end);
        (mutations, is_replace)
    }
}

impl<T: Snapshot> Snapshot for RangeTo<T> {
    type Snapshot = RangeTo<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        ..self.end.to_snapshot()
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.end.eq_snapshot(&snapshot.end)
    }
}

macro_rules! impl_range {
    ($($ty:ident => $ob:ident, $helper_ref:ident, $helper_mut:ident;)*) => {
        $(
            impl<O, S: ?Sized, D> Deref for $ob<O, S, D> {
                type Target = Pointer<S>;

                #[inline]
                fn deref(&self) -> &Self::Target {
                    &self.ptr
                }
            }

            impl<O, S: ?Sized, D> DerefMut for $ob<O, S, D> {
                #[inline]
                fn deref_mut(&mut self) -> &mut Self::Target {
                    Pointer::invalidate(&mut self.ptr);
                    &mut self.ptr
                }
            }

            impl<O, S: ?Sized, D> Debug for $ob<O, S, D>
            where
                O: QuasiObserver,
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: Debug,
            {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_tuple(stringify!($ob)).field(&self.untracked_ref()).finish()
                }
            }

            impl<O, S: ?Sized, D, U> PartialEq<$ty<U>> for $ob<O, S, D>
            where
                O: QuasiObserver,
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: PartialEq<$ty<U>>,
            {
                #[inline]
                fn eq(&self, other: &$ty<U>) -> bool {
                    self.untracked_ref().eq(other)
                }
            }

            impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<$ob<O2, S2, D2>> for $ob<O1, S1, D1>
            where
                O1: QuasiObserver<Target: Deref<Target: AsDeref<O1::InnerDepth>>>,
                O2: QuasiObserver<Target: Deref<Target: AsDeref<O2::InnerDepth>>>,
                D1: Unsigned,
                D2: Unsigned,
                S1: AsDeref<D1>,
                S2: AsDeref<D2>,
                S1::Target: PartialEq<S2::Target>,
            {
                #[inline]
                fn eq(&self, other: &$ob<O2, S2, D2>) -> bool {
                    self.untracked_ref().eq(other.untracked_ref())
                }
            }

            impl<O, S: ?Sized, D> Eq for $ob<O, S, D>
            where
                O: QuasiObserver,
                D: Unsigned,
                S: AsDeref<D>,
                S::Target: Eq,
            {
            }

            spec_impl_observe!($helper_ref, $ty<Self>, $ty<T>, $ob);
            spec_impl_ref_observe!($helper_mut, $ty<Self>, $ty<T>, $ob);
        )*
    };
}

impl_range! {
    Range => RangeObserver, RangeObserveImpl, RangeRefObserveImpl;
    RangeFrom => RangeFromObserver, RangeFromObserveImpl, RangeFromRefObserveImpl;
    RangeInclusive => RangeInclusiveObserver, RangeInclusiveObserveImpl, RangeInclusiveRefObserveImpl;
    RangeTo => RangeToObserver, RangeToObserveImpl, RangeToRefObserveImpl;
}

#[cfg(test)]
mod tests {
    use morphix_test_utils::*;
    use serde_json::json;

    use super::*;
    use crate::adapter::Json;
    use crate::builtin::GeneralObserver;
    use crate::observe::{ObserveExt, SerializeObserverExt};

    #[test]
    fn range_no_change_returns_none() {
        let mut range = 0..10i32;
        let mut ob = range.__observe();
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, None);
    }

    #[test]
    fn range_deref_triggers_replace() {
        let mut range = 0..10i32;
        let mut ob = range.__observe();
        **ob = 5..15;
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!({"start": 5, "end": 15}))));
    }

    #[test]
    fn range_granular_start_change() {
        let mut range = String::from("a")..String::from("z");
        let mut ob = range.__observe();
        ob.start.push_str("bc");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(start, json!("bc"))));
    }

    #[test]
    fn range_granular_end_change() {
        let mut range = String::from("a")..String::from("z");
        let mut ob = range.__observe();
        ob.end.push_str("yx");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(append!(end, json!("yx"))));
    }

    #[test]
    fn range_both_fields_replace_collapse() {
        let mut range = String::from("a")..String::from("z");
        let mut ob = range.__observe();
        **ob = String::from("b")..String::from("y");
        let Json(mutation) = ob.flush().unwrap();
        assert_eq!(mutation, Some(replace!(_, json!({"start": "b", "end": "y"}))));
    }

    #[test]
    fn range_specialization() {
        let mut range = 0..10i32;
        let ob: GeneralObserver<_, _, _> = range.__observe();
        assert_eq!(format!("{ob:?}"), "SnapshotObserver(0..10)");

        let mut range = String::from("a")..String::from("z");
        let ob: RangeObserver<_, _, _> = range.__observe();
        assert_eq!(format!("{ob:?}"), r#"RangeObserver("a".."z")"#);
    }
}
