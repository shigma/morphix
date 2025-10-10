#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod derive_observe;
mod observe;

/// Derive the `Observe` trait for structs to enable mutation tracking.
///
/// This macro generates an observer type that wraps your struct and tracks
/// mutations to its fields. The generated observer provides field-level
/// mutation detection with support for nested structures.
///
/// ## Requirements
///
/// - The struct must also derive or implement `Serialize`
/// - Only named structs are supported (not tuple structs or enums)
///
/// ## Field Attributes
///
/// You can customize how individual fields are observed using the `#[observe(...)]` attribute:
///
/// - `#[observe(hash)]` - Field will use [HashObserver](morphix::observe::HashObserver)
/// - `#[observe(noop)]` - Field will use [NoopObserver](morphix::observe::NoopObserver)
/// - `#[observe(shallow)]` - Field will use [ShallowObserver](morphix::observe::ShallowObserver)
/// - `#[observe(snapshot)]` - Field will use [SnapshotObserver](morphix::observe::SnapshotObserver)
///
/// ## Example
///
/// ```
/// use serde::Serialize;
/// use morphix::Observe;
///
/// #[derive(Serialize, Observe)]
/// struct User {
///     name: String,         // StringObserver
///     age: i32,             // SnapshotObserver<i32>
///
///     #[observe(noop)]
///     cache: String,        // Not tracked
///
///     #[observe(shallow)]
///     metadata: Metadata,   // ShallowObserver<Metadata>
/// }
///
/// #[derive(Serialize)]
/// struct Metadata {
///     created_at: String,
///     updated_at: String,
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

/// Observe and collect mutations within a closure.
///
/// This macro wraps a closure's operations to track all mutations that occur
/// within it. The closure receives a mutable reference to the value, and any
/// mutations made are automatically collected and returned.
///
/// ## Syntax
///
/// ```ignore
/// observe!(Adapter, |mut_binding| { /* mutations */ })
/// observe!(|mut_binding| { /* mutations */ })     // Type inference
/// ```
///
/// ## Parameters
///
/// - `Adapter` (optional) - adapter to use for serialization (e.g., `JsonAdapter`)
/// - `mut_binding` - binding pattern for the mutable value in the closure
///
/// ## Returns
///
/// Returns `Result<Option<Mutation<A>>, A::Error>` where:
/// - `Ok(None)` - No mutations were made
/// - `Ok(Some(mutation))` - Contains the collected mutations
/// - `Err(error)` - Serialization failed
///
/// ## Examples
///
/// With explicit adapter type:
///
/// ```
/// use serde::Serialize;
/// use morphix::{JsonAdapter, Observe, observe};
///
/// #[derive(Serialize, Observe)]
/// struct Point {
///     x: f64,
///     y: f64,
/// }
///
/// let mut point = Point { x: 1.0, y: 2.0 };
///
/// let mutation = observe!(JsonAdapter, |mut point| {
///     point.x += 1.0;
///     point.y *= 2.0;
/// }).unwrap();
///
/// assert_eq!(point.x, 2.0);
/// assert_eq!(point.y, 4.0);
/// ```
///
/// With type inference:
///
/// ```
/// # use serde::Serialize;
/// # use morphix::Observe;
/// # #[derive(Serialize, Observe)]
/// # struct Point {
/// #     x: f64,
/// #     y: f64,
/// # }
/// use morphix::{JsonAdapter, Mutation, observe};
///
/// let mut point = Point { x: 1.0, y: 2.0 };
///
/// let mutation: Option<Mutation<JsonAdapter>> = observe!(|mut point| {
///     point.x += 1.0;
///     point.y *= 2.0;
/// }).unwrap();
/// ```
#[proc_macro]
pub fn observe(input: TokenStream) -> TokenStream {
    match crate::observe::observe(input.into()) {
        Ok(ts) => ts,
        Err(errors) => errors.into_iter().map(|error| error.to_compile_error()).collect(),
    }
    .into()
}
