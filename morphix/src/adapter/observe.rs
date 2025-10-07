use std::borrow::Cow;
use std::convert::Infallible;

use crate::adapter::Adapter;
use crate::change::Change;
use crate::error::ChangeError;
use crate::{Observe, Operation};

pub struct ObserveAdapter;

impl Adapter for ObserveAdapter {
    type Replace = ();
    type Append = usize;
    type Error = Infallible;

    fn apply_change(
        _change: Change<Self>,
        _value: &mut Self::Replace,
        _path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError> {
        Ok(())
    }

    fn append(
        _old_value: &mut Self::Append,
        _new_value: Self::Append,
        _path_stack: &mut Vec<Cow<'static, str>>,
    ) -> Result<(), ChangeError> {
        Ok(())
    }

    fn try_from_observe<'i, T: Observe + ?Sized>(
        _observer: &mut T::Target<'i>,
        change: Operation<ObserveAdapter>,
    ) -> Result<Operation<Self>, Self::Error> {
        Ok(change)
    }

    fn new_replace<T: Observe + ?Sized>(_value: &T) -> Result<Self::Replace, Self::Error> {
        Ok(())
    }

    fn new_append<T: Observe + ?Sized>(_value: &T, start_index: usize) -> Result<Self::Append, Self::Error> {
        Ok(start_index)
    }
}
