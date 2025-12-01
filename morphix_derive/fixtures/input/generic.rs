#[allow(unused_imports)]
use morphix_derive::Observe;
use serde::Serialize;

#[rustfmt::skip]
#[derive(Serialize, Observe)]
#[serde(bound = "T: Serialize")]
pub struct Foo<'i, T, const N: usize> {
    #[serde(serialize_with = "serialize_mut_array")]
    a: &'i mut [T; N],
}

#[rustfmt::skip]
fn serialize_mut_array<T, S, const N: usize>(a: &&mut [T; N], serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize,
    S: serde::Serializer,
{
    <[_]>::serialize(&**a, serializer)
}
