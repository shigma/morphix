use std::mem::take;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_quote, parse_quote_spanned};

type WherePredicates = Punctuated<syn::WherePredicate, syn::Token![,]>;

struct GeneralImpl {
    ob_ident: syn::Ident,
    spec_ident: syn::Ident,
    bounds: Punctuated<syn::TypeParamBound, syn::Token![+]>,
}

#[derive(Default)]
struct ObserveMeta {
    general_impl: Option<GeneralImpl>,
    deref: Option<Span>,
}

impl ObserveMeta {
    fn parse_attrs(attrs: &[syn::Attribute], errors: &mut Vec<syn::Error>) -> Self {
        let mut meta = ObserveMeta::default();
        for attr in attrs {
            if !attr.path().is_ident("observe") {
                continue;
            }
            let syn::Meta::List(meta_list) = &attr.meta else {
                errors.push(syn::Error::new(
                    attr.span(),
                    "the 'observe' attribute must be in the form of #[observe(...)]",
                ));
                continue;
            };
            let args = match Punctuated::<syn::Ident, syn::Token![,]>::parse_terminated.parse2(meta_list.tokens.clone())
            {
                Ok(args) => args,
                Err(err) => {
                    errors.push(err);
                    continue;
                }
            };
            for ident in args {
                if ident == "hash" {
                    meta.general_impl = Some(GeneralImpl {
                        ob_ident: syn::Ident::new("HashObserver", ident.span()),
                        spec_ident: syn::Ident::new("HashSpec", ident.span()),
                        bounds: parse_quote! { ::std::hash::Hash },
                    });
                } else if ident == "noop" {
                    meta.general_impl = Some(GeneralImpl {
                        ob_ident: syn::Ident::new("NoopObserver", ident.span()),
                        spec_ident: syn::Ident::new("DefaultSpec", ident.span()),
                        bounds: Default::default(),
                    });
                } else if ident == "shallow" {
                    meta.general_impl = Some(GeneralImpl {
                        ob_ident: syn::Ident::new("ShallowObserver", ident.span()),
                        spec_ident: syn::Ident::new("DefaultSpec", ident.span()),
                        bounds: Default::default(),
                    });
                } else if ident == "snapshot" {
                    meta.general_impl = Some(GeneralImpl {
                        ob_ident: syn::Ident::new("SnapshotObserver", ident.span()),
                        spec_ident: syn::Ident::new("SnapshotSpec", ident.span()),
                        bounds: parse_quote! { ::std::clone::Clone + ::std::cmp::PartialEq },
                    });
                } else if ident == "deref" {
                    meta.deref = Some(ident.span());
                } else {
                    errors.push(syn::Error::new(
                        ident.span(),
                        "unknown argument, expected 'deref', 'hash', 'noop', 'shallow' or 'snapshot'",
                    ));
                }
            }
        }
        meta
    }
}

pub fn derive_observe(mut input: syn::DeriveInput) -> TokenStream {
    let input_ident = &input.ident;
    let mut errors = vec![];
    let input_meta = ObserveMeta::parse_attrs(&input.attrs, &mut errors);
    if !errors.is_empty() {
        return errors.into_iter().map(|error| error.to_compile_error()).collect();
    }
    if let Some(GeneralImpl {
        ob_ident,
        spec_ident,
        bounds,
    }) = input_meta.general_impl
    {
        let mut where_predicates = match take(&mut input.generics.where_clause) {
            Some(where_clause) => where_clause.predicates,
            None => Default::default(),
        };
        if !bounds.is_empty() {
            where_predicates.push(parse_quote! { Self: #bounds });
        }
        let (impl_generics, type_generics, _) = input.generics.split_for_impl();
        return quote! {
            const _: () = {
                #[automatically_derived]
                impl #impl_generics ::morphix::Observe for #input_ident #type_generics where #where_predicates {
                    type Observer<'morphix, __S, __N>
                        = ::morphix::observe::#ob_ident<'morphix, __S, __N>
                    where
                        Self: 'morphix,
                        __N: ::morphix::helper::Unsigned,
                        __S: ::morphix::helper::AsDerefMut<__N, Target = Self> + ?Sized + 'morphix;

                    type Spec = ::morphix::observe::#spec_ident;
                }
            };
        };
    }

    let ob_ident = format_ident!("{}Observer", input_ident);
    let input_vis = &input.vis;

    let mut ob_assignable_generics = input.generics.clone();
    ob_assignable_generics.params.insert(0, parse_quote! { 'morphix });
    let mut ob_generics = ob_assignable_generics.clone();
    ob_generics.params.push(parse_quote! { __S: ?Sized });
    ob_generics.params.push(parse_quote! { __N = Zero });
    ob_assignable_generics.params.push(parse_quote! { __S });

    let mut ob_where_predicates = match take(&mut ob_generics.where_clause) {
        Some(where_clause) => where_clause.predicates,
        None => Default::default(),
    };
    let mut ob_struct_where_predicates = ob_where_predicates.clone();
    let mut input_observer_where_predicates = Punctuated::<syn::WherePredicate, syn::Token![,]>::new();
    for param in &input.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let ident = &type_param.ident;
            ob_where_predicates.push(parse_quote! { #ident: Observe });
            ob_struct_where_predicates.push(parse_quote! { #ident: Observe + 'morphix });
            input_observer_where_predicates.push(parse_quote! { #ident: 'morphix });
        }
    }
    input_observer_where_predicates.push(parse_quote! { Self: 'morphix });
    input_observer_where_predicates.push(parse_quote! { __N: Unsigned });
    input_observer_where_predicates.push(parse_quote! {
        __S: AsDerefMut<__N, Target = Self> + ?Sized + 'morphix
    });

    let (input_impl_generics, type_generics, _) = input.generics.split_for_impl();
    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
    let (ob_assignable_impl_generics, ob_assignable_type_generics, _) = ob_assignable_generics.split_for_impl();

    let mut input_impl_observe_where_predicates = ob_where_predicates.clone();
    input_impl_observe_where_predicates.push(parse_quote! { Self: ::serde::Serialize });
    let mut ob_default_where_predicates = ob_where_predicates.clone();
    let mut ob_observer_where_predicates = ob_where_predicates.clone();
    ob_observer_where_predicates.push(parse_quote! { #input_ident #type_generics: ::serde::Serialize });
    ob_observer_where_predicates.push(parse_quote! { __N: Unsigned });
    ob_observer_where_predicates.push(parse_quote! {
        __S: AsDerefMut<__N, Target = #input_ident #type_generics> + 'morphix
    });

    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    let mut default_fields = vec![];
    let mut collect_stmts = vec![];
    let mut ob_serialize_observer_where_predicates = WherePredicates::default();
    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            // let mut deref_fields = vec![];
            for field in named {
                let field_meta = ObserveMeta::parse_attrs(&field.attrs, &mut errors);
                let field_ident = field.ident.as_ref().unwrap();
                let mut field_cloned = field.clone();
                field_cloned.attrs = vec![];
                let field_span = field_cloned.span();
                let field_ty = &field.ty;
                let ob_field_ty = match &field_meta.general_impl {
                    None => {
                        if let Some(_span) = field_meta.deref {
                            quote_spanned! { field_span =>
                                <#field_ty as Observe>::Observer<'morphix, __S, Succ<__N>>
                            }
                        } else {
                            quote_spanned! { field_span =>
                                DefaultObserver<'morphix, #field_ty>
                            }
                        }
                    }
                    Some(GeneralImpl { ob_ident, .. }) => {
                        quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<'morphix, #field_ty>
                        }
                    }
                };
                type_fields.push(quote_spanned! { field_span =>
                    pub #field_ident: #ob_field_ty,
                });
                inst_fields.push(quote_spanned! { field_span =>
                    #field_ident: Observer::observe(&mut __value.#field_ident),
                });
                default_fields.push(quote_spanned! { field_span =>
                    #field_ident: Default::default(),
                });
                collect_stmts.push(quote_spanned! { field_span =>
                    if let Some(mut mutation) = SerializeObserver::collect::<A>(&mut this.#field_ident)? {
                        mutation.path.push(stringify!(#field_ident).into());
                        mutations.push(mutation);
                    }
                });
                ob_default_where_predicates.push(parse_quote_spanned! { field_span =>
                    #ob_field_ty: Default
                });
                ob_serialize_observer_where_predicates.push(parse_quote! {
                    #field_ty: Observe
                });
                ob_serialize_observer_where_predicates.push(parse_quote! {
                    #ob_field_ty: SerializeObserver<'morphix>
                });
            }
        }
        _ => {
            return syn::Error::new(input.span(), "Observe can only be derived for named structs").to_compile_error();
        }
    };
    if !errors.is_empty() {
        return errors.into_iter().map(|error| error.to_compile_error()).collect();
    }

    ob_serialize_observer_where_predicates.push(parse_quote! { #input_ident #type_generics: ::serde::Serialize });
    ob_serialize_observer_where_predicates.push(parse_quote! {
        __S: AsDerefMut<__N, Target = #input_ident #type_generics> + 'morphix
    });
    ob_serialize_observer_where_predicates.push(parse_quote! { __N: Unsigned });

    quote! {
        const _: () = {
            // #[allow(unused_imports)]
            use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
            // #[allow(unused_imports)]
            use ::morphix::observe::{DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver};

            #[allow(private_interfaces)]
            #input_vis struct #ob_ident #ob_generics
            where #ob_struct_where_predicates {
                __ptr: ObserverPointer<__S>,
                __mutated: bool,
                __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
                #(#type_fields)*
            }

            #[automatically_derived]
            impl #ob_impl_generics Default
            for #ob_ident #ob_type_generics
            where #ob_default_where_predicates {
                fn default() -> Self {
                    Self {
                        __ptr: ObserverPointer::default(),
                        __mutated: false,
                        __phantom: ::std::marker::PhantomData,
                        #(#default_fields)*
                    }
                }
            }

            #[automatically_derived]
            impl #ob_impl_generics ::std::ops::Deref
            for #ob_ident #ob_type_generics
            where #ob_where_predicates {
                type Target = ObserverPointer<__S>;
                fn deref(&self) -> &Self::Target {
                    &self.__ptr
                }
            }

            #[automatically_derived]
            impl #ob_impl_generics ::std::ops::DerefMut
            for #ob_ident #ob_type_generics
            where #ob_where_predicates {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.__mutated = true;
                    &mut self.__ptr
                }
            }

            #[automatically_derived]
            impl #ob_assignable_impl_generics ::morphix::helper::Assignable
            for #ob_ident #ob_assignable_type_generics
            where #ob_where_predicates {
                type Depth = Succ<Zero>;
            }

            #[automatically_derived]
            impl #ob_impl_generics Observer<'morphix>
            for #ob_ident #ob_type_generics
            where #ob_observer_where_predicates {
                type Head = __S;
                type InnerDepth = __N;
                type OuterDepth = Zero;

                fn observe(value: &'morphix mut __S) -> Self {
                    let __ptr = ObserverPointer::new(value);
                    let __value = value.as_deref_mut();
                    Self {
                        __ptr,
                        __mutated: false,
                        __phantom: ::std::marker::PhantomData,
                        #(#inst_fields)*
                    }
                }
            }

            #[automatically_derived]
            impl #ob_impl_generics SerializeObserver<'morphix>
            for #ob_ident #ob_type_generics
            where #ob_serialize_observer_where_predicates {
                unsafe fn collect_unchecked<A: ::morphix::Adapter>(
                    this: &mut Self,
                ) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A>>, A::Error> {
                    if this.__mutated {
                        return Ok(Some(::morphix::Mutation {
                            path: ::morphix::Path::new(),
                            kind: ::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?),
                        }));
                    };
                    let mut mutations = ::std::vec::Vec::new();
                    #(#collect_stmts)*
                    Ok(::morphix::Mutation::coalesce(mutations))
                }
            }

            #[automatically_derived]
            impl #input_impl_generics Observe
            for #input_ident #type_generics
            where #input_impl_observe_where_predicates {
                type Observer<'morphix, __S, __N> = #ob_ident #ob_type_generics
                where #input_observer_where_predicates;
                type Spec = ::morphix::observe::DefaultSpec;
            }
        };
    }
}
