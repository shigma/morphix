use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use crate::{Adapter, Change, Observe, Observer, Operation};

pub struct RawOb<'i, T> {
    ptr: *mut T,
    replaced: bool,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T> Observer<'i, T> for RawOb<'i, T> {
    #[inline]
    fn observe(value: &'i mut T) -> Self {
        RawOb::new(value)
    }

    fn collect<A: Adapter>(this: &mut Self) -> Result<Option<Change<A>>, A::Error>
    where
        T: Observe,
    {
        Ok(if this.replaced {
            Some(Change {
                path_rev: vec![],
                operation: Operation::Replace(A::new_replace(&**this)?),
            })
        } else {
            None
        })
    }
}

impl<'i, T> RawOb<'i, T> {
    pub fn new(value: &'i mut T) -> Self {
        Self {
            ptr: value as *mut T,
            replaced: false,
            phantom: PhantomData,
        }
    }
}

macro_rules! impl_observe {
    ($($ty:ty $(=> $target:ty)?),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Observer<'i> = RawOb<'i, $ty>
                where
                    Self: 'i;
            }
        )*
    };
}

impl_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool,
}

impl<'i, T> Deref for RawOb<'i, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T> DerefMut for RawOb<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.replaced = true;
        unsafe { &mut *self.ptr }
    }
}

impl<'i, T: Index<U>, U> Index<U> for RawOb<'i, T> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        (**self).index(index)
    }
}

impl<'i, T: IndexMut<U>, U> IndexMut<U> for RawOb<'i, T> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        (**self).index_mut(index)
    }
}

impl<'i, T: PartialEq<U>, U: ?Sized> PartialEq<U> for RawOb<'i, T> {
    fn eq(&self, other: &U) -> bool {
        (**self).eq(other)
    }
}

impl<'i, T: PartialOrd<U>, U: ?Sized> PartialOrd<U> for RawOb<'i, T> {
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for RawOb<'i, T> {
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
            impl<'i, T: Copy + ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for RawOb<'i, T> {
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
