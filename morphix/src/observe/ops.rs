use std::mem::take;
use std::ops::{
    AddAssign, BitAndAssign, BitOrAssign, BitXorAssign, Deref, DerefMut, DivAssign, Index, IndexMut, MulAssign,
    RemAssign, ShlAssign, ShrAssign, SubAssign,
};

use crate::{Ob, Operation};

impl<'i, T, U: Default> Deref for Ob<'i, T, U> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        Self::get(self)
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

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: $trait<U>, U> $trait<U> for Ob<'i, T> {
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

impl<'i, T: Index<U>, U> Index<U> for Ob<'i, T> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        Self::get(self).index(index)
    }
}

impl<'i, T: IndexMut<U>, U> IndexMut<U> for Ob<'i, T> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        Self::record(self, Operation::Replace(()));
        Self::get_mut(self).index_mut(index)
    }
}
