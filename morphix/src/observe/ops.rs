use std::mem::take;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::observe::ObInner;
use crate::{Ob, Observer};

impl<'i, T, U: ObInner> Deref for Ob<'i, T, U> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get_ref()
    }
}

impl<'i, T, U: ObInner> DerefMut for Ob<'i, T, U> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mark_replace();
        take(&mut self.inner);
        self.get_mut()
    }
}

impl<'i, T: Index<U>, U> Index<U> for Ob<'i, T> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        self.get_ref().index(index)
    }
}

impl<'i, T: IndexMut<U>, U> IndexMut<U> for Ob<'i, T> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        self.mark_replace();
        self.get_mut().index_mut(index)
    }
}

impl<'i, T: PartialEq<U>, U: ?Sized> PartialEq<U> for Ob<'i, T> {
    fn eq(&self, other: &U) -> bool {
        self.get_ref().eq(other)
    }
}

impl<'i, T: PartialOrd<U>, U: ?Sized> PartialOrd<U> for Ob<'i, T> {
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        self.get_ref().partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for Ob<'i, T> {
                fn $method(&mut self, rhs: U) {
                    self.mark_replace();
                    self.get_mut().$method(rhs);
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
