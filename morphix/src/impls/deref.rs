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
pub struct DerefObserver<'ob, O> {
    inner: O,
    phantom: PhantomData<&'ob mut ()>,
}

impl<'ob, O> Deref for DerefObserver<'ob, O> {
    type Target = O;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'ob, O> DerefMut for DerefObserver<'ob, O> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'ob, O> Assignable for DerefObserver<'ob, O>
where
    O: Observer<'ob>,
{
    type Depth = Succ<O::OuterDepth>;
}

impl<'ob, O, D> Observer<'ob> for DerefObserver<'ob, O>
where
    O: Observer<'ob, InnerDepth = Succ<D>>,
    O::Head: AsDerefMut<D>,
    D: Unsigned,
{
    type OuterDepth = Succ<O::OuterDepth>;
    type InnerDepth = D;
    type Head = O::Head;

    #[inline]
    fn observe(value: &'ob mut Self::Head) -> Self {
        Self {
            inner: O::observe(value),
            phantom: PhantomData,
        }
    }

    #[inline]
    unsafe fn refresh(this: &mut Self, value: &mut Self::Head) {
        unsafe { O::refresh(&mut this.inner, value) }
    }
}

impl<'ob, O, D> SerializeObserver<'ob> for DerefObserver<'ob, O>
where
    O: SerializeObserver<'ob, InnerDepth = Succ<D>>,
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
            impl<'ob, O> std::fmt::$trait for DerefObserver<'ob, O>
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

impl<'ob, O> Debug for DerefObserver<'ob, O>
where
    O: Debug,
{
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DerefObserver").field(&self.inner).finish()
    }
}

impl<'ob, O, D, T: ?Sized, U: ?Sized> PartialEq<U> for DerefObserver<'ob, O>
where
    O: Observer<'ob, InnerDepth = Succ<D>>,
    O::Head: AsDerefMut<D, Target = T>,
    D: Unsigned,
    T: PartialEq<U>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        AsDeref::<D>::as_deref(&**O::as_ptr(self)).eq(other)
    }
}

impl<'ob, O, D, T: ?Sized, U: ?Sized> PartialOrd<U> for DerefObserver<'ob, O>
where
    O: Observer<'ob, InnerDepth = Succ<D>>,
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
    type Observer<'ob, S, D>
        = DerefObserver<'ob, U::Observer<'ob, S, Succ<D>>>
    where
        Self: 'ob,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob,
        D: Unsigned;

    type Spec = DefaultSpec;
}

impl<U> Observe for &mut U
where
    U: Observe,
{
    type Observer<'ob, S, D>
        = DerefObserver<'ob, U::Observer<'ob, S, Succ<D>>>
    where
        Self: 'ob,
        S: AsDerefMut<D, Target = Self> + ?Sized + 'ob,
        D: Unsigned;

    type Spec = DefaultSpec;
}
