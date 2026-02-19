#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[serde(bound = "T: Serialize")]
pub struct Foo<'a, T, U, const N: usize> {
    #[serde(serialize_with = "serialize_mut_array")]
    a: &'a mut [T; N],
    #[serde(skip)]
    pub b: U,
}

#[rustfmt::skip]
fn serialize_mut_array<T, S, const N: usize>(a: &&mut [T; N], serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: serde::Serializer,
{
    <[_]>::serialize(&**a, serializer)
}
