#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;

mod derive_observe;
mod observe;

/// Derive the [`Observe`](morphix::Observe) trait for structs to enable mutation tracking.
///
/// This macro automatically generates an [`Observe`](morphix::Observe) implementation for the
/// struct, producing a default [`Observer`](morphix::observe::Observer) type that wraps the struct
/// and tracks mutations to each field according to that field's own [`Observe`](morphix::Observe)
/// implementation.
///
/// ## Requirements
///
/// - The struct must also derive or implement [`Serialize`](serde::Serialize)
/// - Only named structs are supported (not tuple structs or enums)
///
/// ## Customizing Behavior
///
/// If a field type `T` does not implement `Observe`, or you need an alternative observer
/// implementation, you can customize this via the `#[observe(...)]` field attribute inside a
/// `#[derive(Observe)]` struct:
///
/// - `#[observe(hash)]` — use [`HashObserver`](morphix::observe::HashObserver) for this field
/// - `#[observe(noop)]` — use [`NoopObserver`](morphix::observe::NoopObserver) for this field
/// - `#[observe(shallow)]` — use [`ShallowObserver`](morphix::observe::ShallowObserver) for this
///   field
/// - `#[observe(snapshot)]` — use [`SnapshotObserver`](morphix::observe::SnapshotObserver) for this
///   field
///
/// These attributes allow you to override the default `Observer` type that would otherwise come
/// from the field's `Observe` implementation.
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
    derive_observe::derive_observe(input).into()
}

/// Observe and collect mutations within a closure.
///
/// This macro wraps a closure's operations to track all mutations that occur within it. The closure
/// receives a mutable reference to the value, and any mutations made are automatically collected
/// and returned.
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
/// Returns `Result<Option<Mutation<Adapter>>, Adapter::Error>` where:
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
    let input: observe::ObserveInput = syn::parse_macro_input!(input);
    observe::observe(input).into()
}

#[cfg(test)]
mod test {
    use std::env::var;
    use std::fs::{create_dir_all, read_to_string, write};
    use std::path::{Path, PathBuf};

    use macro_expand::Context;
    use pretty_assertions::StrComparison;
    use prettyplease::unparse;
    use walkdir::WalkDir;

    struct TestDiff {
        path: PathBuf,
        expect: String,
        actual: String,
    }

    #[test]
    fn fixtures() {
        let input_dir = "fixtures/input";
        let output_dir = "fixtures/output";
        let mut diffs = vec![];
        let will_emit = var("EMIT").is_ok_and(|v| !v.is_empty());
        for entry in WalkDir::new(input_dir).into_iter().filter_map(Result::ok) {
            let input_path = entry.path();
            if !input_path.is_file() || input_path.extension() != Some("rs".as_ref()) {
                continue;
            }
            let path = input_path.strip_prefix(input_dir).unwrap();
            let output_path = Path::new(output_dir).join(path);
            let input = read_to_string(input_path).unwrap().parse().unwrap();
            let mut ctx = Context::new();
            ctx.register_proc_macro_derive(
                "Observe".into(),
                crate::derive_observe::derive_observe,
                vec!["observe".into()],
            );
            let actual = unparse(&syn::parse2(ctx.transform(input)).unwrap());
            let expect_result = read_to_string(&output_path);
            if let Ok(expect) = &expect_result
                && expect == &actual
            {
                continue;
            }
            if will_emit {
                create_dir_all(output_path.parent().unwrap()).unwrap();
                write(output_path, &actual).unwrap();
            }
            if let Ok(expect) = expect_result {
                diffs.push(TestDiff {
                    path: path.to_path_buf(),
                    expect,
                    actual,
                });
            }
        }
        let len = diffs.len();
        for diff in diffs {
            eprintln!("diff {}", diff.path.display());
            eprintln!("{}", StrComparison::new(&diff.expect, &diff.actual));
        }
        if len > 0 && !will_emit {
            panic!("Some tests failed");
        }
    }
}
