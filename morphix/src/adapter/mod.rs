use std::mem::take;

use serde::Serialize;

use crate::{Mutation, MutationError, MutationKind, Path, PathSegment};

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

    type IntoValues: ExactSizeIterator<Item = Self::Value>;

    /// Constructs the adapter from an optional mutation.
    fn from_mutation(mutation: Option<Mutation<Self::Value>>) -> Self;

    /// Serializes a value into the adapter's Value type.
    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error>;

    fn get_mut<'a>(
        value: &'a mut Self::Value,
        segment: &PathSegment,
        allow_create: bool,
    ) -> Option<&'a mut Self::Value>;

    /// Merges one append value into another.
    #[cfg(feature = "append")]
    fn apply_append(
        value: &mut Self::Value,
        append_value: Self::Value,
        path_stack: &mut Path<false>,
    ) -> Result<usize, MutationError>;

    #[cfg(feature = "truncate")]
    fn apply_truncate(
        value: &mut Self::Value,
        truncate_len: usize,
        path_stack: &mut Path<false>,
    ) -> Result<Option<usize>, MutationError>;

    fn into_values(value: Self::Value) -> Option<Self::IntoValues>;

    fn from_values(values: Self::IntoValues) -> Self::Value;

    fn len(value: &Self::Value, path_stack: &mut Path<false>) -> Result<usize, MutationError>;

    /// Applies a [Mutation] to an existing value.
    fn apply_mutation(
        mut value: &mut Self::Value,
        mut mutation: Mutation<Self::Value>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        let is_replace = matches!(mutation.kind, MutationKind::Replace { .. });

        while let Some(segment) = mutation.path.pop() {
            let next_value = Self::get_mut(value, &segment, is_replace && mutation.path.is_empty());
            path_stack.push(segment);
            let Some(next_value) = next_value else {
                return Err(MutationError::IndexError { path: take(path_stack) });
            };
            value = next_value;
        }

        match mutation.kind {
            MutationKind::Replace(replace_value) => {
                *value = replace_value;
            }
            #[cfg(feature = "append")]
            MutationKind::Append(append_value) => {
                Self::apply_append(value, append_value, path_stack)?;
            }
            #[cfg(feature = "truncate")]
            MutationKind::Truncate(truncate_len) => {
                if let Some(actual_len) = Self::apply_truncate(value, truncate_len, path_stack)? {
                    return Err(MutationError::TruncateError {
                        path: take(path_stack),
                        actual_len,
                        truncate_len,
                    });
                }
            }
            MutationKind::Batch(mutations) => {
                let len = path_stack.len();
                for mutation in mutations {
                    Self::apply_mutation(value, mutation, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }
}
