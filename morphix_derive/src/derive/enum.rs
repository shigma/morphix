use std::mem::take;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_quote, parse_quote_spanned};

use crate::derive::meta::{AttributeKind, DeriveKind, GeneralImpl, ObserveMeta};
use crate::derive::{FMT_TRAITS, GenericsDetector, GenericsVisitor};

pub fn derive_observe_for_enum(
    input: &syn::DeriveInput,
    variants: &Punctuated<syn::Variant, syn::Token![,]>,
    input_meta: &ObserveMeta,
) -> TokenStream {
    let input_ident = &input.ident;
    // let ob_name = format!("{}Observer", input_ident);
    let ob_ident = format_ident!("{}Observer", input_ident);
    let ob_initial_ident = format_ident!("{}ObserverInitial", input_ident);
    let ob_variant_ident = format_ident!("{}ObserverVariant", input_ident);
    let input_vis = &input.vis;

    let mut generics_visitor = GenericsVisitor::default();
    generics_visitor.visit_derive_input(input);
    let head = generics_visitor.allocate_ty(parse_quote!(S));
    let depth = generics_visitor.allocate_ty(parse_quote!(N));
    let ob_lt = generics_visitor.allocate_lt(parse_quote!('ob));

    let mut ob_initial_variants = quote! {};
    let mut ob_variant_variants = quote! {};
    let mut initial_observe_arms = quote! {};
    let mut initial_flush_pats = quote! {};
    let mut variant_observe_arms = quote! {};
    let mut variant_refresh_arms = quote! {};
    let mut variant_flush_arms = quote! {};

    let mut errors = quote! {};
    let mut field_tys = vec![];
    let mut ob_field_tys = vec![];
    let mut skipped_tys = vec![];
    let mut has_variant = false;
    let mut has_initial = false;
    for variant in variants {
        let variant_ident = &variant.ident;
        let variant_name = variant.ident.to_string();
        if variant.fields.is_empty() {
            has_initial = true;
            let mut variant = variant.clone();
            take(&mut variant.attrs);
            ob_initial_variants.extend(quote! {
                #variant_ident,
            });
            initial_observe_arms.extend(quote! {
                #input_ident::#variant => #ob_initial_ident::#variant_ident,
            });
            initial_flush_pats.extend(quote! {
                | (#ob_initial_ident::#variant_ident, #input_ident::#variant)
            });
            continue;
        }

        has_variant = true;
        let variant_meta =
            ObserveMeta::parse_attrs(&variant.attrs, &mut errors, AttributeKind::Variant, DeriveKind::Enum);
        let tag_segment = if variant_meta.serde.untagged {
            None
        } else if let Some(rename) = &variant_meta.serde.rename {
            Some(quote! { #rename })
        } else if let Some(expr) = &input_meta.serde.content {
            Some(quote! { #expr })
        } else if input_meta.serde.untagged || input_meta.serde.tag.is_some() {
            None
        } else {
            let segment = input_meta.serde.rename_all.apply(&variant_name);
            Some(quote! { #segment })
        };

        let if_named: Vec<TokenStream> = match &variant.fields {
            syn::Fields::Named(_) => vec![quote! {}],
            _ => vec![],
        };

        let mut idents = vec![];
        let mut ob_idents = vec![];
        let mut value_idents = vec![];
        let mut flush_idents = vec![];
        let mut variant_fields = quote! {};
        let mut observe_fields = quote! {};
        let mut refresh_stmts = quote! {};
        let mut pre_flush_stmts = quote! {};
        let mut post_flush_stmts = quote! {};
        let mut flush_capacity = vec![];
        let mut has_skipped = false;

        let field_count = variant.fields.len();
        for (index, field) in variant.fields.iter().enumerate() {
            let field_meta =
                ObserveMeta::parse_attrs(&field.attrs, &mut errors, AttributeKind::Field, DeriveKind::Enum);
            let mut field_cloned = field.clone();
            field_cloned.attrs = vec![];
            let field_span = field_cloned.span();
            let field_trivial = !GenericsDetector::detect(&field.ty, &input.generics);
            let field_ty = &field.ty;
            let field_ident = &field.ident;
            let ob_ident = syn::Ident::new(&format!("u{}", index), field_span);
            let value_ident = syn::Ident::new(&format!("v{}", index), field_span);
            if let Some(field_ident) = field_ident {
                idents.push(quote! { #field_ident });
            }
            ob_idents.push(quote! { #ob_ident });
            value_idents.push(quote! { #value_ident });
            let observe_ident = if let Some(field_ident) = field_ident {
                field_ident
            } else {
                &value_ident
            };
            let flush_ident = if let Some(field_ident) = field_ident {
                field_ident
            } else {
                &ob_ident
            };

            if field_meta.skip || field_meta.serde.skip || field_meta.serde.skip_serializing {
                has_skipped = true;
                if !field_trivial {
                    skipped_tys.push(quote! { #field_ty });
                }
                variant_fields.extend(quote! {
                    #(#if_named #field_ident:)* ::morphix::helper::Pointer<#field_ty>,
                });
                observe_fields.extend(quote_spanned! { field_span =>
                    #(#if_named #field_ident:)* ::morphix::helper::Pointer::new(#observe_ident),
                });
                refresh_stmts.extend(quote_spanned! { field_span =>
                    ::morphix::helper::Pointer::set(#ob_ident, #value_ident);
                });
                if field_ident.is_none() {
                    flush_idents.push(quote! { _ });
                }
                continue;
            }

            flush_idents.push(quote! { #flush_ident });
            let ob_field_ty: syn::Type = match &field_meta.general_impl {
                None => parse_quote_spanned! { field_span =>
                    ::morphix::observe::DefaultObserver<#ob_lt, #field_ty>
                },
                Some(GeneralImpl { ob_ident, .. }) => parse_quote_spanned! { field_span =>
                    ::morphix::builtin::#ob_ident<#ob_lt, #field_ty>
                },
            };
            if !field_trivial {
                field_tys.push(quote! { #field_ty });
                ob_field_tys.push(quote! { #ob_field_ty });
            }
            variant_fields.extend(quote! {
                #(#if_named #field_ident:)* #ob_field_ty,
            });
            observe_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* ::morphix::observe::Observer::observe(#observe_ident),
            });
            refresh_stmts.extend(quote_spanned! { field_span =>
                ::morphix::observe::Observer::refresh(#ob_ident, #value_ident);
            });

            let mutable_ident = if let Some(field_ident) = &field_ident {
                let mut field_name = field_ident.to_string();
                if field_name.starts_with("r#") {
                    field_name = field_name[2..].to_string();
                }
                syn::Ident::new(&format!("mutations_{field_name}"), field_span)
            } else {
                syn::Ident::new(&format!("mutations_{index}"), field_span)
            };
            pre_flush_stmts.extend(
                if cfg!(feature = "delete")
                    && let Some(path) = field_meta.serde.skip_serializing_if
                {
                    quote_spanned! { field_span =>
                        let mut #mutable_ident = ::morphix::observe::SerializeObserver::flush::<A>(#flush_ident)?;
                        if !#mutable_ident.is_empty() && #path(::morphix::observe::Observer::as_inner(#flush_ident)) {
                            #mutable_ident = ::morphix::MutationKind::Delete.into();
                        }
                    }
                } else {
                    quote_spanned! { field_span =>
                        let #mutable_ident = ::morphix::observe::SerializeObserver::flush::<A>(#flush_ident)?;
                    }
                },
            );
            flush_capacity.push(quote_spanned! { field_span =>
                #mutable_ident.len()
            });

            let field_segment = if let Some(field_ident) = field_ident {
                // named
                if field_meta.serde.flatten {
                    None
                } else if let Some(rename) = &field_meta.serde.rename {
                    Some(quote! { #rename })
                } else {
                    let field_name = field_ident.to_string();
                    let segment = variant_meta
                        .serde
                        .rename_all
                        .or(input_meta.serde.rename_all_fields)
                        .apply(&field_name);
                    Some(quote! { #segment })
                }
            } else {
                // unnamed
                if field_count > 1 { Some(quote! { #index }) } else { None }
            };
            let segment_count = field_segment.iter().len() + tag_segment.iter().len();
            let segments = tag_segment.iter().chain(&field_segment);
            post_flush_stmts.extend(match segment_count {
                0 => quote! { mutations.extend(#mutable_ident); },
                1 => quote! { mutations.insert(#(#segments),*, #mutable_ident); },
                2 => quote! { mutations.insert2(#(#segments),*, #mutable_ident); },
                _ => unreachable!(),
            });
        }

        let variant_flush_expr = if flush_capacity.is_empty() {
            quote! { Ok(::morphix::Mutations::new()) }
        } else {
            quote! {{
                #pre_flush_stmts
                let mut mutations = ::morphix::Mutations::with_capacity(#(#flush_capacity)+*);
                #post_flush_stmts
                Ok(mutations)
            }}
        };

        match &variant.fields {
            syn::Fields::Named(_) => {
                if has_skipped {
                    flush_idents.push(quote! { .. });
                }
                ob_variant_variants.extend(quote! {
                    #variant_ident { #variant_fields },
                });
                variant_observe_arms.extend(quote! {
                    #input_ident::#variant_ident { #(#idents,)* } => Self::#variant_ident { #observe_fields },
                });
                variant_refresh_arms.extend(quote! {
                    (Self::#variant_ident { #(#idents: #ob_idents,)* }, #input_ident::#variant_ident { #(#idents: #value_idents,)* }) => { #refresh_stmts }
                });
                variant_flush_arms.extend(quote! {
                    Self::#variant_ident { #(#flush_idents),* } => #variant_flush_expr,
                });
            }
            syn::Fields::Unnamed(_) => {
                ob_variant_variants.extend(quote! {
                    #variant_ident(#variant_fields),
                });
                variant_observe_arms.extend(quote! {
                    #input_ident::#variant_ident(#(#value_idents),*) => Self::#variant_ident(#observe_fields),
                });
                variant_refresh_arms.extend(quote! {
                    (Self::#variant_ident(#(#ob_idents),*), #input_ident::#variant_ident(#(#value_idents),*)) => { #refresh_stmts }
                });
                variant_flush_arms.extend(quote! {
                    Self::#variant_ident(#(#flush_idents),*) => #variant_flush_expr,
                });
            }
            syn::Fields::Unit => {
                variant_observe_arms.extend(quote! {
                    #input_ident::#variant_ident => Self::#variant_ident,
                });
                variant_refresh_arms.extend(quote! {
                    (Self::#variant_ident, #input_ident::#variant_ident) => {},
                });
                variant_flush_arms.extend(quote! {
                    Self::#variant_ident => Ok(None),
                });
            }
        }
    }
    if !errors.is_empty() {
        return errors;
    }

    ob_initial_variants.extend(quote! { __None, });
    if has_variant {
        initial_observe_arms.extend(quote! {
            _ => #ob_initial_ident::__None,
        });
    }

    ob_variant_variants.extend(quote! { __None, });
    if has_initial {
        variant_observe_arms.extend(quote! {
            _ => Self::__None,
        });
    }
    variant_refresh_arms.extend(quote! {
        (Self::__None, _) => {},
    });
    variant_flush_arms.extend(quote! {
        Self::__None => Ok(::morphix::Mutations::new()),
    });

    let ob_flush_prefix_stmt = if has_initial {
        quote! {
            let __value = this.__ptr.as_deref();
            let __initial = this.__initial;
            this.__initial = #ob_initial_ident::new(__value);
        }
    } else {
        quote! {}
    };
    let ob_flush_suffix_stmt = if has_initial {
        quote! {
            match (__initial, __value) {
                #initial_flush_pats => Ok(::morphix::Mutations::new()),
                _ => Ok(::morphix::MutationKind::Replace(A::serialize_value(__value)?).into()),
            }
        }
    } else {
        quote! {
            Ok(::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?).into())
        }
    };

    let if_has_initial = match has_initial {
        true => vec![quote! {}],
        false => vec![],
    };
    let if_has_variant = match has_variant {
        true => vec![quote! {}],
        false => vec![],
    };

    let inconsistent_state = format!("inconsistent state for {ob_ident}");

    let mut input_generics = input.generics.clone();
    let input_predicates = match take(&mut input_generics.where_clause) {
        Some(where_clause) => where_clause.predicates.into_iter().collect::<Vec<_>>(),
        None => Default::default(),
    };
    let (input_impl_generics, input_type_generics, _) = input_generics.split_for_impl();

    let mut ob_variant_generics = input_generics.clone();
    ob_variant_generics.params.insert(0, parse_quote! { #ob_lt });

    let mut ob_generics = ob_variant_generics.clone();
    ob_generics.params.push(parse_quote! { #head: ?Sized });
    ob_generics
        .params
        .push(parse_quote! { #depth = ::morphix::helper::Zero });

    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
    let (ob_variant_impl_generics, ob_variant_type_generics, _) = ob_variant_generics.split_for_impl();

    let input_trivial = input.generics.params.is_empty();
    let input_serialize_predicates = if input_trivial {
        quote! {}
    } else {
        quote! {
            #input_ident #input_type_generics: ::serde::Serialize,
        }
    };
    let self_serialize_predicates = if input_trivial {
        quote! {}
    } else {
        quote! {
            Self: ::serde::Serialize,
        }
    };

    let derive_idents = &input_meta.derive.0;

    let ob_initial_metas = &input_meta.__initial;
    let ob_initial_impl = quote! {
        #(#[#ob_initial_metas])*
        #[derive(Clone, Copy)]
        #input_vis enum #ob_initial_ident {
            #ob_initial_variants
        }

        impl #ob_initial_ident {
            fn new #input_impl_generics(value: &#input_ident #input_type_generics) -> Self
            where
                #(#input_predicates,)*
            {
                match value {
                    #initial_observe_arms
                }
            }
        }
    };

    let ob_variant_metas = &input_meta.__variant;
    let ob_variant_impl = quote! {
        #(#[#ob_variant_metas])*
        #input_vis enum #ob_variant_ident #ob_variant_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe + #ob_lt),*
        {
            #ob_variant_variants
        }

        impl #ob_variant_impl_generics #ob_variant_ident #ob_variant_type_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe),*
        {
            fn observe(value: &#ob_lt mut #input_ident #input_type_generics) -> Self {
                match value {
                    #variant_observe_arms
                }
            }

            unsafe fn refresh(&mut self, value: &mut #input_ident #input_type_generics) {
                unsafe {
                    match (self, value) {
                        #variant_refresh_arms
                        _ => panic!(#inconsistent_state),
                    }
                }
            }

            fn flush<A: ::morphix::Adapter>(&mut self) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error>
            where
                #(#ob_field_tys: ::morphix::observe::SerializeObserver<#ob_lt>,)*
            {
                match self {
                    #variant_flush_arms
                }
            }
        }
    };

    let mut output = quote! {
        #(#[::std::prelude::v1::#derive_idents()])*
        #input_vis struct #ob_ident #ob_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe + #ob_lt),*
        {
            __ptr: ::morphix::helper::Pointer<#head>,
            #(#if_has_variant __mutated: bool,)*
            __phantom: ::std::marker::PhantomData<&#ob_lt mut #depth>,
            #(#if_has_initial __initial: #ob_initial_ident,)*
            #(#if_has_variant __variant: #ob_variant_ident #ob_variant_type_generics,)*
        }

        #(#if_has_initial #ob_initial_impl)*

        #(#if_has_variant #ob_variant_impl)*

        #[automatically_derived]
        impl #ob_impl_generics ::std::ops::Deref
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            type Target = ::morphix::helper::Pointer<#head>;
            fn deref(&self) -> &Self::Target {
                &self.__ptr
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::std::ops::DerefMut
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            fn deref_mut(&mut self) -> &mut Self::Target {
                #(#if_has_variant
                    self.__mutated = true;
                    self.__variant = #ob_variant_ident::__None;
                )*
                &mut self.__ptr
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::morphix::helper::AsNormalized
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        }

        #[automatically_derived]
        impl #ob_impl_generics ::morphix::observe::Observer<#ob_lt>
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#skipped_tys: #ob_lt,)*
            #(#field_tys: ::morphix::Observe,)*
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt,
            #depth: ::morphix::helper::Unsigned,
        {
            type Head = #head;
            type InnerDepth = #depth;

            fn uninit() -> Self {
                Self {
                    __ptr: ::morphix::helper::Pointer::uninit(),
                    #(#if_has_variant __mutated: false,)*
                    __phantom: ::std::marker::PhantomData,
                    #(#if_has_initial __initial: #ob_initial_ident::__None,)*
                    #(#if_has_variant __variant: #ob_variant_ident::__None,)*
                }
            }

            fn observe(value: &#ob_lt mut #head) -> Self {
                let __ptr = ::morphix::helper::Pointer::new(value);
                let __value = value.as_deref_mut();
                Self {
                    __ptr,
                    #(#if_has_variant __mutated: false,)*
                    __phantom: ::std::marker::PhantomData,
                    #(#if_has_initial __initial: #ob_initial_ident::new(__value),)*
                    #(#if_has_variant __variant: #ob_variant_ident::observe(__value),)*
                }
            }

            unsafe fn refresh(this: &mut Self, value: &mut #head) {
                ::morphix::helper::Pointer::set(this, value);
                #(#if_has_variant
                    let __value = value.as_deref_mut();
                    unsafe { this.__variant.refresh(__value) }
                )*
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::morphix::observe::SerializeObserver<#ob_lt>
        for #ob_ident #ob_type_generics
        where
            #input_serialize_predicates
            #(#input_predicates,)*
            #(#skipped_tys: #ob_lt,)*
            #(#field_tys: ::morphix::Observe,)*
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt,
            #depth: ::morphix::helper::Unsigned,
            #(#ob_field_tys: ::morphix::observe::SerializeObserver<#ob_lt>,)*
        {
            unsafe fn flush_unchecked<A: ::morphix::Adapter>(
                this: &mut Self,
            ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
                #ob_flush_prefix_stmt
                #(#if_has_variant
                    if !this.__mutated {
                        return this.__variant.flush::<A>();
                    }
                    this.__mutated = false;
                    this.__variant = #ob_variant_ident::__None;
                )*
                #ob_flush_suffix_stmt
            }
        }

        #[automatically_derived]
        impl #input_impl_generics ::morphix::Observe
        for #input_ident #input_type_generics
        where
            #self_serialize_predicates
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            type Observer<#ob_lt, #head, #depth> = #ob_ident #ob_type_generics
            where
                Self: #ob_lt,
                #(#field_tys: #ob_lt,)*
                #depth: ::morphix::helper::Unsigned,
                #head: ::morphix::helper::AsDerefMut<#depth, Target = Self> + ?Sized + #ob_lt,
            ;
            type Spec = ::morphix::observe::DefaultSpec;
        }
    };

    for path in &input_meta.derive.1 {
        // We just assume what the user wants is one of the standard formatting traits.
        if FMT_TRAITS.iter().any(|name| path.is_ident(name)) {
            output.extend(quote! {
                #[automatically_derived]
                impl #ob_impl_generics ::std::fmt::#path
                for #ob_ident #ob_type_generics
                where
                    #(#input_predicates,)*
                    #(#field_tys: ::morphix::Observe,)*
                    #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt,
                    #depth: ::morphix::helper::Unsigned,
                {
                    #[inline]
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        ::std::fmt::#path::fmt(self.as_deref(), f)
                    }
                }
            });
        }
    }

    if input_meta.expose {
        output
    } else {
        quote! {
            const _: () = {
                #output
            };
        }
    }
}
