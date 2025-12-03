use serde::Serialize;

use crate::{Mutation, MutationError, Path};

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "yaml")]
mod yaml;

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub use json::Json;
#[cfg(feature = "yaml")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml")))]
pub use yaml::Yaml;

/// Trait for adapting mutations to different serialization formats.
///
/// The `Adapter` trait provides an abstraction layer between the mutation detection system and the
/// serialization format. This allows morphix to support multiple output formats while maintaining
/// type safety.
///
/// ## Type Parameters
///
/// - `Value`: Type used to represent `Replace` and `Append` values.
/// - `Error`: Error type for serialization / deserialization operations.
pub trait Adapter: Sized {
    /// Type used to represent `Replace` and `Append` values.
    type Value;

    /// Error type for serialization / deserialization operations.
    type Error;

    /// Constructs the adapter from an optional mutation.
    fn from_mutation(mutation: Option<Mutation<Self::Value>>) -> Self;

    /// Serializes a value into the adapter's Value type.
    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error>;

    /// Applies a [Mutation] to an existing value.
    fn apply_mutation(
        old_value: &mut Self::Value,
        mutation: Mutation<Self::Value>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError>;

    /// Merges one append value into another.
    #[cfg(feature = "append")]
    fn merge_append(
        old_value: &mut Self::Value,
        append_value: Self::Value,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError>;

    fn get_len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError>;
}
