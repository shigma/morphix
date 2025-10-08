use std::borrow::Cow;

use serde::Serialize;

use crate::{Mutation, MutationError, Observe};

#[cfg(feature = "json")]
pub mod json;
#[cfg(feature = "yaml")]
pub mod yaml;

/// Trait for adapting mutations to different serialization formats.
/// 
/// The `Adapter` trait provides an abstraction layer between the mutation detection
/// system and the serialization format. This allows morphix to support multiple
/// output formats while maintaining type safety.
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
///     fn new_replace<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error> {
///         value.serialize(Serializer)
///     }
///     
///     // ... other methods
///     # fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Value, Self::Error> {
///     #     unimplemented!()
///     # }
/// 
///     # fn apply_change(
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

    /// Creates a replacement value from a serializable type.
    /// 
    /// ## Arguments
    /// 
    /// - `value` - The value to be serialized as a replacement
    fn new_replace<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error>;

    /// Creates an append value from an observable type.
    /// 
    /// ## Arguments
    /// 
    /// - `value` - The value to be serialized for appending
    /// - `start_index` - The starting index for serialization (used for partial serialization)
    fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Value, Self::Error>;

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
    fn apply_change(
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
