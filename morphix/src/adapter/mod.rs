use std::borrow::Cow;

use crate::adapter::observe::ObserveAdapter;
use crate::change::Change;
use crate::error::ChangeError;
use crate::{Observe, Operation};

pub mod json;
pub mod observe;

pub trait Adapter: Sized {
    type Replace;
    type Append;
    type Error;

    fn apply_change(
        change: Change<Self>,
        value: &mut Self::Replace,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError>;

    fn append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError>;

    fn try_from_observe<'i, T: Observe + ?Sized>(
        observer: &mut T::Target<'i>,
        operation: Operation<ObserveAdapter>,
    ) -> Result<Operation<Self>, Self::Error>;

    fn new_replace<T: Observe + ?Sized>(value: &T) -> Result<Self::Replace, Self::Error>;

    fn new_append<T: Observe + ?Sized>(value: &T, start_index: usize) -> Result<Self::Append, Self::Error>;
}
