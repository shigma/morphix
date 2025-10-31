use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;

pub struct ObserveInput {
    ty: Option<(syn::Type, syn::Token![,])>,
    closure: syn::ExprClosure,
}

impl Parse for ObserveInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if let Ok(ty) = input.parse::<syn::Type>() {
            let comma = input.parse::<syn::Token![,]>()?;
            let closure = input.parse::<syn::ExprClosure>()?;
            Ok(Self {
                ty: Some((ty, comma)),
                closure,
            })
        } else {
            let closure = input.parse::<syn::ExprClosure>()?;
            Ok(Self { ty: None, closure })
        }
    }
}

pub fn observe(mut input: ObserveInput) -> TokenStream {
    if input.closure.inputs.len() != 1 {
        return syn::Error::new(input.closure.span(), "expect a closure with one argument").to_compile_error();
    }

    let turbofish = if let Some((ty, _)) = input.ty {
        quote! {::<#ty>}
    } else {
        quote! {}
    };

    let (init, output) = match &input.closure.inputs[0] {
        syn::Pat::Ident(syn::PatIdent { ident, .. }) => (
            quote! { let mut #ident = #ident.__observe(); },
            quote! { ::morphix::observe::SerializeObserver::collect #turbofish(&mut #ident) },
        ),
        syn::Pat::Tuple(syn::PatTuple { elems, .. }) => {
            let mut inits = Vec::new();
            let mut outputs = Vec::new();
            let mut errors = Vec::new();
            for pat in elems {
                match pat {
                    syn::Pat::Ident(syn::PatIdent { ident, .. }) => {
                        inits.push(quote! { let mut #ident = #ident.__observe(); });
                        outputs.push(quote! {
                            match ::morphix::observe::SerializeObserver::collect #turbofish(&mut #ident) {
                                Ok(mutation) => mutation,
                                Err(error) => break 'ob Err(error),
                            }
                        });
                    }
                    _ => errors.push(syn::Error::new(pat.span(), "expect a closure with ident pattern")),
                }
            }
            if !errors.is_empty() {
                return errors.into_iter().map(|error| error.to_compile_error()).collect();
            }
            (quote! { #(#inits)* }, quote! { Ok((#(#outputs),*,)) })
        }
        _ => {
            return syn::Error::new(input.closure.span(), "expect a closure with ident pattern").to_compile_error();
        }
    };

    let body = &mut input.closure.body;
    DerefAssign.visit_expr_mut(body);

    quote! {
        'ob: {
            #[allow(unused_imports)]
            use ::morphix::helper::Assignable;
            use ::morphix::observe::ObserveExt;
            #init
            #[allow(clippy::needless_borrow)]
            #body;
            #output
        }
    }
}

struct DerefAssign;

impl VisitMut for DerefAssign {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Assign(expr_assign) = expr {
            let left = &expr_assign.left;
            let right = &expr_assign.right;
            *expr = parse_quote! {
                (&mut #left).__assign(#right)
            };
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}
