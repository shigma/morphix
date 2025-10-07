use std::borrow::Cow;

use serde::Serialize;

use crate::{Change, ChangeError, Observe};

#[cfg(feature = "json")]
pub mod json;

pub trait Adapter: Sized {
    type Replace;
    type Append;
    type Error;

    fn new_replace<T: Serialize + ?Sized>(value: &T) -> Result<Self::Replace, Self::Error>;

    fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Append, Self::Error>;

    fn apply_change(
        old_value: &mut Self::Replace,
        change: Change<Self>,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError>;

    fn merge_append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError>;
}
