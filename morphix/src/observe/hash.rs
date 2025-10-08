use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use serde::Serialize;

use crate::{Adapter, Mutation, MutationKind, Observer};

/// An observer that uses hashing for efficient change detection.
///
/// `HashObserver` computes a hash of the initial value and compares it with the hash of the final
/// value to detect changes. This is more efficient than full value comparison for large structures,
/// though it cannot detect the specific nature of the change.
///
/// ## Use Cases
///
/// This observer is ideal for:
/// - Large structures where full comparison is expensive
/// - Types that implement `Hash` but not `Clone` or where cloning is expensive
/// - Scenarios where you only need to know if something changed, not what changed
/// - Configuration objects with many fields
///
/// ## Limitations
///
/// - Only produces `Replace` mutations (cannot detect `Append` operations)
/// - Hash collisions are theoretically possible (though extremely rare)
/// - Requires recomputing the hash on collection
///
/// ## Example
///
/// ```
/// use std::collections::HashMap;
/// use morphix::{Observe, Observer, JsonAdapter};
///
/// #[derive(Serialize, Hash, Observe)]
/// struct LargeConfig {
///     #[observe(hash)]
///     data: Vec<u8>,  // Large binary data
/// }
///
/// let mut config = LargeConfig {
///     data: vec![0; 1024],
/// };
///
/// let mutation = observe!(JsonAdapter, |mut config| {
///     config.data[0] = 1;  // Modify the data
/// }).unwrap();
///
/// // Efficiently detected change without cloning the entire Vec
/// assert!(mutation.is_some());
/// ```
pub struct HashObserver<'i, T, H = DefaultHasher> {
    ptr: *mut T,
    initial_hash: u64,
    phantom: PhantomData<(&'i mut T, H)>,
}

impl<'i, T: Hash, H: Hasher + Default> HashObserver<'i, T, H> {
    fn hash(value: &T) -> u64 {
        let mut hasher = H::default();
        value.hash(&mut hasher);
        hasher.finish()
    }
}

impl<'i, T: Hash, H: Hasher + Default> Observer<'i> for HashObserver<'i, T, H> {
    fn observe(value: &'i mut T) -> Self {
        Self {
            ptr: value as *mut T,
            initial_hash: Self::hash(value),
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(this: Self) -> Result<Option<Mutation<A>>, A::Error>
    where
        T: Serialize,
    {
        Ok(if this.initial_hash != Self::hash(&*this) {
            Some(Mutation {
                path_rev: vec![],
                operation: MutationKind::Replace(A::serialize_value(&*this)?),
            })
        } else {
            None
        })
    }
}

impl<'i, T, H> Deref for HashObserver<'i, T, H> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T, H> DerefMut for HashObserver<'i, T, H> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<'i, T: Index<U>, H, U> Index<U> for HashObserver<'i, T, H> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        (**self).index(index)
    }
}

impl<'i, T: IndexMut<U>, H, U> IndexMut<U> for HashObserver<'i, T, H> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        (**self).index_mut(index)
    }
}

impl<'i, T: PartialEq<U>, H, U: ?Sized> PartialEq<U> for HashObserver<'i, T, H> {
    fn eq(&self, other: &U) -> bool {
        (**self).eq(other)
    }
}

impl<'i, T: PartialOrd<U>, H, U: ?Sized> PartialOrd<U> for HashObserver<'i, T, H> {
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, H, U> ::std::ops::$trait<U> for HashObserver<'i, T, H> {
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
            impl<'i, T: Copy + ::std::ops::$trait<U>, H, U> ::std::ops::$trait<U> for HashObserver<'i, T, H> {
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
