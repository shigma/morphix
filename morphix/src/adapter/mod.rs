use crate::change::Change;

pub mod json;
pub mod mutation;

pub trait Adapter: Sized {
    type Replace;
    type Append;
    type Error;

    fn apply_change(
        change: Change<Self>,
        value: &mut Self::Replace,
        path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error>;

    fn append(
        old_value: &mut Self::Append,
        new_value: Self::Append,
        path_stack: &mut Vec<String>,
    ) -> Result<(), Self::Error>;
}
