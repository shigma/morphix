use proc_macro2::TokenStream;
use syn::parse::{Parse, Parser};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

pub struct MetaArgument {
    ident: syn::Ident,
    inner: Option<(syn::token::Paren, Punctuated<MetaArgument, syn::Token![,]>)>,
    value: Option<(syn::Token![=], syn::Expr)>,
}

impl Parse for MetaArgument {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let inner = if input.peek(syn::token::Paren) {
            let content;
            let paren = syn::parenthesized!(content in input);
            let args = Punctuated::<MetaArgument, syn::Token![,]>::parse_terminated(&content)?;
            Some((paren, args))
        } else {
            None
        };
        let value = if input.peek(syn::Token![=]) {
            let eq_token: syn::Token![=] = input.parse()?;
            let expr: syn::Expr = input.parse()?;
            Some((eq_token, expr))
        } else {
            None
        };
        Ok(MetaArgument { ident, inner, value })
    }
}

pub struct GeneralImpl {
    pub ob_ident: syn::Ident,
    pub spec_ident: syn::Ident,
    pub bounds: Punctuated<syn::TypeParamBound, syn::Token![+]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MetaPosition {
    Field,
    Struct,
}

#[derive(Default)]
pub struct ObserveMeta {
    pub general_impl: Option<GeneralImpl>,
    pub deref: Option<syn::Ident>,
    pub flatten: bool,
    pub untagged: bool,
    pub tag: Option<syn::Expr>,
    pub content: Option<syn::Expr>,
    pub derive: Vec<syn::Ident>,
}

impl ObserveMeta {
    pub fn parse_attrs(attrs: &[syn::Attribute], errors: &mut TokenStream, position: MetaPosition) -> Self {
        let mut meta = ObserveMeta::default();
        for attr in attrs {
            if attr.path().is_ident("morphix") {
                let syn::Meta::List(meta_list) = &attr.meta else {
                    errors.extend(
                        syn::Error::new(
                            attr.span(),
                            "the 'morphix' attribute must be in the form of #[morphix(...)]",
                        )
                        .to_compile_error(),
                    );
                    continue;
                };
                let args = match Punctuated::<MetaArgument, syn::Token![,]>::parse_terminated
                    .parse2(meta_list.tokens.clone())
                {
                    Ok(args) => args,
                    Err(err) => {
                        errors.extend(err.to_compile_error());
                        continue;
                    }
                };
                for arg in args {
                    if arg.ident == "hash" {
                        meta.general_impl = Some(GeneralImpl {
                            ob_ident: syn::Ident::new("HashObserver", arg.ident.span()),
                            spec_ident: syn::Ident::new("HashSpec", arg.ident.span()),
                            bounds: parse_quote! { ::std::hash::Hash },
                        });
                    } else if arg.ident == "noop" {
                        meta.general_impl = Some(GeneralImpl {
                            ob_ident: syn::Ident::new("NoopObserver", arg.ident.span()),
                            spec_ident: syn::Ident::new("DefaultSpec", arg.ident.span()),
                            bounds: Default::default(),
                        });
                    } else if arg.ident == "shallow" {
                        meta.general_impl = Some(GeneralImpl {
                            ob_ident: syn::Ident::new("ShallowObserver", arg.ident.span()),
                            spec_ident: syn::Ident::new("DefaultSpec", arg.ident.span()),
                            bounds: Default::default(),
                        });
                    } else if arg.ident == "snapshot" {
                        meta.general_impl = Some(GeneralImpl {
                            ob_ident: syn::Ident::new("SnapshotObserver", arg.ident.span()),
                            spec_ident: syn::Ident::new("SnapshotSpec", arg.ident.span()),
                            bounds: parse_quote! { ::std::clone::Clone + ::std::cmp::PartialEq },
                        });
                    } else if arg.ident == "deref" {
                        if position == MetaPosition::Struct {
                            errors.extend(
                                syn::Error::new(arg.ident.span(), "the 'deref' argument is only allowed on fields")
                                    .to_compile_error(),
                            );
                        }
                        meta.deref = Some(arg.ident);
                    } else if arg.ident == "derive" {
                        if position != MetaPosition::Struct {
                            errors.extend(
                                syn::Error::new(arg.ident.span(), "the 'derive' argument is only allowed on structs")
                                    .to_compile_error(),
                            );
                        }
                        let Some((_, derive_args)) = arg.inner else {
                            errors.extend(
                                syn::Error::new(
                                    arg.ident.span(),
                                    "the 'derive' argument requires a list of traits, e.g., derive(Debug)",
                                )
                                .to_compile_error(),
                            );
                            continue;
                        };
                        for derive_arg in derive_args {
                            meta.derive.push(derive_arg.ident);
                        }
                    } else {
                        errors.extend(
                            syn::Error::new(
                                arg.ident.span(),
                                "unknown argument, expected 'deref', 'hash', 'noop', 'shallow' or 'snapshot'",
                            )
                            .to_compile_error(),
                        );
                    }
                }
            } else if attr.path().is_ident("serde") {
                let syn::Meta::List(meta_list) = &attr.meta else {
                    errors.extend(
                        syn::Error::new(
                            attr.span(),
                            "the 'serde' attribute must be in the form of #[serde(...)]",
                        )
                        .to_compile_error(),
                    );
                    continue;
                };
                let args = match Punctuated::<MetaArgument, syn::Token![,]>::parse_terminated
                    .parse2(meta_list.tokens.clone())
                {
                    Ok(args) => args,
                    Err(err) => {
                        errors.extend(err.to_compile_error());
                        continue;
                    }
                };
                for arg in args {
                    if arg.ident == "flatten" {
                        meta.flatten = true;
                    } else if arg.ident == "untagged" {
                        meta.untagged = true;
                    } else if arg.ident == "tag" {
                        if let Some((_, expr)) = arg.value {
                            meta.tag = Some(expr);
                        }
                    } else if arg.ident == "content" {
                        // no-collapse
                        if let Some((_, expr)) = arg.value {
                            meta.content = Some(expr);
                        }
                    }
                }
            }
        }
        meta
    }
}
