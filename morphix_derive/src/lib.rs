#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::parse_quote;
use syn::visit_mut::VisitMut;

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
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let ident_ob = format_ident!("{}Ob", ident);
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            for name in named {
                let ident = name.ident.as_ref().unwrap();
                let ty = &name.ty;
                type_fields.push(quote! {
                    pub #ident: ::morphix::Ob<'i, #ty>,
                });
                inst_fields.push(quote! {
                    #ident: ::morphix::Ob {
                        value: &mut self.#ident,
                        ctx: ctx.extend(stringify!(#ident)),
                    },
                });
            }
        }
        _ => unimplemented!("not implemented"),
    };
    let mod_ident = format_ident!("__morphix_{}", ident);
    quote! {
        #[allow(nonstandard_style)]
        mod #mod_ident {
            use super::*;

            pub struct #ident_ob<'i> {
                #(#type_fields)*
            }

            #[automatically_derived]
            impl #impl_generics Observe for #ident #type_generics #where_clause {
                type Target<'i> = #ident_ob<'i>;

                fn observe(&mut self, ctx: &::morphix::Context) -> Self::Target<'_> {
                    #ident_ob {
                        #(#inst_fields)*
                    }
                }
            }
        }
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
