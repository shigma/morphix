use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};

use serde::Serialize;

use crate::{Adapter, Mutation, MutationKind, Observe, Observer};

/// An observer that detects changes by comparing snapshots.
///
/// Unlike [`ShallowObserver`](super::ShallowObserver) which tracks any
/// [`DerefMut`](std::ops::DerefMut) access as a mutation, `SnapshotObserver` creates an initial
/// snapshot of the value and only reports mutation if the final value actually differs from the
/// snapshot.
///
/// This observer is ideal for:
/// - Small, cheaply cloneable types (e.g., `Uuid`, `DateTime`, small enums)
/// - Types where [`DerefMut`] might be called without actual modification
/// - Cases where you only care about actual value changes, not access patterns
///
/// ## Requirements
///
/// The observed type must implement:
/// - [`Clone`] - for creating the snapshot (should be cheap)
/// - [`PartialEq`] - for comparing the final value with the snapshot
/// - [`Serialize`] - for generating the mutation
///
/// ## Example
///
/// ```
/// use morphix::{JsonAdapter, Observe, Observer, observe};
/// use serde::Serialize;
/// use uuid::Uuid;
///
/// #[derive(Clone, PartialEq, Serialize, Observe)]
/// struct Config {
///     #[observe(snapshot)]
///     id: Uuid,
///     #[observe(snapshot)]
///     status: Status,
/// }
///
/// #[derive(Clone, PartialEq, Serialize)]
/// enum Status {
///     Active,
///     Inactive,
/// }
///
/// let mut config = Config {
///     id: Uuid::new_v4(),
///     status: Status::Active,
/// };
///
/// let mutation = observe!(JsonAdapter, |mut config| {
///     // `DerefMut` is called but value doesn't change
///     config.status = Status::Active;
/// }).unwrap();
///
/// assert_eq!(mutation, None); // No mutation because value didn't change
/// ```
///
/// ## Performance Considerations
///
/// SnapshotObserver is most efficient when:
/// - The type is cheap to clone (e.g., [`Copy`] types, small structs)
/// - The type is cheap to compare (e.g., simple equality checks)
/// - Changes are relatively rare compared to access
///
/// For large or expensive-to-clone types, consider using [ShallowObserver](super::ShallowObserver)
/// or implementing a custom [Observe] trait.
pub struct SnapshotObserver<'i, T> {
    ptr: *mut T,
    snapshot: T,
    phantom: PhantomData<&'i mut T>,
}

impl<'i, T: Clone + PartialEq> Observer<'i> for SnapshotObserver<'i, T> {
    #[inline]
    fn observe(value: &'i mut T) -> Self {
        Self {
            ptr: value as *mut T,
            snapshot: value.clone(),
            phantom: PhantomData,
        }
    }

    fn collect<A: Adapter>(this: Self) -> Result<Option<Mutation<A>>, A::Error>
    where
        T: Serialize,
    {
        Ok(if this.snapshot != *this {
            Some(Mutation {
                path_rev: vec![],
                operation: MutationKind::Replace(A::serialize_value(&*this)?),
            })
        } else {
            None
        })
    }
}

impl<'i, T> Deref for SnapshotObserver<'i, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<'i, T> DerefMut for SnapshotObserver<'i, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<'i, T: Index<U>, U> Index<U> for SnapshotObserver<'i, T> {
    type Output = T::Output;
    fn index(&self, index: U) -> &Self::Output {
        (**self).index(index)
    }
}

impl<'i, T: IndexMut<U>, U> IndexMut<U> for SnapshotObserver<'i, T> {
    fn index_mut(&mut self, index: U) -> &mut Self::Output {
        (**self).index_mut(index)
    }
}

impl<'i, T: PartialEq<U>, U: ?Sized> PartialEq<U> for SnapshotObserver<'i, T> {
    fn eq(&self, other: &U) -> bool {
        (**self).eq(other)
    }
}

impl<'i, T: PartialOrd<U>, U: ?Sized> PartialOrd<U> for SnapshotObserver<'i, T> {
    fn partial_cmp(&self, other: &U) -> Option<std::cmp::Ordering> {
        (**self).partial_cmp(other)
    }
}

macro_rules! impl_assign_ops {
    ($($trait:ident => $method:ident),* $(,)?) => {
        $(
            impl<'i, T: ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for SnapshotObserver<'i, T> {
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
            impl<'i, T: Copy + ::std::ops::$trait<U>, U> ::std::ops::$trait<U> for SnapshotObserver<'i, T> {
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

macro_rules! impl_observe {
    ($($ty:ty $(=> $target:ty)?),* $(,)?) => {
        $(
            impl Observe for $ty {
                type Observer<'i> = SnapshotObserver<'i, $ty>
                where
                    Self: 'i;
            }
        )*
    };
}

impl_observe! {
    usize, u8, u16, u32, u64, u128, isize, i8, i16, i32, i64, i128, f32, f64, bool,
}
