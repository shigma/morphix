use heck::{ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro2::TokenStream;
use syn::parse::{Parse, Parser};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

use crate::derive::snapshot::{derive_default, derive_noop_snapshot, derive_snapshot};

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
    pub extra_derive: fn(&syn::DeriveInput) -> TokenStream,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AttributeKind {
    Item,
    Field,
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
    pub rename: Option<syn::Expr>,
    pub rename_all: RenameRule,
    pub rename_all_fields: RenameRule,
}

#[derive(Default, Copy, Clone, PartialEq, Eq)]
pub enum RenameRule {
    #[default]
    None,
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

const RENAME_RULES: &[(&str, RenameRule)] = &[
    ("lowercase", RenameRule::LowerCase),
    ("UPPERCASE", RenameRule::UpperCase),
    ("PascalCase", RenameRule::PascalCase),
    ("camelCase", RenameRule::CamelCase),
    ("snake_case", RenameRule::SnakeCase),
    ("SCREAMING_SNAKE_CASE", RenameRule::ScreamingSnakeCase),
    ("kebab-case", RenameRule::KebabCase),
    ("SCREAMING-KEBAB-CASE", RenameRule::ScreamingKebabCase),
];

impl RenameRule {
    pub fn from_str(input: &str) -> Option<Self> {
        for (name, rule) in RENAME_RULES {
            if input == *name {
                return Some(*rule);
            }
        }
        None
    }

    pub fn or(self, other: Self) -> Self {
        if self == Self::None { other } else { self }
    }

    pub fn apply(self, name: &str) -> String {
        match self {
            Self::None => name.to_string(),
            Self::LowerCase => name.to_ascii_lowercase(),
            Self::UpperCase => name.to_ascii_uppercase(),
            Self::PascalCase => name.to_upper_camel_case(),
            Self::CamelCase => name.to_lower_camel_case(),
            Self::SnakeCase => name.to_snake_case(),
            Self::ScreamingSnakeCase => name.to_shouty_snake_case(),
            Self::KebabCase => name.to_kebab_case(),
            Self::ScreamingKebabCase => name.to_shouty_kebab_case(),
        }
    }
}

#[derive(Default)]
pub struct ObserveMeta {
    pub general_impl: Option<GeneralImpl>,
    pub deref: Option<syn::Ident>,
    pub serde: SerdeMeta,
    pub derive: (Vec<syn::Ident>, Vec<syn::Path>),
    pub expose: bool,
}

impl ObserveMeta {
    fn parse_morphix(
        &mut self,
        arg: MetaArgument,
        errors: &mut TokenStream,
        attribute_kind: AttributeKind,
        derive_kind: DeriveKind,
    ) {
        if arg.ident == "noop" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("NoopObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("SnapshotSpec", arg.ident.span()),
                bounds: Default::default(),
                extra_derive: derive_noop_snapshot,
            });
        } else if arg.ident == "shallow" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("ShallowObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("DefaultSpec", arg.ident.span()),
                bounds: Default::default(),
                extra_derive: derive_default,
            });
        } else if arg.ident == "snapshot" {
            self.general_impl = Some(GeneralImpl {
                ob_ident: syn::Ident::new("SnapshotObserver", arg.ident.span()),
                spec_ident: syn::Ident::new("SnapshotSpec", arg.ident.span()),
                bounds: parse_quote! { ::morphix::builtin::Snapshot },
                extra_derive: derive_snapshot,
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
            self.derive.0.push(arg.ident);
            match Punctuated::<syn::Path, syn::Token![,]>::parse_terminated.parse2(derive_args) {
                Ok(paths) => self.derive.1.extend(paths),
                Err(error) => errors.extend(error.to_compile_error()),
            };
        } else if arg.ident == "expose" {
            if attribute_kind != AttributeKind::Item {
                errors.extend(
                    syn::Error::new(arg.ident.span(), "the 'expose' argument is only allowed on items")
                        .to_compile_error(),
                );
            }
            self.expose = true;
        } else {
            errors.extend(
                syn::Error::new(
                    arg.ident.span(),
                    "unknown argument, expected 'deref', 'noop', 'shallow' or 'snapshot'",
                )
                .to_compile_error(),
            );
        }
    }

    // do not handle serde attributes parsing errors
    fn parse_serde(&mut self, arg: MetaArgument) {
        if arg.ident == "flatten" {
            self.serde.flatten = true;
        } else if arg.ident == "untagged" {
            self.serde.untagged = true;
        } else if arg.ident == "tag" {
            let Some((_, expr)) = arg.value else {
                return;
            };
            self.serde.tag = Some(expr);
        } else if arg.ident == "content" {
            let Some((_, expr)) = arg.value else {
                return;
            };
            self.serde.content = Some(expr);
        } else if arg.ident == "rename" {
            let Some((_, expr)) = arg.value else {
                return;
            };
            self.serde.rename = Some(expr);
        } else if arg.ident == "rename_all" {
            let Some((
                _,
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }),
            )) = arg.value
            else {
                return;
            };
            let Some(rule) = RenameRule::from_str(&lit_str.value()) else {
                return;
            };
            self.serde.rename_all = rule;
        } else if arg.ident == "rename_all_fields" {
            let Some((
                _,
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }),
            )) = arg.value
            else {
                return;
            };
            let Some(rule) = RenameRule::from_str(&lit_str.value()) else {
                return;
            };
            self.serde.rename_all_fields = rule;
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
                    meta.parse_serde(arg);
                }
            }
        }
        meta
    }
}
