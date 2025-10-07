use std::ops::DerefMut;

use serde::{Serialize, Serializer};

use crate::Change;
use crate::adapter::Adapter;

mod shallow;
mod string;
mod vec;

pub use shallow::ShallowObserver;

/// A trait for types that can be observed for changes.
///
/// Types implementing `Observe` can be wrapped in [Observers] that track mutations.
/// The trait is typically derived using the `#[derive(Observe)]` macro.
///
/// ## Example
///
/// ```
/// use morphix::Observe;
/// use serde::Serialize;
///
/// #[derive(Serialize, Observe)]
/// struct MyStruct {
///     field: String,
/// }
///
/// let mut data = MyStruct { field: "value".to_string() };
/// let mut data_observer = data.observe();
/// // Mutations through observer are tracked
/// data_observer.field.push_str(" modified");
/// ```
///
/// [`Observers`]: crate::Observer
pub trait Observe: Serialize {
    /// Associated observer type.
    type Observer<'i>: Observer<'i, Self>
    where
        Self: 'i;

    /// Creates an observer for this value.
    ///
    /// ## Returns
    ///
    /// An observer that wraps this value and tracks mutations.
    #[inline]
    fn observe<'i>(&'i mut self) -> Self::Observer<'i> {
        Self::Observer::observe(self)
    }

    /// Serializes only the appended portion of the value.
    ///
    /// This method is used for optimizing append operations by only
    /// serializing the new data rather than the entire value.
    ///
    /// ## Arguments
    ///
    /// - `serializer` - serializer to use
    /// - `start_index` - index from which to start serialization
    ///
    /// ## Errors
    ///
    /// - Returns serialization errors from the underlying serializer.
    ///
    /// ## Panics
    ///
    /// - Panics if called on types that don't support append operations.
    #[inline]
    #[expect(unused_variables)]
    fn serialize_append<S: Serializer>(&self, serializer: S, start_index: usize) -> Result<S::Ok, S::Error> {
        unimplemented!()
    }
}

/// A trait for observer types that wrap and track mutations to values.
///
/// Observers provide transparent access to the underlying value while
/// recording any mutations that occur.
pub trait Observer<'i, T: ?Sized>: DerefMut<Target = T> {
    /// Creates a new observer for the given value.
    ///
    /// ## Arguments
    ///
    /// - `value` - value to observe
    fn observe(value: &'i mut T) -> Self;

    /// Collects all recorded changes using the specified adapter.
    ///
    /// ## Type Parameters
    ///
    /// - `A` - adapter to use for serialization
    ///
    /// ## Returns
    ///
    /// - `None` if no changes were recorded,
    /// - otherwise a `Change` containing all mutations that occurred.
    ///
    /// ## Errors
    ///
    /// - Returns an error if serialization fails.
    fn collect<A: Adapter>(this: Self) -> Result<Option<Change<A>>, A::Error>
    where
        T: Serialize;
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub enum Mutation {
    Replace,
    Append(usize),
}

#[doc(hidden)]
pub trait MutationObserver<'i, T>: Observer<'i, T> {
    fn mutation(this: &mut Self) -> &mut Option<Mutation>;

    fn mark_replace(this: &mut Self) {
        *Self::mutation(this) = Some(Mutation::Replace);
    }

    fn mark_append(this: &mut Self, start_index: usize) {
        let mutation = Self::mutation(this);
        if mutation.is_some() {
            return;
        }
        *mutation = Some(Mutation::Append(start_index));
    }
}
