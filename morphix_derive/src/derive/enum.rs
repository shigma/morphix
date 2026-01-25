use std::mem::take;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{parse_quote, parse_quote_spanned};

use crate::derive::meta::{AttributeKind, DeriveKind, GeneralImpl, ObserveMeta};
use crate::derive::{FMT_TRAITS, GenericsDetector, GenericsVisitor, StripAttributes};

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
    let mut has_variant = false;
    let mut has_initial = false;
    for variant in variants {
        let variant_ident = &variant.ident;
        let variant_name = variant.ident.to_string();
        let variant_meta =
            ObserveMeta::parse_attrs(&variant.attrs, &mut errors, AttributeKind::Variant, DeriveKind::Enum);
        let mut ob_variant = variant.clone();
        let tag_segment = if let Some(expr) = &input_meta.serde.content {
            Some(quote! { #expr })
        } else if input_meta.serde.untagged || input_meta.serde.tag.is_some() {
            None
        } else if let Some(rename) = &variant_meta.serde.rename {
            Some(quote! { #rename })
        } else {
            let segment = input_meta.serde.rename_all.apply(&variant_name);
            Some(quote! { #segment })
        };
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
        match &mut ob_variant.fields {
            syn::Fields::Named(fields_named) => {
                let mut idents = vec![];
                let mut ob_idents = vec![];
                let mut field_segments = vec![];
                let mut value_idents = vec![];
                let mut observe_exprs = vec![];
                for (index, field) in fields_named.named.iter_mut().enumerate() {
                    let field_meta =
                        ObserveMeta::parse_attrs(&field.attrs, &mut errors, AttributeKind::Field, DeriveKind::Enum);
                    let mut field_cloned = field.clone();
                    field_cloned.attrs = vec![];
                    let field_span = field_cloned.span();
                    let field_ident = &field.ident;
                    let field_trivial = !GenericsDetector::detect(&field.ty, &input.generics);
                    let field_ty = &field.ty;
                    let ob_ident = syn::Ident::new(&format!("u{}", index), field_span);
                    let value_ident = syn::Ident::new(&format!("v{}", index), field_span);
                    ob_idents.push(quote! { #ob_ident });
                    value_idents.push(quote! { #value_ident });
                    idents.push(quote! { #field_ident });
                    if field_meta.serde.flatten {
                        field_segments.push(None);
                    } else if let Some(rename) = &field_meta.serde.rename {
                        field_segments.push(Some(quote! { #rename }));
                    } else {
                        let field_name = field_ident.as_ref().unwrap().to_string();
                        let segment = variant_meta
                            .serde
                            .rename_all
                            .or(input_meta.serde.rename_all_fields)
                            .apply(&field_name);
                        field_segments.push(Some(quote! { #segment }));
                    };
                    observe_exprs.push(quote! {
                        ::morphix::observe::Observer::observe(#field_ident)
                    });
                    let ob_field_ty: syn::Type = match &field_meta.general_impl {
                        None => parse_quote_spanned! { field_span =>
                            ::morphix::observe::DefaultObserver<#ob_lt, #field_ty>
                        },
                        Some(GeneralImpl { ob_ident, .. }) => parse_quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<#ob_lt, #field_ty>
                        },
                    };
                    if !field_trivial {
                        field_tys.push(quote! { #field_ty });
                        ob_field_tys.push(quote! { #ob_field_ty });
                    }
                    field.ty = ob_field_ty;
                }
                variant_observe_arms.extend(quote! {
                    #input_ident::#variant_ident { #(#idents),* } => Self::#variant_ident { #(#idents: #observe_exprs),* },
                });
                variant_refresh_arms.extend(quote! {
                    (Self::#variant_ident { #(#idents: #ob_idents),* }, #input_ident::#variant_ident { #(#idents: #value_idents),* }) => {
                        #(::morphix::observe::Observer::refresh(#ob_idents, #value_idents));*
                    }
                });
                let flush_stmts = idents.iter().zip(field_segments).map(|(ident, field_segment)| {
                    let segment_count = field_segment.iter().len() + tag_segment.iter().len();
                    let segments = field_segment.iter().chain(&tag_segment);
                    let children = quote! {
                        ::morphix::observe::SerializeObserver::flush::<A>(#ident)?
                    };
                    match segment_count {
                        0 => quote! { mutations.extend(#children); },
                        1 => quote! { mutations.insert(#(#segments),*, #children); },
                        2 => quote! { mutations.insert2(#(#segments),*, #children); },
                        _ => unreachable!(),
                    }
                });
                variant_flush_arms.extend(quote! {
                    Self::#variant_ident { #(#idents),* } => {
                        let mut mutations = ::morphix::Mutations::new();
                        #(#flush_stmts)*
                        Ok(mutations)
                    },
                });
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let mut ob_idents = vec![];
                let mut value_idents = vec![];
                let mut field_segments = vec![];
                let mut observe_exprs = vec![];
                for (index, field) in fields_unnamed.unnamed.iter_mut().enumerate() {
                    let field_meta =
                        ObserveMeta::parse_attrs(&field.attrs, &mut errors, AttributeKind::Field, DeriveKind::Enum);
                    let mut field_cloned = field.clone();
                    field_cloned.attrs = vec![];
                    let field_span = field_cloned.span();
                    let field_trivial = !GenericsDetector::detect(&field.ty, &input.generics);
                    let field_ty = &field.ty;
                    let ob_ident = syn::Ident::new(&format!("u{}", index), field_span);
                    let value_ident = syn::Ident::new(&format!("v{}", index), field_span);
                    ob_idents.push(quote! { #ob_ident });
                    value_idents.push(quote! { #value_ident });
                    field_segments.push(quote! { #index });
                    observe_exprs.push(quote! {
                        ::morphix::observe::Observer::observe(#value_ident)
                    });
                    let ob_field_ty: syn::Type = match &field_meta.general_impl {
                        None => parse_quote_spanned! { field_span =>
                            ::morphix::observe::DefaultObserver<#ob_lt, #field_ty>
                        },
                        Some(GeneralImpl { ob_ident, .. }) => parse_quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<#ob_lt, #field_ty>
                        },
                    };
                    if !field_trivial {
                        field_tys.push(quote! { #field_ty });
                        ob_field_tys.push(quote! { #ob_field_ty });
                    }
                    field.ty = ob_field_ty;
                }
                variant_observe_arms.extend(quote! {
                    #input_ident::#variant_ident(#(#value_idents),*) => Self::#variant_ident(#(#observe_exprs),*),
                });
                variant_refresh_arms.extend(quote! {
                    (Self::#variant_ident(#(#ob_idents),*), #input_ident::#variant_ident(#(#value_idents),*)) => {
                        #(::morphix::observe::Observer::refresh(#ob_idents, #value_idents));*
                    }
                });
                let flush_stmts = ob_idents.iter().zip(field_segments).map(|(ident, field_segment)| {
                    let children = quote! {
                        ::morphix::observe::SerializeObserver::flush::<A>(#ident)?
                    };
                    match &tag_segment {
                        Some(tag_segment) => quote! { mutations.insert2(#field_segment, #tag_segment, #children); },
                        None => quote! { mutations.insert(#field_segment, #children); },
                    }
                });
                variant_flush_arms.extend(quote! {
                    Self::#variant_ident(#(#ob_idents),*) => {
                        let mut mutations = ::morphix::Mutations::new();
                        #(#flush_stmts)*
                        Ok(mutations)
                    },
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
        StripAttributes.visit_variant_mut(&mut ob_variant);
        ob_variant_variants.extend(quote! {
            #ob_variant,
        });
    }
    if !errors.is_empty() {
        return errors;
    }

    ob_initial_variants.extend(quote! {
        __None,
    });
    if has_variant {
        initial_observe_arms.extend(quote! {
            _ => #ob_initial_ident::__None,
        });
    }

    ob_variant_variants.extend(quote! {
        __None,
    });
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

    let mut ob_generics = input_generics.clone();
    ob_generics.params.insert(0, parse_quote! { #ob_lt });
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

    let ob_initial_impl = quote! {
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

    let ob_variant_impl = quote! {
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
            __ptr: ::morphix::observe::ObserverPointer<#head>,
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
            type Target = ::morphix::observe::ObserverPointer<#head>;
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
            #(#field_tys: ::morphix::Observe,)*
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt,
            #depth: ::morphix::helper::Unsigned,
        {
            type Head = #head;
            type InnerDepth = #depth;

            fn uninit() -> Self {
                Self {
                    __ptr: ::morphix::observe::ObserverPointer::uninit(),
                    #(#if_has_variant __mutated: false,)*
                    __phantom: ::std::marker::PhantomData,
                    #(#if_has_initial __initial: #ob_initial_ident::__None,)*
                    #(#if_has_variant __variant: #ob_variant_ident::__None,)*
                }
            }

            fn observe(value: &#ob_lt mut #head) -> Self {
                let __ptr = ::morphix::observe::ObserverPointer::new(value);
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
                ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
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

    quote! {
        const _: () = {
            #output
        };
    }
}
