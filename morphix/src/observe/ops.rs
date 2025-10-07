use std::mem::take;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::{Ob, Observer, Operation};

impl<'i, T, U: Default> Deref for Ob<'i, T, U> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        Self::get_ref(self)
    }
}

impl<'i, T, U: Default> DerefMut for Ob<'i, T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Self::record(self, Operation::Replace(()));
        take(&mut self.inner);
        take(&mut self.ctx);
        Self::get_mut(self)
    }
}

impl<'i, T: Index<U>, U> Index<U> for Ob<'i, T> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        Self::get_ref(self).index(index)
    }
}

impl<'i, T: IndexMut<U>, U> IndexMut<U> for Ob<'i, T> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        Self::record(self, Operation::Replace(()));
        Self::get_mut(self).index_mut(index)
    }
}

impl<'i, T: PartialEq<U>, U: ?Sized> PartialEq<U> for Ob<'i, T> {
    fn eq(&self, other: &U) -> bool {
        Self::get_ref(self).eq(other)
    }
}

impl<'i, T: PartialOrd<U>, U: ?Sized> PartialOrd<U> for Ob<'i, T> {
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        Self::get_ref(self).partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for Ob<'i, T> {
                fn $method(&mut self, rhs: U) {
                    Self::record(self, Operation::Replace(()));
                    Self::get_mut(self).$method(rhs);
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

macro_rules! impl_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for Ob<'i, T> {
                type Output = T::Output;
                fn $method(self, rhs: U) -> Self::Output {
                    Self::into_inner(self).$method(rhs)
                }
            }
        )*
    };
}

impl_ops! {
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
