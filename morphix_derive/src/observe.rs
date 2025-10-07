use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::Parse;
use syn::parse_quote;
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;

struct Observe {
    ty: Option<(syn::Type, syn::Token![,])>,
    closure: syn::ExprClosure,
}

impl Parse for Observe {
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
pub fn observe(input: TokenStream) -> Result<TokenStream, syn::Error> {
    let mut input: Observe = syn::parse2(input)?;
    if input.closure.inputs.len() != 1 {
        return Err(syn::Error::new(
            input.closure.span(),
            "expect a closure with one argument",
        ));
    }
    let syn::Pat::Ident(syn::PatIdent { ident, .. }) = &input.closure.inputs[0] else {
        return Err(syn::Error::new(
            input.closure.span(),
            "expect a closure with one argument",
        ));
    };
    let body = &mut input.closure.body;
    let mut body_shadow: syn::Expr = parse_quote! {
        {
            let mut #ident = #ident.observe();
            #body;
            #ident
        }
    };
    CallSite.visit_expr_mut(&mut body_shadow);
    let turbofish = if let Some((ty, _)) = input.ty {
        quote! {::<#ty>}
    } else {
        quote! {}
    };
    Ok(quote! {
        {
            let _ = || #body;
            ::morphix::Observer::collect #turbofish(#body_shadow)
        }
    })
}

struct CallSite;

impl VisitMut for CallSite {
    fn visit_span_mut(&mut self, span: &mut Span) {
        *span = Span::call_site();
    }
}
