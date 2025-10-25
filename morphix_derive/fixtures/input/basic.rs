use morphix_derive::Observe;
use serde::Serialize;
#[derive(Serialize, Observe)]
struct Foo {
    a: i32,
}
