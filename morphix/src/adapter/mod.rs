use std::borrow::Cow;

use serde::Serialize;

use crate::{Mutation, MutationError};

#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "yaml")]
pub mod yaml;

/// Trait for adapting mutations to different serialization formats.
///
/// The `Adapter` trait provides an abstraction layer between the mutation detection system and the
/// serialization format. This allows morphix to support multiple output formats while maintaining
/// type safety.
///
/// ## Type Parameters
///
/// - `Replace`: The type used to represent replacement values
/// - `Append`: The type used to represent append values
/// - `Error`: The error type returned by serialization / deserialization operations
///
/// ## Example
///
/// ```
/// # use std::borrow::Cow;
/// # use morphix::{Mutation, MutationError, Observe};
/// use morphix::Adapter;
/// use serde::Serialize;
/// use serde_json::value::Serializer;
/// use serde_json::{Error, Value};
///
/// struct JsonAdapter;
///
/// impl Adapter for JsonAdapter {
///     type Value = Value;
///     type Error = Error;
///
///     fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error> {
///         value.serialize(Serializer)
///     }
///
///     // ... other methods
///     # fn apply_mutation(
///     #     old_value: &mut Self::Value,
///     #     mutation: Mutation<Self>,
///     #     path_stack: &mut Vec<Cow<'static, str>>,
///     # ) -> Result<(), MutationError> {
///     #     unimplemented!()
///     # }
///
///     # fn merge_append(
///     #     old_value: &mut Self::Value,
///     #     new_value: Self::Value,
///     #     path_stack: &mut Vec<Cow<'static, str>>,
///     # ) -> Result<(), MutationError> {
///     #     unimplemented!()
///     # }
/// }
/// ```
pub trait Adapter: Sized {
    /// Type used to represent `Replace` and `Append` values.
    type Value;

    /// Error type for serialization / deserialization operations.
    type Error;

    /// Serializes a value into the adapter's Value type.
    ///
    /// ## Arguments
    ///
    /// - `value` - value to be serialized as a replacement
    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error>;

    /// Applies a [Mutation](crate::Mutation) to an existing value.
    ///
    /// ## Arguments
    ///
    /// - `old_value` - value to be modified
    /// - `mutation` - mutation to apply
    /// - `path_stack` - stack for tracking the current path (used for error reporting)
    ///
    /// ## Errors
    ///
    /// - Returns `MutationError::IndexError` if the path doesn't exist.
    /// - Returns `MutationError::OperationError` if the operation cannot be performed.
    fn apply_mutation(
        old_value: &mut Self::Value,
        mutation: Mutation<Self>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), MutationError>;

    /// Merges one append value into another.
    ///
    /// ## Arguments
    ///
    /// - `old_value` - The existing append value
    /// - `new_value` - The new append value to merge
    /// - `path_stack` - Stack for tracking the current path (used for error reporting)
    ///
    /// ## Errors
    ///
    /// - Returns `MutationError::OperationError` if the values cannot be merged.
    fn merge_append(
        old_value: &mut Self::Value,
        new_value: Self::Value,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), MutationError>;
}
