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

fn build_output(
    pat: &syn::Pat,
    turbofish: &TokenStream,
    inits: &mut Vec<TokenStream>,
) -> Result<TokenStream, TokenStream> {
    match pat {
        syn::Pat::Ident(syn::PatIdent { ident, .. }) => {
            inits.push(quote! { let mut #ident = #ident.__observe(); });
            Ok(quote! {
                match ::morphix::observe::SerializeObserver::collect #turbofish(&mut #ident) {
                    Ok(mutation) => mutation,
                    Err(error) => break 'ob Err(error),
                }
            })
        }
        syn::Pat::Tuple(syn::PatTuple { elems, .. }) => {
            let mut outputs = vec![];
            let mut errors = TokenStream::new();
            for pat in elems {
                match build_output(pat, turbofish, inits) {
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
        _ => Err(syn::Error::new(pat.span(), "only ident or tuple patterns are supported").to_compile_error()),
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

    let mut inits = vec![];
    let output = match build_output(&input.closure.inputs[0], &turbofish, &mut inits) {
        Ok(output) => quote! { Ok(#output) },
        Err(errors) => return errors,
    };

    let body = &mut input.closure.body;
    DerefAssign.visit_expr_mut(body);

    quote! {
        'ob: {
            #[allow(unused_imports)]
            use ::morphix::helper::Assignable;
            use ::morphix::observe::ObserveExt;
            #(#inits)*
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
