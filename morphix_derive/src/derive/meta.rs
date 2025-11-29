use proc_macro2::TokenStream;
use syn::parse::{Parse, Parser};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

pub struct MetaArgument {
    ident: syn::Ident,
    args: Option<(syn::token::Paren, TokenStream)>,
    value: Option<(syn::Token![=], syn::Expr)>,
}

impl Parse for MetaArgument {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let args = if input.peek(syn::token::Paren) {
            let content;
            let paren = syn::parenthesized!(content in input);
            let args = content.parse()?;
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
        Ok(MetaArgument { ident, args, value })
    }
}

pub struct GeneralImpl {
    pub ob_ident: syn::Ident,
    pub spec_ident: syn::Ident,
    pub bounds: Punctuated<syn::TypeParamBound, syn::Token![+]>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttributeKind {
    Item,
    Field,
    #[expect(dead_code)]
    Variant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DeriveKind {
    Struct,
    Enum,
    Union,
}

#[derive(Default)]
pub struct SerdeMeta {
    pub flatten: bool,
    pub untagged: bool,
    pub tag: Option<syn::Expr>,
    pub content: Option<syn::Expr>,
}

#[derive(Default)]
pub struct ObserveMeta {
    pub general_impl: Option<GeneralImpl>,
    pub deref: Option<syn::Ident>,
    pub serde: SerdeMeta,
    pub derive: Vec<syn::Path>,
}

impl ObserveMeta {
    fn parse_morphix(
        &mut self,
        arg: MetaArgument,
        errors: &mut TokenStream,
        attribute_kind: AttributeKind,
        derive_kind: DeriveKind,
    ) {
        if arg.ident == "hash" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("HashObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("HashSpec", arg.ident.span()),
                bounds: parse_quote! { ::std::hash::Hash },
            });
        } else if arg.ident == "noop" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("NoopObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("DefaultSpec", arg.ident.span()),
                bounds: Default::default(),
            });
        } else if arg.ident == "shallow" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("ShallowObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("DefaultSpec", arg.ident.span()),
                bounds: Default::default(),
            });
        } else if arg.ident == "snapshot" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("SnapshotObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("SnapshotSpec", arg.ident.span()),
                bounds: parse_quote! { ::std::clone::Clone + ::std::cmp::PartialEq },
            });
        } else if arg.ident == "deref" {
            if attribute_kind != AttributeKind::Field || derive_kind != DeriveKind::Struct {
                errors.extend(
                    syn::Error::new(
                        arg.ident.span(),
                        "the 'deref' argument is only allowed on struct fields",
                    )
                    .to_compile_error(),
                );
            }
            self.deref = Some(arg.ident);
        } else if arg.ident == "derive" {
            if attribute_kind != AttributeKind::Item {
                errors.extend(
                    syn::Error::new(arg.ident.span(), "the 'derive' argument is only allowed on items")
                        .to_compile_error(),
                );
            }
            let Some((_, derive_args)) = arg.args else {
                errors.extend(
                    syn::Error::new(
                        arg.ident.span(),
                        "the 'derive' argument requires a list of traits, e.g., derive(Debug)",
                    )
                    .to_compile_error(),
                );
                return;
            };
            match Punctuated::<syn::Path, syn::Token![,]>::parse_terminated.parse2(derive_args) {
                Ok(paths) => self.derive.extend(paths),
                Err(error) => errors.extend(error.to_compile_error()),
            };
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

    fn parse_serde(&mut self, arg: MetaArgument, errors: &mut TokenStream) {
        if arg.ident == "flatten" {
            self.serde.flatten = true;
        } else if arg.ident == "untagged" {
            self.serde.untagged = true;
        } else if arg.ident == "tag" {
            let Some((_, expr)) = arg.value else {
                errors.extend(
                    syn::Error::new(
                        arg.ident.span(),
                        "the 'tag' argument requires a value, e.g., tag = \"type\"",
                    )
                    .to_compile_error(),
                );
                return;
            };
            self.serde.tag = Some(expr);
        } else if arg.ident == "content" {
            let Some((_, expr)) = arg.value else {
                errors.extend(
                    syn::Error::new(
                        arg.ident.span(),
                        "the 'content' argument requires a value, e.g., content = \"data\"",
                    )
                    .to_compile_error(),
                );
                return;
            };
            self.serde.content = Some(expr);
        }
    }

    pub fn parse_attrs(
        attrs: &[syn::Attribute],
        errors: &mut TokenStream,
        attribute_kind: AttributeKind,
        derive_kind: DeriveKind,
    ) -> Self {
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
                    meta.parse_morphix(arg, errors, attribute_kind, derive_kind);
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
                    meta.parse_serde(arg, errors);
                }
            }
        }
        meta
    }
}
