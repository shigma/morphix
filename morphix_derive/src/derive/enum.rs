use std::mem::take;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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
    // let ob_variant_name = format!("{}ObserverVariant", input_ident);
    let ob_variant_ident = format_ident!("{}ObserverVariant", input_ident);
    let input_vis = &input.vis;

    let mut generics_visitor = GenericsVisitor::default();
    generics_visitor.visit_derive_input(input);
    let head = generics_visitor.allocate_ty(parse_quote!(S));
    let depth = generics_visitor.allocate_ty(parse_quote!(N));
    let ob_lt = generics_visitor.allocate_lt(parse_quote!('ob));

    let mut ty_variants = quote! {};
    let mut variant_observe_arms = quote! {};
    let mut variant_refresh_arms = quote! {};
    let mut variant_collect_arms = quote! {};

    let mut errors = quote! {};
    let mut field_tys = vec![];
    let mut ob_field_tys = vec![];
    for variant in variants {
        let variant_ident = &variant.ident;
        let variant_name = variant.ident.to_string();
        let mut ob_variant = variant.clone();
        take(&mut ob_variant.attrs);
        let push_tag = if let Some(expr) = &input_meta.serde.content {
            quote! {
                mutation.path.push(#expr.into());
            }
        } else if input_meta.serde.untagged || input_meta.serde.tag.is_some() {
            quote! {}
        } else {
            quote! {
                mutation.path.push(#variant_name.into());
            }
        };
        match &mut ob_variant.fields {
            syn::Fields::Named(fields_named) => {
                let mut idents = vec![];
                let mut ob_idents = vec![];
                let mut segments = vec![];
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
                    let segment = format!("{}", field_ident.as_ref().unwrap());
                    segments.push(quote! { #segment });
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
                let variant_collect_expr = match fields_named.named.len() {
                    0 => quote! { Ok(None) },
                    1 => quote! {
                        match ::morphix::observe::SerializeObserver::collect::<A>(#(#idents),*) {
                            Ok(Some(mut mutation)) => {
                                mutation.path.push(#(#segments.into()),*);
                                #push_tag
                                Ok(Some(mutation))
                            },
                            result => result,
                        }
                    },
                    n => quote! {{
                        let mut mutations = ::std::vec::Vec::with_capacity(#n);
                        #(
                            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<A>(#idents)? {
                                mutation.path.push(#segments.into());
                                #push_tag
                                mutations.push(mutation);
                            }
                        )*
                        Ok(::morphix::Mutation::coalesce(mutations))
                    }},
                };
                variant_collect_arms.extend(quote! {
                    Self::#variant_ident { #(#idents),* } => #variant_collect_expr,
                });
            }
            syn::Fields::Unnamed(fields_unnamed) => {
                let mut ob_idents = vec![];
                let mut value_idents = vec![];
                let mut segments = vec![];
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
                    let segment = format!("{index}");
                    segments.push(quote! { #segment });
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
                let variant_collect_expr = match fields_unnamed.unnamed.len() {
                    0 => quote! { Ok(None) },
                    1 => match push_tag.is_empty() {
                        true => quote! {
                            ::morphix::observe::SerializeObserver::collect::<A>(#(#ob_idents),*)
                        },
                        false => quote! {
                            match ::morphix::observe::SerializeObserver::collect::<A>(#(#ob_idents),*) {
                                Ok(Some(mut mutation)) => {
                                    #push_tag
                                    Ok(Some(mutation))
                                },
                                result => result,
                            }
                        },
                    },
                    n => quote! {{
                        let mut mutations = ::std::vec::Vec::with_capacity(#n);
                        #(
                            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<A>(#ob_idents)? {
                                mutation.path.push(#segments.into());
                                #push_tag
                                mutations.push(mutation);
                            }
                        )*
                        Ok(::morphix::Mutation::coalesce(mutations))
                    }},
                };
                variant_collect_arms.extend(quote! {
                    Self::#variant_ident(#(#ob_idents),*) => #variant_collect_expr,
                });
            }
            syn::Fields::Unit => {
                variant_observe_arms.extend(quote! {
                    #input_ident::#variant_ident => Self::#variant_ident,
                });
                variant_refresh_arms.extend(quote! {
                    (Self::#variant_ident, #input_ident::#variant_ident) => {},
                });
                variant_collect_arms.extend(quote! {
                    Self::#variant_ident => Ok(None),
                });
            }
        }
        ty_variants.extend(quote! {
            #ob_variant,
        });
    }
    if !errors.is_empty() {
        return errors;
    }

    let inconsistent_state = format!("inconsistent state for {ob_ident}");

    let mut input_generics = input.generics.clone();
    let input_predicates = match take(&mut input_generics.where_clause) {
        Some(where_clause) => where_clause.predicates.into_iter().collect::<Vec<_>>(),
        None => Default::default(),
    };
    let (input_impl_generics, input_type_generics, _) = input_generics.split_for_impl();

    let mut ob_variant_generics = input_generics.clone();
    ob_variant_generics.params.insert(0, parse_quote! { #ob_lt });

    let mut ob_assignable_generics = input_generics.clone();
    ob_assignable_generics.params.insert(0, parse_quote! { #ob_lt });
    ob_assignable_generics.params.push(parse_quote! { #head });

    let mut ob_generics = input_generics.clone();
    ob_generics.params.insert(0, parse_quote! { #ob_lt });
    ob_generics.params.push(parse_quote! { #head: ?Sized });
    ob_generics
        .params
        .push(parse_quote! { #depth = ::morphix::helper::Zero });

    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
    let (ob_variant_impl_generics, ob_variant_type_generics, _) = ob_variant_generics.split_for_impl();
    let (ob_assignable_impl_generics, ob_assignable_type_generics, _) = ob_assignable_generics.split_for_impl();

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

    let mut output = quote! {
        #(#[::std::prelude::v1::#derive_idents()])*
        #input_vis struct #ob_ident #ob_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe + #ob_lt),*
        {
            __ptr: ::morphix::observe::ObserverPointer<#head>,
            __mutated: bool,
            __phantom: ::std::marker::PhantomData<&#ob_lt mut #depth>,
            __variant: ::std::mem::MaybeUninit<#ob_variant_ident #ob_variant_type_generics>,
        }

        #input_vis enum #ob_variant_ident #ob_variant_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe + #ob_lt),*
        {
            #ty_variants
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

            fn collect<A: ::morphix::Adapter>(&mut self) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A::Value>>, A::Error>
            where
                #(#ob_field_tys: ::morphix::observe::SerializeObserver<#ob_lt>,)*
            {
                match self {
                    #variant_collect_arms
                }
            }
        }

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
                &mut self.__ptr
            }
        }

        #[automatically_derived]
        impl #ob_assignable_impl_generics ::morphix::helper::Assignable
        for #ob_ident #ob_assignable_type_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
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
            type OuterDepth = ::morphix::helper::Zero;

            fn uninit() -> Self {
                Self {
                    __ptr: ::morphix::observe::ObserverPointer::default(),
                    __mutated: false,
                    __phantom: ::std::marker::PhantomData,
                    __variant: ::std::mem::MaybeUninit::uninit(),
                }
            }

            fn observe(value: &#ob_lt mut #head) -> Self {
                let __ptr = ::morphix::observe::ObserverPointer::new(value);
                let __value = value.as_deref_mut();
                Self {
                    __ptr,
                    __mutated: false,
                    __phantom: ::std::marker::PhantomData,
                    __variant: ::std::mem::MaybeUninit::new(#ob_variant_ident::observe(__value)),
                }
            }

            unsafe fn refresh(this: &mut Self, value: &mut #head) {
                ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
                let __value = value.as_deref_mut();
                unsafe { this.__variant.assume_init_mut().refresh(__value) }
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
            unsafe fn collect_unchecked<A: ::morphix::Adapter>(
                this: &mut Self,
            ) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A::Value>>, A::Error> {
                if this.__mutated {
                    return Ok(Some(::morphix::Mutation {
                        path: ::morphix::Path::new(),
                        kind: ::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?),
                    }));
                };
                unsafe { this.__variant.assume_init_mut() }.collect::<A>()
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
