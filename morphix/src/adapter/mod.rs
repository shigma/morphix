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

    /// Constructs the adapter from an optional mutation.
    fn from_mutation(mutation: Option<Mutation<Self::Value>>) -> Self;

    /// Serializes a value into the adapter's Value type.
    fn serialize_value<T: Serialize + ?Sized>(value: &T) -> Result<Self::Value, Self::Error>;

    /// Gets a mutable reference to a nested value by path segment.
    ///
    /// This method navigates into `value` using the provided `segment` and returns a mutable
    /// reference to the nested value if it exists.
    ///
    /// ## Parameters
    ///
    /// - `value`: The value to navigate into
    /// - `segment`: The path segment indicating which nested value to access
    /// - `allow_create`: If `true` and the segment refers to a non-existent key in an object / map,
    ///   an empty value will be created at that location
    ///
    /// ## Returns
    ///
    /// - [`Some(value)`](Some): A mutable reference to the nested value
    /// - [`None`]: If the operation is not supported on this value type, or if the segment refers
    ///   to a non-existent location and `allow_create` is `false`
    fn get_mut<'a>(
        value: &'a mut Self::Value,
        segment: &PathSegment,
        allow_create: bool,
    ) -> Option<&'a mut Self::Value>;

    /// Appends a value to the end of another value.
    ///
    /// This method performs an append operation similar to [`String::push_str`] or
    /// [`Extend::extend`], merging `append_value` into the end of `value`.
    ///
    /// ## Parameters
    ///
    /// - `value`: The value to append to
    /// - `append_value`: The value to be appended
    ///
    /// ## Returns
    ///
    /// - [`Some(append_len)`](Some): The length of the appended portion
    /// - [`None`]: If the operation is not supported (e.g., incompatible types between `value` and
    ///   `append_value`, or `value` is not an appendable type)
    ///
    /// ## Note
    ///
    /// For strings, the returned length represents the char count, not the byte length.
    #[cfg(feature = "append")]
    fn append(value: &mut Self::Value, append_value: Self::Value) -> Option<usize>;

    /// Returns the appendable length of a value.
    ///
    /// This method returns the current length of a value that can be used with
    /// [`append`](Adapter::append) operations.
    ///
    /// ## Returns
    ///
    /// - [`Some(len)`](Some): The current length of the value
    /// - [`None`]: If the value is not an appendable type
    ///
    /// ## Note
    ///
    /// For strings, the returned length represents the char count, not the byte length.
    #[cfg(feature = "append")]
    fn len(value: &Self::Value) -> Option<usize>;

    /// Truncates a value by removing elements from the end.
    ///
    /// This method removes up to `truncate_len` elements from the end of `value`.
    ///
    /// ## Parameters
    ///
    /// - `value`: The value to truncate
    /// - `truncate_len`: The number of elements to remove from the end
    ///
    /// ## Returns
    ///
    /// - [`Some(remaining)`](Some): The remaining truncation length that could not be applied.
    ///   Returns `0` if the full truncation was successful. If `truncate_len` exceeds the actual
    ///   length, returns `truncate_len - actual_len` and clears the value.
    /// - [`None`]: If the operation is not supported on this value type
    ///
    /// ## Note
    ///
    /// For strings, the returned length represents the char count, not the byte length.
    #[cfg(feature = "truncate")]
    fn truncate(value: &mut Self::Value, truncate_len: usize) -> Option<usize>;

    /// Applies a [Mutation] to an existing value.
    fn mutate(
        mut value: &mut Self::Value,
        mut mutation: Mutation<Self::Value>,
        path_stack: &mut Path<false>,
    ) -> Result<(), MutationError> {
        let is_replace = matches!(mutation.kind, MutationKind::Replace { .. });

        while let Some(segment) = mutation.path.pop() {
            let inner_value = Self::get_mut(value, &segment, is_replace && mutation.path.is_empty());
            path_stack.push(segment);
            let Some(inner_value) = inner_value else {
                return Err(MutationError::IndexError { path: take(path_stack) });
            };
            value = inner_value;
        }

        match mutation.kind {
            MutationKind::Replace(replace_value) => {
                *value = replace_value;
            }
            #[cfg(feature = "append")]
            MutationKind::Append(append_value) => {
                if Self::append(value, append_value).is_none() {
                    return Err(MutationError::OperationError { path: take(path_stack) });
                }
            }
            #[cfg(feature = "truncate")]
            MutationKind::Truncate(truncate_len) => {
                let Some(remaining) = Self::truncate(value, truncate_len) else {
                    return Err(MutationError::OperationError { path: take(path_stack) });
                };
                if remaining > 0 {
                    return Err(MutationError::TruncateError {
                        path: take(path_stack),
                        actual_len: truncate_len - remaining,
                        truncate_len,
                    });
                }
            }
            MutationKind::Batch(mutations) => {
                let len = path_stack.len();
                for mutation in mutations {
                    Self::mutate(value, mutation, path_stack)?;
                    path_stack.truncate(len);
                }
            }
        }

        Ok(())
    }
}
