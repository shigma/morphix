use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use serde::Serialize;

use crate::{Adapter, Mutation, MutationKind, Observer};

pub trait GeneralHandler<T> {
    /// Called when observation begins.
    fn on_observe(value: &mut T) -> Self;

    /// Called when the value is accessed through [DerefMut].
    fn on_deref_mut(&mut self);

    /// Called when collecting changes, returns whether a change occurred.
    ///
    /// The actual serialization is handled by [GeneralObserver].
    fn on_collect(&self, value: &T) -> bool;
}

pub struct GeneralObserver<'i, T, H> {
    ptr: *mut T,
    handler: H,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T, H: GeneralHandler<T>> Observer<'i> for GeneralObserver<'i, T, H> {
    fn observe(value: &'i mut T) -> Self {
        Self {
            ptr: value as *mut T,
            handler: H::on_observe(value),
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(this: Self) -> Result<Option<Mutation<A>>, A::Error>
    where
        T: Serialize,
    {
        Ok(if this.handler.on_collect(&*this) {
            Some(Mutation {
                path_rev: vec![],
                operation: MutationKind::Replace(A::serialize_value(&*this)?),
            })
        } else {
            None
        })
    }
}

impl<'i, T, H> Deref for GeneralObserver<'i, T, H> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T, H> DerefMut for GeneralObserver<'i, T, H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<'i, T: Index<U>, H, U> Index<U> for GeneralObserver<'i, T, H> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        (**self).index(index)
    }
}

impl<'i, T: IndexMut<U>, H, U> IndexMut<U> for GeneralObserver<'i, T, H> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        (**self).index_mut(index)
    }
}

impl<'i, T: PartialEq<U>, H, U: ?Sized> PartialEq<U> for GeneralObserver<'i, T, H> {
    fn eq(&self, other: &U) -> bool {
        (**self).eq(other)
    }
}

impl<'i, T: PartialOrd<U>, H, U: ?Sized> PartialOrd<U> for GeneralObserver<'i, T, H> {
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, H, U> ::std::ops::$trait<U> for GeneralObserver<'i, T, H> {
                fn $method(&mut self, rhs: U) {
                    (**self).$method(rhs);
                }
            }
        )*
    };
}

impl_assign_ops! {
    AddAssign => add_assign,
    SubAssign => sub_assign,
    MulAssign => mul_assign,
    DivAssign => div_assign,
    RemAssign => rem_assign,
    BitAndAssign => bitand_assign,
    BitOrAssign => bitor_assign,
    BitXorAssign => bitxor_assign,
    ShlAssign => shl_assign,
    ShrAssign => shr_assign,
}

macro_rules! impl_ops_copy {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: Copy + ::std::ops::$trait<U>, H, U> ::std::ops::$trait<U> for GeneralObserver<'i, T, H> {
                type Output = T::Output;
                fn $method(self, rhs: U) -> Self::Output {
                    (*self).$method(rhs)
                }
            }
        )*
    };
}

impl_ops_copy! {
    Add => add,
    Sub => sub,
    Mul => mul,
    Div => div,
    Rem => rem,
    BitAnd => bitand,
    BitOr => bitor,
    BitXor => bitxor,
    Shl => shl,
    Shr => shr,
}
