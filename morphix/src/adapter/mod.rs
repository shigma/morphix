use serde::Serialize;

use crate::{Mutation, MutationError, Path};

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
/// # use morphix::{Mutation, MutationError, Observe, Path};
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
///     #     path_stack: &mut Path<false>,
///     # ) -> Result<(), MutationError> {
///     #     unimplemented!()
///     # }
///
///     # fn merge_append(
///     #     old_value: &mut Self::Value,
///     #     new_value: Self::Value,
///     #     path_stack: &mut Path<false>,
///     # ) -> Result<(), MutationError> {
///     #     unimplemented!()
///     # }
///
///     # fn get_len(
///     #     value: &Self::Value,
///     #     path_stack: &mut Path<false>,
///     # ) -> Result<usize, MutationError> {
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
    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error>;

    /// Applies a [Mutation](crate::Mutation) to an existing value.
    fn apply_mutation(
        old_value: &mut Self::Value,
        mutation: Mutation<Self>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError>;

    /// Merges one append value into another.
    fn merge_append(
        old_value: &mut Self::Value,
        append_value: Self::Value,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError>;

    fn get_len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError>;
}
