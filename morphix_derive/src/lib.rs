#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod derive_observe;
mod observe;

/// Derive `Observe` trait for a struct.
///
/// ## Example
///
/// ```
/// use serde::Serialize;
/// use morphix::Observe;
///
/// // Observe: Serialize
/// #[derive(Serialize, Observe)]
/// struct Point {
///    x: f64,
///    y: f64,
/// }
/// ```
#[proc_macro_derive(Observe, attributes(observe))]
pub fn derive_observe(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input);

    match crate::derive_observe::derive_observe(input) {
        Ok(ts) => ts,
        Err(errors) => errors.into_iter().map(|error| error.to_compile_error()).collect(),
    }
    .into()
}

/// Observe the side effects of a closure.
///
/// ## Example
///
/// ```
/// use serde::Serialize;
/// use morphix::{JsonAdapter, Change, Observe, observe};
///
/// #[derive(Serialize, Observe)]
/// struct Point {
///   x: f64,
///   y: f64,
/// }
///
/// let mut point = Point { x: 1.0, y: 2.0 };
/// let change = observe!(JsonAdapter, |mut point| {
///    point.x += 1.0;
///    point.y += 1.0;
/// })
/// .unwrap();
/// ```
#[proc_macro]
pub fn observe(input: TokenStream) -> TokenStream {
    match crate::observe::observe(input.into()) {
        Ok(ts) => ts,
        Err(errors) => errors.into_iter().map(|error| error.to_compile_error()).collect(),
    }
    .into()
}
