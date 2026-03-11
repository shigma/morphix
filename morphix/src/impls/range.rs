use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Range, RangeFrom, RangeInclusive, RangeTo};

use serde::Serialize;

use crate::Mutations;
use crate::builtin::Snapshot;
use crate::helper::macros::{spec_impl_observe, spec_impl_observe_from_ref, spec_impl_ref_observe};
use crate::helper::{AsDeref, AsDerefMut, Pointer, QuasiObserver, Succ, Unsigned, Zero};
use crate::observe::{Observer, RefObserver, SerializeObserver};

macro_rules! impl_range {
    ($($ty:ident ($($field:ident),* $(,)?) => $ob:ident, $helper_ref:ident, $helper_mut:ident;)*) => {
        $(
            /// Observer implementation for [`Range<Idx>`].
            #[doc = concat!("Observer implementation for [`", stringify!($ty), "<Idx>`].")]
            pub struct $ob<O, S: ?Sized, D = Zero> {
                $(
                    #[doc = concat!("See [`", stringify!($ty), "::", stringify!($field), "`].")]
                    pub $field: O,
                )*
                ptr: Pointer<S>,
                phantom: PhantomData<D>,
            }

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
                    std::ptr::from_mut(self).expose_provenance();
                    Pointer::invalidate(&mut self.ptr);
                    &mut self.ptr
                }
            }

            impl<O, S: ?Sized, D> QuasiObserver for $ob<O, S, D>
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
                    $(O::invalidate(&mut this.$field);)*
                }
            }

            impl<O, S: ?Sized, D> Observer for $ob<O, S, D>
            where
                D: Unsigned,
                S: AsDerefMut<D, Target = $ty<O::Head>>,
                O: Observer<InnerDepth = Zero>,
                O::Head: Sized,
            {
                #[inline]
                fn uninit() -> Self {
                    Self {
                        ptr: Pointer::uninit(),
                        $($field: O::uninit(),)*
                        phantom: PhantomData,
                    }
                }

                #[inline]
                fn observe(mut head: &mut Self::Head) -> Self {
                    let ptr = Pointer::new(&mut head);
                    let value = head.as_deref_mut();
                    let mut this = Self {
                        ptr,
                        $($field: O::observe(&mut value.$field),)*
                        phantom: PhantomData,
                    };
                    $(Pointer::register_observer(&mut this.ptr, &mut this.$field);)*
                    this
                }

                #[inline]
                unsafe fn refresh(this: &mut Self, mut head: &mut Self::Head) {
                    Pointer::set(this, &mut head);
                    let value = head.as_deref_mut();
                    unsafe {
                        $(O::refresh(&mut this.$field, &mut value.$field);)*
                    }
                }
            }

            impl<O, S: ?Sized, D> RefObserver for $ob<O, S, D>
            where
                D: Unsigned,
                S: AsDeref<D, Target = $ty<O::Head>>,
                O: RefObserver<InnerDepth = Zero>,
                O::Head: Sized,
            {
                #[inline]
                fn uninit() -> Self {
                    Self {
                        $($field: O::uninit(),)*
                        ptr: Pointer::uninit(),
                        phantom: PhantomData,
                    }
                }

                #[inline]
                fn observe(head: &Self::Head) -> Self {
                    let value = head.as_deref();
                    let mut this = Self {
                        $($field: O::observe(&value.$field),)*
                        ptr: Pointer::new(head),
                        phantom: PhantomData,
                    };
                    $(Pointer::register_observer(&mut this.ptr, &mut this.$field);)*
                    this
                }

                #[inline]
                unsafe fn refresh(this: &mut Self, head: &Self::Head) {
                    Pointer::set(this, head);
                    let value = head.as_deref();
                    unsafe {
                        $(O::refresh(&mut this.$field, &value.$field);)*
                    }
                }
            }

            impl<O, S: ?Sized, D> SerializeObserver for $ob<O, S, D>
            where
                D: Unsigned,
                S: AsDeref<D, Target = $ty<O::Head>>,
                O: SerializeObserver<InnerDepth = Zero>,
                O::Head: Serialize + Sized + 'static,
            {
                unsafe fn flush(this: &mut Self) -> Mutations {
                    $(
                        let $field = unsafe { SerializeObserver::flush(&mut this.$field) };
                    )*
                    if $($field.is_replace())&&* {
                        Mutations::replace((*this).untracked_ref())
                    } else {
                        let mut mutations = Mutations::new();
                        $(mutations.insert(stringify!($field), $field);)*
                        mutations
                    }
                }

                unsafe fn flat_flush(this: &mut Self) -> (Mutations, bool) {
                    $(
                        let $field = unsafe { SerializeObserver::flush(&mut this.$field) };
                    )*
                    let is_replace = $($field.is_replace())&&*;
                    let mut mutations = Mutations::new();
                    $(mutations.insert(stringify!($field), $field);)*
                    (mutations, is_replace)
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

            impl<T: Snapshot> Snapshot for $ty<T> {
                type Snapshot = $ty<T::Snapshot>;

                #[inline]
                fn to_snapshot(&self) -> Self::Snapshot {
                    $ty {
                        $($field: self.$field.to_snapshot(),)*
                    }
                }

                #[inline]
                fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
                    $(self.$field.eq_snapshot(&snapshot.$field))&&*
                }
            }
        )*
    };
}

impl_range! {
    Range (start, end) => RangeObserver, RangeObserveImpl, RangeRefObserveImpl;
    RangeFrom (start) => RangeFromObserver, RangeFromObserveImpl, RangeFromRefObserveImpl;
    RangeTo (end) => RangeToObserver, RangeToObserveImpl, RangeToRefObserveImpl;
}

/// Observer implementation for [`RangeInclusive<Idx>`].
pub struct RangeInclusiveObserver<O, S: ?Sized, D = Zero> {
    start: O,
    end: O,
    ptr: Pointer<S>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> Deref for RangeInclusiveObserver<O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<O, S: ?Sized, D> DerefMut for RangeInclusiveObserver<O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        std::ptr::from_mut(self).expose_provenance();
        Pointer::invalidate(&mut self.ptr);
        &mut self.ptr
    }
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
    O: RefObserver<InnerDepth = Zero>,
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
    fn observe(mut head: &mut Self::Head) -> Self {
        let ptr = Pointer::new(&mut head);
        let value = (*head).as_deref();
        let mut this = Self {
            ptr,
            start: O::observe(value.start()),
            end: O::observe(value.end()),
            phantom: PhantomData,
        };
        Pointer::register_observer(&mut this.ptr, &mut this.start);
        Pointer::register_observer(&mut this.ptr, &mut this.end);
        this
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, mut head: &mut Self::Head) {
        Pointer::set(this, &mut head);
        let value = (*head).as_deref();
        unsafe {
            O::refresh(&mut this.start, value.start());
            O::refresh(&mut this.end, value.end());
        }
    }
}

impl<O, S: ?Sized, D> RefObserver for RangeInclusiveObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target = RangeInclusive<O::Head>>,
    O: RefObserver<InnerDepth = Zero>,
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
            ptr: Pointer::new(head),
            start: O::observe(value.start()),
            end: O::observe(value.end()),
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

impl<O, S: ?Sized, D> Debug for RangeInclusiveObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RangeInclusiveObserver")
            .field(&self.untracked_ref())
            .finish()
    }
}

impl<O, S: ?Sized, D, U> PartialEq<RangeInclusive<U>> for RangeInclusiveObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: PartialEq<RangeInclusive<U>>,
{
    #[inline]
    fn eq(&self, other: &RangeInclusive<U>) -> bool {
        self.untracked_ref().eq(other)
    }
}

impl<O1, O2, S1: ?Sized, S2: ?Sized, D1, D2> PartialEq<RangeInclusiveObserver<O2, S2, D2>>
    for RangeInclusiveObserver<O1, S1, D1>
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
    fn eq(&self, other: &RangeInclusiveObserver<O2, S2, D2>) -> bool {
        self.untracked_ref().eq(other.untracked_ref())
    }
}

impl<O, S: ?Sized, D> Eq for RangeInclusiveObserver<O, S, D>
where
    O: QuasiObserver,
    D: Unsigned,
    S: AsDeref<D>,
    S::Target: Eq,
{
}

spec_impl_observe_from_ref!(
    RangeInclusiveObserveImpl,
    RangeInclusive<Self>,
    RangeInclusive<T>,
    RangeInclusiveObserver
);

spec_impl_ref_observe!(
    RangeInclusiveRefObserveImpl,
    RangeInclusive<Self>,
    RangeInclusive<T>,
    RangeInclusiveObserver
);

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
