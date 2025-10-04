use crate::adapter::observe::ObserveAdapter;
use crate::change::Change;
use crate::error::ChangeError;
use crate::observe::Observe;

pub mod json;
pub mod observe;

pub trait Adapter: Sized {
    type Replace;
    type Append;
    type Error;

    fn apply_change(
        change: Change<Self>,
        value: &mut Self::Replace,
        path_stack: &mut Vec<String>,
    ) -> Result<(), ChangeError>;

    fn append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<String>,
    ) -> Result<(), ChangeError>;

    fn from_observe<T: Observe>(value: &T, change: Change<ObserveAdapter>) -> Result<Change<Self>, Self::Error>;
}
