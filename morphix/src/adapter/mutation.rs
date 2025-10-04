use std::convert::Infallible;

use crate::adapter::Adapter;
use crate::change::Change;

pub struct MutationAdapter;

impl Adapter for MutationAdapter {
    type Replace = ();
    type Append = usize;
    type Error = Infallible;

    fn apply_change(
        _change: Change<Self>,
        _value: &mut Self::Replace,
        _path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn append(
        _old_value: &mut Self::Append,
        _new_value: Self::Append,
        _path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
