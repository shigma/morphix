use std::mem::take;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::derive::GenericsAllocator;
use crate::derive::meta::ObserveMeta;

type WherePredicates = Punctuated<syn::WherePredicate, syn::Token![,]>;

pub fn derive_observe_for_enum(
    input: &syn::DeriveInput,
    variants: &Punctuated<syn::Variant, syn::Token![,]>,
    _input_meta: &ObserveMeta,
) -> TokenStream {
    let input_ident = &input.ident;
    // let ob_name = format!("{}Observer", input_ident);
    let ob_ident = format_ident!("{}Observer", input_ident);
    // let ob_variant_name = format!("{}ObserverVariant", input_ident);
    let ob_variant_ident = format_ident!("{}ObserverVariant", input_ident);
    let input_vis = &input.vis;

    let mut generics_allocator = GenericsAllocator::default();
    generics_allocator.visit_derive_input(input);
    let head = generics_allocator.allocate_ty(parse_quote!(S));
    let depth = generics_allocator.allocate_ty(parse_quote!(N));
    let ob_lt = generics_allocator.allocate_lt(parse_quote!('ob));

    let mut ty_variants = quote! {};
    let mut variant_observe_arms = quote! {};
    let mut variant_refresh_arms = quote! {};
    let mut variant_collect_arms = quote! {};
    // let mut inst_fields = vec![];
    // let mut refresh_stmts = vec![];
    // let mut collect_stmts = vec![];
    let mut ob_extra_predicates = WherePredicates::default();
    let mut ob_struct_extra_predicates = WherePredicates::default();
    let mut input_observe_observer_predicates = WherePredicates::default();
    let mut ob_default_extra_predicates = WherePredicates::default();
    let mut ob_debug_extra_predicates = WherePredicates::default();
    let mut ob_serialize_observer_extra_predicates = WherePredicates::default();

    for variant in variants {
        let variant_ident = &variant.ident;
        let mut ob_variant = variant.clone();
        match &mut ob_variant.fields {
            syn::Fields::Named(fields_named) => {
                let mut idents = vec![];
                let mut ob_idents = vec![];
                let mut segments = vec![];
                let mut value_idents = vec![];
                let mut observe_exprs = vec![];
                for (index, field) in fields_named.named.iter_mut().enumerate() {
                    let field_ident = &field.ident;
                    let field_ty = &field.ty;
                    field.ty = parse_quote! { ::morphix::observe::DefaultObserver<#ob_lt, #field_ty> };
                    let ob_ident = syn::Ident::new(&format!("u{}", index), field.span());
                    let value_ident = syn::Ident::new(&format!("v{}", index), field.span());
                    ob_idents.push(quote! { #ob_ident });
                    value_idents.push(quote! { #value_ident });
                    idents.push(quote! { #field_ident });
                    let segment = format!("{}", field_ident.as_ref().unwrap());
                    segments.push(quote! { #segment });
                    observe_exprs.push(quote! {
                        ::morphix::observe::Observer::observe(#field_ident)
                    });
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
                        ::morphix::observe::SerializeObserver::collect::<A>(#(#idents),*)
                    },
                    n => quote! {{
                        let mut mutations = ::std::vec::Vec::with_capacity(#n);
                        #(
                            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<A>(#idents)? {
                                mutation.path.push(#segments.into());
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
                    let field_ty = &field.ty;
                    field.ty = parse_quote! { ::morphix::observe::DefaultObserver<#ob_lt, #field_ty> };
                    let ob_ident = syn::Ident::new(&format!("u{}", index), field.span());
                    let value_ident = syn::Ident::new(&format!("v{}", index), field.span());
                    ob_idents.push(quote! { #ob_ident });
                    value_idents.push(quote! { #value_ident });
                    let segment = format!("{index}");
                    segments.push(quote! { #segment });
                    observe_exprs.push(quote! {
                        ::morphix::observe::Observer::observe(#value_ident)
                    });
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
                    1 => quote! {
                        ::morphix::observe::SerializeObserver::collect::<A>(#(#ob_idents),*)
                    },
                    n => quote! {{
                        let mut mutations = ::std::vec::Vec::with_capacity(#n);
                        #(
                            if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<A>(#ob_idents)? {
                                mutation.path.push(#segments.into());
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

    let inconsistent_state = format!("inconsistent state for {ob_ident}");

    let mut input_generics = input.generics.clone();
    let input_predicates = match take(&mut input_generics.where_clause) {
        Some(where_clause) => where_clause.predicates,
        None => Default::default(),
    };
    let (input_impl_generics, input_type_generics, _) = input_generics.split_for_impl();

    let mut ob_predicates = input_predicates.clone();
    ob_predicates.extend(ob_extra_predicates);

    let mut ob_struct_predicates = input_predicates.clone();
    ob_struct_predicates.extend(ob_struct_extra_predicates);

    let mut ob_default_predicates = ob_predicates.clone();
    ob_default_predicates.extend(ob_default_extra_predicates);

    let mut ob_variant_generics = input_generics.clone();
    ob_variant_generics.params.insert(0, parse_quote! { #ob_lt });
    let (ob_variant_impl_generics, ob_variant_type_generics, ob_variant_where_clause) =
        ob_variant_generics.split_for_impl();

    let mut ob_assignable_generics = input_generics.clone();
    let ob_assignable_predicates = ob_predicates.clone();
    ob_assignable_generics.params.insert(0, parse_quote! { #ob_lt });
    let mut ob_generics = ob_assignable_generics.clone();
    let mut ob_observer_extra_generics = Punctuated::<syn::GenericParam, syn::Token![,]>::default();

    let mut ob_observer_extra_predicates = WherePredicates::default();

    ob_generics.params.push(parse_quote! { #head: ?Sized });
    ob_generics
        .params
        .push(parse_quote! { #depth = ::morphix::helper::Zero });
    ob_assignable_generics.params.push(parse_quote! { #head });

    ob_observer_extra_predicates.push(parse_quote! {
        #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt
    });
    ob_observer_extra_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });
    ob_serialize_observer_extra_predicates.push(parse_quote! {
        #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt
    });
    ob_serialize_observer_extra_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });

    let mut ob_debug_predicates = ob_predicates.clone();
    ob_debug_predicates.extend(ob_debug_extra_predicates);

    let mut ob_observer_predicates = ob_predicates.clone();
    ob_observer_predicates.extend(ob_observer_extra_predicates);
    let mut ob_serialize_observer_predicates = ob_predicates.clone();
    ob_serialize_observer_predicates.extend(ob_serialize_observer_extra_predicates);
    let input_trivial = input.generics.params.is_empty();
    if !input_trivial {
        ob_serialize_observer_predicates.insert(
            0,
            parse_quote! {
                #input_ident #input_type_generics: ::serde::Serialize
            },
        );
    }

    input_observe_observer_predicates.push(parse_quote! { Self: #ob_lt });
    input_observe_observer_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });
    input_observe_observer_predicates.push(parse_quote! {
        #head: ::morphix::helper::AsDerefMut<#depth, Target = Self> + ?Sized + #ob_lt
    });

    let mut input_observe_predicates = ob_predicates.clone();
    if !input_trivial {
        input_observe_predicates.push(parse_quote! { Self: ::serde::Serialize });
    }

    let mut ob_observer_generics = ob_generics.clone();
    ob_observer_generics.params.extend(ob_observer_extra_generics);
    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
    let (ob_observer_impl_generics, _, _) = ob_observer_generics.split_for_impl();
    let (ob_assignable_impl_generics, ob_assignable_type_generics, _) = ob_assignable_generics.split_for_impl();

    let output = quote! {
        #input_vis struct #ob_ident #ob_generics
        where #ob_struct_predicates {
            __ptr: ::morphix::observe::ObserverPointer<#head>,
            __mutated: bool,
            __phantom: ::std::marker::PhantomData<&#ob_lt mut #depth>,
            __variant: ::std::mem::MaybeUninit<#ob_variant_ident #ob_variant_type_generics>,
        }

        #input_vis enum #ob_variant_ident #ob_variant_generics
        where #ob_struct_predicates {
            #ty_variants
        }

        impl #ob_variant_impl_generics #ob_variant_ident #ob_variant_type_generics #ob_variant_where_clause {
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

            fn collect<A: ::morphix::Adapter>(&mut self) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A::Value>>, A::Error> {
                match self {
                    #variant_collect_arms
                }
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::std::default::Default
        for #ob_ident #ob_type_generics
        where #ob_default_predicates {
            fn default() -> Self {
                Self {
                    __ptr: ::std::default::Default::default(),
                    __mutated: false,
                    __phantom: ::std::marker::PhantomData,
                    __variant: ::std::mem::MaybeUninit::uninit(),
                }
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::std::ops::Deref
        for #ob_ident #ob_type_generics
        where #ob_predicates {
            type Target = ::morphix::observe::ObserverPointer<#head>;
            fn deref(&self) -> &Self::Target {
                &self.__ptr
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::std::ops::DerefMut
        for #ob_ident #ob_type_generics
        where #ob_predicates {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.__ptr
            }
        }

        #[automatically_derived]
        impl #ob_assignable_impl_generics ::morphix::helper::Assignable
        for #ob_ident #ob_assignable_type_generics
        where #ob_assignable_predicates {
            type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        }

        #[automatically_derived]
        impl #ob_observer_impl_generics ::morphix::observe::Observer<#ob_lt>
        for #ob_ident #ob_type_generics
        where #ob_observer_predicates {
            type Head = #head;
            type InnerDepth = #depth;
            type OuterDepth = ::morphix::helper::Zero;

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
        impl #ob_observer_impl_generics ::morphix::observe::SerializeObserver<#ob_lt>
        for #ob_ident #ob_type_generics
        where #ob_serialize_observer_predicates {
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
        where #input_observe_predicates {
            type Observer<#ob_lt, #head, #depth> = #ob_ident #ob_type_generics
            where #input_observe_observer_predicates;
            type Spec = ::morphix::observe::DefaultSpec;
        }
    };

    quote! {
        const _: () = {
            #output
        };
    }
}
