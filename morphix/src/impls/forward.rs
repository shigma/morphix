use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use crate::helper::{AsDeref, AsDerefMut, Assignable, Succ, Unsigned};
use crate::observe::{DefaultSpec, Observer, SerializeObserver};
use crate::{Adapter, Mutation, Observe};

/// Observer implementation for pointer types such as [`Box<T>`] and `&mut T`.
///
/// This observer wraps the inner type's observer and forwards all operations to it, maintaining
/// proper dereference chains for pointer types.
#[derive(Default)]
pub struct ForwardObserver<'i, O> {
    inner: O,
    phantom: PhantomData<&'i mut ()>,
}

impl<'i, O> Deref for ForwardObserver<'i, O> {
    type Target = O;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'i, O> DerefMut for ForwardObserver<'i, O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'i, O> Assignable for ForwardObserver<'i, O>
where
    O: Observer<'i>,
{
    type Depth = Succ<O::OuterDepth>;
}

impl<'i, O, D> Observer<'i> for ForwardObserver<'i, O>
where
    O: Observer<'i, InnerDepth = Succ<D>>,
    O::Head: AsDerefMut<D>,
    D: Unsigned,
{
    type OuterDepth = Succ<O::OuterDepth>;
    type InnerDepth = D;
    type Head = O::Head;

    #[inline]
    fn observe(value: &'i mut Self::Head) -> Self {
        Self {
            inner: O::observe(value),
            phantom: PhantomData,
        }
    }
}

impl<'i, O, D> SerializeObserver<'i> for ForwardObserver<'i, O>
where
    O: SerializeObserver<'i, InnerDepth = Succ<D>>,
    O::Head: AsDerefMut<D>,
    D: Unsigned,
{
    #[inline]
    unsafe fn collect_unchecked<A: Adapter>(this: &mut Self) -> Result<Option<Mutation<A::Value>>, A::Error> {
        unsafe { O::collect_unchecked::<A>(&mut this.inner) }
    }
}

macro_rules! impl_fmt {
    ($($trait:ident),* $(,)?) => {
        $(
            impl<'i, O> std::fmt::$trait for ForwardObserver<'i, O>
            where
                O: std::fmt::$trait,
            {
                #[inline]
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    std::fmt::$trait::fmt(&self.inner, f)
                }
            }
        )*
    };
}

impl_fmt! {
    Binary,
    Display,
    LowerExp,
    LowerHex,
    Octal,
    Pointer,
    UpperExp,
    UpperHex,
}

impl<'i, O> Debug for ForwardObserver<'i, O>
where
    O: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoxObserver").field(&self.inner).finish()
    }
}

impl<'i, O, D, T: ?Sized, U: ?Sized> PartialEq<U> for ForwardObserver<'i, O>
where
    O: Observer<'i, InnerDepth = Succ<D>>,
    O::Head: AsDerefMut<D, Target = T>,
    D: Unsigned,
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        AsDeref::<D>::as_deref(&**O::as_ptr(self)).eq(other)
    }
}

impl<'i, O, D, T: ?Sized, U: ?Sized> PartialOrd<U> for ForwardObserver<'i, O>
where
    O: Observer<'i, InnerDepth = Succ<D>>,
    O::Head: AsDerefMut<D, Target = T>,
    D: Unsigned,
    T: PartialOrd<U>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        AsDeref::<D>::as_deref(&**O::as_ptr(self)).partial_cmp(other)
    }
}

impl<U> Observe for Box<U>
where
    U: Observe,
{
    type Observer<'i, S, D>
        = ForwardObserver<'i, U::Observer<'i, S, Succ<D>>>
    where
        Self: 'i,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i,
        D: Unsigned;

    type Spec = DefaultSpec;
}

impl<U> Observe for &mut U
where
    U: Observe,
{
    type Observer<'i, S, D>
        = ForwardObserver<'i, U::Observer<'i, S, Succ<D>>>
    where
        Self: 'i,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'i,
        D: Unsigned;

    type Spec = DefaultSpec;
}
