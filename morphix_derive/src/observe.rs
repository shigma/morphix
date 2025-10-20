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
    let body_original = &mut input.closure.body;
    let mut body_actual = body_original.clone();

    DerefAssign.visit_expr_mut(&mut body_actual);

    // reset span to call site
    let mut body_actual: syn::Expr = parse_quote! {
        {
            #[allow(unused_imports)]
            use ::morphix::helper::Assignable;
            let mut #ident = ::morphix::observe::ObserveExt::observe(&mut #ident);
            #[allow(clippy::needless_borrow)]
            #body_actual;
            #ident
        }
    };
    CallSite.visit_expr_mut(&mut body_actual);

    let turbofish = if let Some((ty, _)) = input.ty {
        quote! {::<#ty>}
    } else {
        quote! {}
    };
    Ok(quote! {
        {
            let _ = || #body_original;
            ::morphix::Observer::collect #turbofish(#body_actual)
        }
    })
}

struct DerefAssign;

impl VisitMut for DerefAssign {
    fn visit_expr_assign_mut(&mut self, expr_assign: &mut syn::ExprAssign) {
        syn::visit_mut::visit_expr_assign_mut(self, expr_assign);
        let left = &expr_assign.left;
        expr_assign.left = parse_quote! {
            *(&mut #left).__deref_mut()
        };
    }
}

struct CallSite;

impl VisitMut for CallSite {
    fn visit_span_mut(&mut self, span: &mut Span) {
        *span = Span::call_site();
    }
}
