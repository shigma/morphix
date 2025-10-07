use std::borrow::Cow;

use serde::Serialize;

use crate::{Change, ChangeError, Observe};

#[cfg(feature = "json")]
pub mod json;

/// Trait for adapting changes to different serialization formats.
/// 
/// The `Adapter` trait provides an abstraction layer between the change detection
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
/// ```ignore
/// use morphix::Adapter;
/// use serde_json::value::Serializer;
/// use serde_json::{Error, Value};
/// 
/// struct JsonAdapter;
/// 
/// impl Adapter for JsonAdapter {
///     type Replace = Value;
///     type Append = Value;
///     type Error = Error;
///     
///     fn new_replace<T: Serialize + ?Sized>(value: &T) -> Result<Self::Replace, Self::Error> {
///         value.serialize(Serializer)
///     }
///     
///     // ... other methods
/// }
/// ```
pub trait Adapter: Sized {
    /// Type used to represent `Replace` values.
    type Replace;

    /// Type used to represent `Append` values.
    type Append;

    /// Error type for serialization / deserialization operations.
    type Error;

    /// Creates a replacement value from a serializable type.
    /// 
    /// ## Arguments
    /// 
    /// - `value` - The value to be serialized as a replacement
    fn new_replace<T: Serialize + ?Sized>(value: &T) -> Result<Self::Replace, Self::Error>;

    /// Creates an append value from an observable type.
    /// 
    /// ## Arguments
    /// 
    /// - `value` - The value to be serialized for appending
    /// - `start_index` - The starting index for serialization (used for partial serialization)
    fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Append, Self::Error>;

    /// Applies a [Change] to an existing value.
    /// 
    /// ## Arguments
    /// 
    /// - `old_value` - value to be modified
    /// - `change` - change to apply
    /// - `path_stack` - stack for tracking the current path (used for error reporting)
    /// 
    /// ## Errors
    /// 
    /// - Returns `ChangeError::IndexError` if the path doesn't exist.
    /// - Returns `ChangeError::OperationError` if the operation cannot be performed.
    /// 
    /// [`Change`]: crate::Change
    fn apply_change(
        old_value: &mut Self::Replace,
        change: Change<Self>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError>;

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
    /// - Returns `ChangeError::OperationError` if the values cannot be merged.
    fn merge_append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError>;
}
