use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;
use syn::{Token, parse_quote};

enum ObserveKind {
    Closure(#[expect(dead_code)] Token![|], #[expect(dead_code)] Token![|]),
    Arm(#[expect(dead_code)] Token![=>]),
}

pub struct ObserveInput {
    kind: ObserveKind,
    pat: syn::Pat,
    body: syn::Expr,
}

impl Parse for ObserveInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let or1 = input.parse::<Token![|]>().ok();
        let mut pat = syn::Pat::parse_single(input)?;
        if let Ok(colon) = input.parse::<Token![:]>() {
            let ty: syn::Type = input.parse()?;
            pat = syn::Pat::Type(syn::PatType {
                attrs: vec![],
                pat: Box::new(pat),
                colon_token: colon,
                ty: Box::new(ty),
            });
        }
        let kind = if let Some(or1) = or1 {
            let or2 = input.parse::<Token![|]>()?;
            ObserveKind::Closure(or1, or2)
        } else {
            let fat_arrow = input.parse::<Token![=>]>()?;
            ObserveKind::Arm(fat_arrow)
        };
        let body = input.parse()?;
        Ok(Self { kind, pat, body })
    }
}

fn build_output(pat: &syn::Pat, inits: &mut Vec<TokenStream>) -> Result<TokenStream, TokenStream> {
    match pat {
        syn::Pat::Ident(syn::PatIdent { ident, .. }) => {
            inits.push(quote! { let mut #ident = #ident.__observe(); });
            Ok(quote! {
                match ::morphix::observe::SerializeObserverExt::flush(&mut #ident) {
                    Ok(mutation) => mutation,
                    Err(error) => break 'ob Err(error),
                }
            })
        }
        syn::Pat::Tuple(syn::PatTuple { elems, .. }) => {
            let mut outputs = vec![];
            let mut errors = TokenStream::new();
            for pat in elems {
                match build_output(pat, inits) {
                    Ok(output) => outputs.push(output),
                    Err(error) => errors.extend(error),
                }
            }
            if errors.is_empty() {
                Ok(quote! { (#(#outputs),*,) })
            } else {
                Err(errors)
            }
        }
        syn::Pat::Type(syn::PatType { pat, .. }) => build_output(pat, inits),
        syn::Pat::Wild(_) => Ok(quote! { ::morphix::Adapter::from_mutation(None) }),
        _ => Err(syn::Error::new(pat.span(), "only ident or tuple patterns are supported").to_compile_error()),
    }
}

pub fn observe(mut input: ObserveInput) -> TokenStream {
    let mut inits = vec![];
    let pat = &input.pat;
    let output = match build_output(pat, &mut inits) {
        Ok(output) => quote! { Ok(#output) },
        Err(errors) => return errors,
    };

    let body = &mut input.body;
    DerefAssign.visit_expr_mut(body);

    let body = quote! {
        'ob: {
            #[allow(unused_imports)]
            use ::morphix::helper::AsNormalized;
            use ::morphix::observe::ObserveExt;
            #(#inits)*
            #[allow(clippy::needless_borrow)]
            #body;
            #output
        }
    };

    match input.kind {
        ObserveKind::Closure(_, _) => quote! { |#pat| #body },
        ObserveKind::Arm(_) => body,
    }
}

struct DerefAssign;

impl VisitMut for DerefAssign {
    fn visit_expr_mut(&mut self, expr: &mut syn::Expr) {
        if let syn::Expr::Assign(expr_assign) = expr {
            let left = &expr_assign.left;
            let right = &expr_assign.right;
            *expr = parse_quote! {
                **(&mut #left).as_normalized_mut() = #right
            };
        }
        syn::visit_mut::visit_expr_mut(self, expr);
    }
}
