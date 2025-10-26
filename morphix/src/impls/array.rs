use std::array::from_fn;
use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use serde::Serialize;

use crate::helper::{AsDerefMut, Assignable, Succ, Unsigned, Zero};
use crate::observe::{DefaultSpec, Observer, ObserverPointer, SerializeObserver};
use crate::{Adapter, Mutation, MutationKind, Observe, PathSegment};

pub struct ArrayObserver<'i, const N: usize, O, S: ?Sized, D = Zero> {
    ptr: ObserverPointer<S>,
    obs: UnsafeCell<[O; N]>,
    is_replaced: bool,
    phantom: PhantomData<&'i mut D>,
}

impl<'i, const N: usize, O, S: ?Sized, D> Default for ArrayObserver<'i, N, O, S, D>
where
    O: Default,
{
    #[inline]
    fn default() -> Self {
        Self {
            ptr: ObserverPointer::default(),
            obs: UnsafeCell::new(from_fn(|_| O::default())),
            is_replaced: false,
            phantom: PhantomData,
        }
    }
}

impl<'i, const N: usize, O, S: ?Sized, D> Deref for ArrayObserver<'i, N, O, S, D> {
    type Target = ObserverPointer<S>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<'i, const N: usize, O, S: ?Sized, D> DerefMut for ArrayObserver<'i, N, O, S, D>
where
    O: Default,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.is_replaced = true;
        self.obs = UnsafeCell::new(from_fn(|_| O::default()));
        &mut self.ptr
    }
}

impl<'i, const N: usize, O, S> Assignable for ArrayObserver<'i, N, O, S>
where
    O: Default,
{
    type Depth = Succ<Zero>;
}

impl<'i, const N: usize, O, S: ?Sized, D, T> Observer<'i> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'i,
    O: Observer<'i, InnerDepth = Zero, Head = T>,
{
    type InnerDepth = D;
    type OuterDepth = Zero;
    type Head = S;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            ptr: ObserverPointer::new(value),
            obs: UnsafeCell::new(from_fn(|_| O::default())),
            is_replaced: false,
            phantom: PhantomData,
        }
    }
}

impl<'i, const N: usize, O, S: ?Sized, D, T> SerializeObserver<'i> for ArrayObserver<'i, N, O, S, D>
where
    D: Unsigned,
    S: AsDerefMut<D, Target = [T; N]> + 'i,
    O: SerializeObserver<'i, InnerDepth = Zero, Head = T>,
    T: Serialize,
{
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A>>, A::Error> {
        let mut mutations = vec![];
        if this.is_replaced {
            mutations.push(Mutation {
                path: Default::default(),
                kind: MutationKind::Replace(A::serialize_value(this.as_deref().as_ref())?),
            });
        } else {
            let obs = unsafe { &mut *this.obs.get() };
            for (index, ob) in obs.iter_mut().enumerate() {
                if let Some(mut mutation) = SerializeObserver::collect::<A>(ob)? {
                    mutation.path.push(PathSegment::PosIndex(index));
                    mutations.push(mutation);
                }
            }
        }
        Ok(Mutation::coalesce(mutations))
    }
}

impl<T, const N: usize> Observe for [T; N]
where
    T: Observe + ArrayObserveImpl<T, N, T::Spec>,
{
    type Observer<'i, S, D>
        = <T as ArrayObserveImpl<T, N, T::Spec>>::Observer<'i, S, D>
    where
        Self: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i;

    type Spec = T::Spec;
}

/// Helper trait for selecting appropriate observer implementations for [`[T; N]`](std::array).
#[doc(hidden)]
pub trait ArrayObserveImpl<T: Observe, const N: usize, Spec> {
    /// The observer type for [`[T; N]`](std::array) with the given specification.
    type Observer<'i, S, D>: Observer<'i, Head = S, InnerDepth = D>
    where
        T: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = [T; N]> + ?Sized + 'i;
}

impl<T, const N: usize> ArrayObserveImpl<T, N, DefaultSpec> for T
where
    T: Observe<Spec = DefaultSpec>,
{
    type Observer<'i, S, D>
        = ArrayObserver<'i, N, T::Observer<'i, T, Zero>, S, D>
    where
        T: 'i,
        D: Unsigned,
        S: AsDerefMut<D, Target = [T; N]> + ?Sized + 'i;
}
