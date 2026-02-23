use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::builtin::Snapshot;
use crate::helper::macros::{spec_impl_observe_from_ref, spec_impl_ref_observe};
use crate::helper::{AsDeref, AsDerefMut, AsNormalized, Pointer, Succ, Unsigned, Zero};
use crate::observe::{Observer, SerializeObserver};
use crate::{Adapter, MutationKind, Mutations};

trait Weak<T: ?Sized> {
    type Ptr: Deref<Target = T>;

    fn upgrade(&self) -> Option<Self::Ptr>;
}

impl<T: ?Sized> Weak<T> for std::rc::Weak<T> {
    type Ptr = std::rc::Rc<T>;

    #[inline]
    fn upgrade(&self) -> Option<Self::Ptr> {
        self.upgrade()
    }
}

impl<T: ?Sized> Weak<T> for std::sync::Weak<T> {
    type Ptr = std::sync::Arc<T>;

    #[inline]
    fn upgrade(&self) -> Option<Self::Ptr> {
        self.upgrade()
    }
}

/// Observer implementation for [`std::rc::Weak<T>`] and [`std::sync::Weak<T>`].
pub struct WeakObserver<O, S: ?Sized, D> {
    ptr: Pointer<S>,
    mutated: bool,
    initial: bool,
    inner: Option<O>,
    phantom: PhantomData<D>,
}

impl<O, S: ?Sized, D> Deref for WeakObserver<O, S, D> {
    type Target = Pointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<O, S: ?Sized, D> DerefMut for WeakObserver<O, S, D> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutated = true;
        self.inner = None;
        &mut self.ptr
    }
}

impl<O, S: ?Sized, D> AsNormalized for WeakObserver<O, S, D> {
    type OuterDepth = Succ<Zero>;
}

impl<'ob, O, S: ?Sized, D> Observer<'ob> for WeakObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target: Weak<O::Head>> + 'ob,
    O: Observer<'ob, InnerDepth = Zero>,
{
    type InnerDepth = D;
    type Head = S;

    #[inline]
    fn uninit() -> Self {
        Self {
            ptr: Pointer::uninit(),
            mutated: false,
            initial: false,
            inner: None,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn observe(value: &Self::Head) -> Self {
        let ptr = Pointer::new(value);
        let rc = value.as_deref().upgrade();
        Self {
            ptr,
            mutated: false,
            initial: rc.is_some(),
            inner: rc.map(|ptr| O::observe(&*ptr)),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &Self::Head) {
        if let Some(inner) = &mut this.inner
            && let Some(ptr) = value.as_deref().upgrade()
        {
            unsafe { O::refresh(inner, &*ptr) }
        }
    }
}

impl<'ob, O, S: ?Sized, D> SerializeObserver<'ob> for WeakObserver<O, S, D>
where
    D: Unsigned,
    S: AsDeref<D, Target: Weak<O::Head>> + 'ob,
    O: SerializeObserver<'ob, InnerDepth = Zero>,
    O::Head: Serialize,
{
    #[inline]
    unsafe fn flush_unchecked<A: Adapter>(this: &mut Self) -> Result<Mutations<A::Value>, A::Error> {
        let rc = (*this.ptr).as_deref().upgrade();
        let initial = this.initial;
        this.initial = rc.is_some();
        if !this.mutated {
            if let Some(ob) = &mut this.inner {
                return SerializeObserver::flush::<A>(ob);
            } else {
                return Ok(Mutations::new());
            }
        }
        this.mutated = false;
        if initial || rc.is_some() {
            Ok(MutationKind::Replace(A::serialize_value(&rc.as_deref())?).into())
        } else {
            Ok(Mutations::new())
        }
    }
}

impl<O, S: ?Sized, D> Debug for WeakObserver<O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D>,
    S::Target: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WeakObserver")
            .field(&AsDeref::<D>::as_deref(&*self.ptr))
            .finish()
    }
}

spec_impl_observe_from_ref!(WeakObserveImpl, std::rc::Weak<Self>, std::rc::Weak<T>, WeakObserver);
spec_impl_ref_observe!(WeakRefObserveImpl, std::rc::Weak<Self>, std::rc::Weak<T>);

impl<T: Snapshot + ?Sized> Snapshot for std::rc::Weak<T> {
    type Snapshot = Option<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.upgrade().map(|v| v.to_snapshot())
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.upgrade()
            .zip(snapshot.as_ref())
            .is_some_and(|(v, snapshot)| v.eq_snapshot(snapshot))
    }
}

impl<T: Snapshot + ?Sized> Snapshot for std::sync::Weak<T> {
    type Snapshot = Option<T::Snapshot>;

    #[inline]
    fn to_snapshot(&self) -> Self::Snapshot {
        self.upgrade().map(|v| v.to_snapshot())
    }

    #[inline]
    fn eq_snapshot(&self, snapshot: &Self::Snapshot) -> bool {
        self.upgrade()
            .zip(snapshot.as_ref())
            .is_some_and(|(v, snapshot)| v.eq_snapshot(snapshot))
    }
}
