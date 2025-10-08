use std::ops::DerefMut;

use serde::Serialize;

use crate::{Adapter, Mutation};

mod shallow;
mod string;
mod vec;

pub use shallow::ShallowObserver;

/// A trait for types that can be observed for mutations.
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
    type Observer<'i>: Observer<'i, Target = Self>
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
}

/// A trait for observer types that wrap and track mutations to values.
///
/// Observers provide transparent access to the underlying value while recording any mutations that
/// occur.
pub trait Observer<'i>: DerefMut {
    /// Creates a new observer for the given value.
    ///
    /// ## Arguments
    ///
    /// - `value` - value to observe
    fn observe(value: &'i mut Self::Target) -> Self;

    /// Collects all recorded mutations using the specified adapter.
    ///
    /// ## Type Parameters
    ///
    /// - `A` - adapter to use for serialization
    ///
    /// ## Returns
    ///
    /// - `None` if no mutations were recorded,
    /// - otherwise a `Mutation` containing all mutations that occurred.
    ///
    /// ## Errors
    ///
    /// - Returns an error if serialization fails.
    fn collect<A: Adapter>(this: Self) -> Result<Option<Mutation<A>>, A::Error>
    where
        Self::Target: Serialize;

    /// Helper for autoref-based specialization.
    #[doc(hidden)]
    fn __morphix_deref_mut(&mut self) -> &mut Self::Target {
        self.deref_mut()
    }
}

impl<'i, T> Observer<'i> for &'i mut T {
    fn observe(value: &'i mut Self::Target) -> Self {
        value
    }

    fn collect<A: Adapter>(_: Self) -> Result<Option<Mutation<A>>, A::Error> {
        Ok(None)
    }
}

/// State of mutations tracked by a StatefulObserver(crate::StatefulObserver).
///
/// This enum represents the specific type of mutation that has been detected by observers that
/// implement StatefulObserver.
#[derive(Clone, Copy)]
pub enum MutationState {
    /// Complete replacement of the value
    Replace,
    /// Append operation starting from the given index
    Append(usize),
}

/// An [Observer] that maintains internal state about mutations.
///
/// Unlike [ShallowObserver] which only tracks whether a mutation occurred, StatefulObserver
/// implementations can distinguish between different types of mutations (replace vs. append) and
/// optimize the resulting mutation representation accordingly.
///
/// ## Implementation Notes
///
/// Implementing StatefulObserver allows an observer to track its own mutation state (e.g., replace
/// or append), but this doesn't preclude tracking additional mutations. Complex types like `Vec<T>`
/// may need to track both:
///
/// - Their own mutation state (via StatefulObserver)
/// - Changes to their elements (via nested observers)
///
/// These different sources of mutations are then combined into a final result:
///
/// ```ignore
/// // Example from VecObserver implementation
/// impl<'i, T: Observe> Observer<'i, Vec<T>> for VecObserver<'i, T> {
///     fn collect<A: Adapter>(mut this: Self) -> Result<Option<Mutation<A>>, A::Error> {
///         let mut mutations = vec![];
///
///         // 1. Collect own mutation state (replacement or append)
///         if let Some(state) = Self::mutation_state(&mut this).take() {
///             mutations.push(Mutation {
///                 operation: match state {
///                     MutationState::Replace => MutationKind::Replace(..),
///                     MutationState::Append(idx) => MutationKind::Append(..),
///                 },
///                 // ...
///             });
///         }
///
///         // 2. Collect mutations from nested element observers
///         for (index, observer) in element_observers {
///             if let Some(mutation) = observer.collect()? {
///                 mutations.push(mutation);
///             }
///         }
///
///         // 3. Combine all mutations (may result in a Batch)
///         Ok(Batch::build(mutations))
///     }
/// }
/// ```
///
/// This design allows for sophisticated mutation tracking where:
/// - Simple operations (like `vec.push()`) produce an `Append` mutation
/// - Element modifications (like `vec[0].field = value`) produce element-specific mutations
/// - Multiple operations produce a `Batch` containing all mutations
///
/// Currently implemented for:
/// - `String`
/// - `Vec<T>`
pub trait StatefulObserver<'i>: Observer<'i> {
    fn mutation_state(this: &mut Self) -> &mut Option<MutationState>;

    fn mark_replace(this: &mut Self) {
        *Self::mutation_state(this) = Some(MutationState::Replace);
    }

    fn mark_append(this: &mut Self, start_index: usize) {
        let mutation = Self::mutation_state(this);
        if mutation.is_some() {
            return;
        }
        *mutation = Some(MutationState::Append(start_index));
    }
}
