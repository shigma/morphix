use std::borrow::Cow;
use std::convert::Infallible;

use crate::Observe;
use crate::adapter::Adapter;
use crate::change::Change;
use crate::error::ChangeError;

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

    fn try_from_observe<T: Observe>(_value: &T, change: Change<ObserveAdapter>) -> Result<Change<Self>, Self::Error> {
        Ok(change)
    }
}
