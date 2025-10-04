#![doc = include_str!("../README.md")]

use proc_macro::TokenStream;
use quote::{ToTokens, format_ident, quote};
use sub::SynSub;

mod sub;

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
    let derive: syn::DeriveInput = syn::parse_macro_input!(input);
    let ident = &derive.ident;
    let (impl_generics, type_generics, where_clause) = derive.generics.split_for_impl();
    let ident_ob = format_ident!("{}Ob", ident);
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    match &derive.data {
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
    quote! {
        #[automatically_derived]
        impl #impl_generics Observe for #ident #type_generics #where_clause {
            type Target<'i> = #ident_ob<'i>;

            fn observe(&mut self, ctx: &::morphix::Context) -> Self::Target<'_> {
                #ident_ob {
                    #(#inst_fields)*
                }
            }
        }

        pub struct #ident_ob<'i> {
            #(#type_fields)*
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
/// #[derive(Serialize, Clone, PartialEq, Observe)]
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
    let body_shadow = body.to_token_stream();
    SubstIdent { ident: ident.clone() }.expr(body);
    quote! {
        {
            use ::std::ops::*;
            let _ = || #body_shadow;
            let ctx = ::morphix::Context::new();
            let mut #ident = #ident.observe(&ctx);
            #body;
            ctx.collect()
        }
    }
    .into()
}

struct SubstIdent {
    ident: syn::Ident,
}

impl SubstIdent {
    fn _expr_field(&mut self, expr_field: &mut syn::ExprField, inner: bool) -> Option<syn::Expr> {
        // erase span info from expr_field
        let member = format_ident!("{}", expr_field.member.to_token_stream().to_string());
        let method = match inner {
            true => format_ident!("borrow"),
            false => format_ident!("borrow_mut"),
        };
        match &mut *expr_field.base {
            syn::Expr::Path(expr_path) => {
                if self.ident == expr_path.to_token_stream().to_string() {
                    let ident = &self.ident;
                    return Some(syn::parse_quote! {
                        #ident.#member.#method()
                    });
                }
            }
            syn::Expr::Field(expr_field) => {
                if let Some(new_expr) = self._expr_field(expr_field, true) {
                    return Some(syn::parse_quote! {
                        #new_expr.#member.#method()
                    });
                }
            }
            _ => self.expr(&mut expr_field.base),
        }
        None
    }
}

impl SynSub for SubstIdent {
    fn expr_binary(&mut self, expr_binary: &mut syn::ExprBinary) -> Option<syn::Expr> {
        self.expr(&mut expr_binary.left);
        self.expr(&mut expr_binary.right);
        match &expr_binary.op {
            syn::BinOp::AddAssign(..) => {
                let left = &expr_binary.left;
                let right = &expr_binary.right;
                Some(syn::parse_quote! {
                    #left.add_assign(#right)
                })
            }
            _ => None,
        }
    }

    fn expr_field(&mut self, expr_filed: &mut syn::ExprField) -> Option<syn::Expr> {
        self._expr_field(expr_filed, false)
    }
}
