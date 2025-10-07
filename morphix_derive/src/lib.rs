#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse_quote;
use syn::visit_mut::VisitMut;

mod derive_observe;

/// Derive `Observe` trait for a struct.
///
/// ## Example
///
/// ```
/// use serde::Serialize;
/// use morphix::Observe;
///
/// // It is commonly used with `Serialize`, `Clone` and `PartialEq` traits.
/// #[derive(Serialize, Clone, PartialEq, Observe)]
/// struct Point {
///    x: f64,
///    y: f64,
/// }
/// ```
#[proc_macro_derive(Observe)]
pub fn derive_observe(input: TokenStream) -> TokenStream {
    let input: syn::DeriveInput = syn::parse_macro_input!(input);

    match crate::derive_observe::derive_observe(input) {
        Ok(ts) => ts,
        Err(error) => error.to_compile_error(),
    }
    .into()
}

/// Observe the side effects of a closure.
///
/// ## Example
///
/// ```
/// use serde::Serialize;
/// use morphix::{observe, Observe};
///
/// #[derive(Serialize, Observe)]
/// struct Point {
///   x: f64,
///   y: f64,
/// }
///
/// let mut point = Point { x: 1.0, y: 2.0 };
/// observe!(|mut point| {
///    point.x += 1.0;
///    point.y += 1.0;
/// }).unwrap();
/// ```
#[proc_macro]
pub fn observe(input: TokenStream) -> TokenStream {
    let input: syn::Expr = syn::parse_macro_input!(input);
    let syn::Expr::Closure(mut closure) = input else {
        panic!("expect a closure expression")
    };
    if closure.inputs.len() != 1 {
        panic!("expect a closure with one argument")
    }
    let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &closure.inputs[0] else {
        panic!("expect a closure with one argument")
    };
    let body = &mut closure.body;
    let mut body_shadow: syn::Expr = parse_quote! {
        {
            #[allow(unused)]
            let mut #ident = #ident.observe(&ctx);
            #body;
        }
    };
    CallSite.visit_expr_mut(&mut body_shadow);
    quote! {
        {
            let _ = || #body;
            let ctx = ::morphix::Context::new();
            #body_shadow
            ctx.collect(&#ident)
        }
    }
    .into()
}

struct CallSite;

impl VisitMut for CallSite {
    fn visit_span_mut(&mut self, span: &mut Span) {
        *span = Span::call_site();
    }
}
