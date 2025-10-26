use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

struct ObserveArguments(Punctuated<syn::Ident, syn::Token![,]>);

impl Parse for ObserveArguments {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args = Punctuated::parse_terminated(input)?;
        Ok(Self(args))
    }
}

#[derive(Default)]
struct ObserveMeta {
    ob_with_spec: Option<(syn::Ident, syn::Ident, Punctuated<syn::TypeParamBound, syn::Token![+]>)>,
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
            let args: ObserveArguments = match syn::parse2(meta_list.tokens.clone()) {
                Ok(args) => args,
                Err(err) => {
                    errors.push(err);
                    continue;
                }
            };
            for ident in args.0 {
                if ident == "hash" {
                    meta.ob_with_spec = Some((
                        syn::Ident::new("HashObserver", ident.span()),
                        syn::Ident::new("HashSpec", ident.span()),
                        syn::parse_quote! { ::std::hash::Hash },
                    ));
                } else if ident == "noop" {
                    meta.ob_with_spec = Some((
                        syn::Ident::new("NoopObserver", ident.span()),
                        syn::Ident::new("DefaultSpec", ident.span()),
                        Default::default(),
                    ));
                } else if ident == "shallow" {
                    meta.ob_with_spec = Some((
                        syn::Ident::new("ShallowObserver", ident.span()),
                        syn::Ident::new("DefaultSpec", ident.span()),
                        Default::default(),
                    ));
                } else if ident == "snapshot" {
                    meta.ob_with_spec = Some((
                        syn::Ident::new("SnapshotObserver", ident.span()),
                        syn::Ident::new("SnapshotSpec", ident.span()),
                        syn::parse_quote! { ::std::clone::Clone + ::std::cmp::PartialEq },
                    ));
                } else {
                    errors.push(syn::Error::new(
                        ident.span(),
                        "unknown argument, expected 'hash', 'noop', 'shallow' or 'snapshot'",
                    ));
                }
            }
        }
        meta
    }
}

pub fn derive_observe(input: syn::DeriveInput) -> TokenStream {
    let input_ident = &input.ident;
    let mut errors = vec![];
    let input_meta = ObserveMeta::parse_attrs(&input.attrs, &mut errors);
    if !errors.is_empty() {
        return errors.into_iter().map(|error| error.to_compile_error()).collect();
    }
    if let Some((ob_ident, spec_ident, bounds)) = input_meta.ob_with_spec {
        let mut where_predicates = match &input.generics.where_clause {
            Some(where_clause) => where_clause.predicates.clone(),
            None => Default::default(),
        };
        if !bounds.is_empty() {
            where_predicates.push(syn::parse_quote! { Self: #bounds });
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
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    let mut default_fields = vec![];
    let mut collect_stmts = vec![];

    let mut ob_generics = input.generics.clone();
    ob_generics.params.insert(0, syn::parse_quote! { 'morphix });
    ob_generics.params.push(syn::parse_quote! { __S: ?Sized });
    ob_generics.params.push(syn::parse_quote! { __N });

    let mut ob_where_predicates = match &input.generics.where_clause {
        Some(where_clause) => where_clause.predicates.clone(),
        None => Default::default(),
    };
    let mut ob_where_predicates_with_lifetime_bounds = ob_where_predicates.clone();
    let mut input_impl_observe_observer_where_predicates = Punctuated::<syn::WherePredicate, syn::Token![,]>::new();
    for param in &input.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let ident = &type_param.ident;
            ob_where_predicates.push(syn::parse_quote! { #ident: ::morphix::Observe });
            ob_where_predicates_with_lifetime_bounds.push(syn::parse_quote! { #ident: ::morphix::Observe + 'morphix });
            input_impl_observe_observer_where_predicates.push(syn::parse_quote! { #ident: 'morphix });
        }
    }
    input_impl_observe_observer_where_predicates.push(syn::parse_quote! { Self: 'morphix });
    input_impl_observe_observer_where_predicates.push(syn::parse_quote! { __N: ::morphix::helper::Unsigned });
    input_impl_observe_observer_where_predicates.push(syn::parse_quote! {
        __S: ::morphix::helper::AsDerefMut<__N, Target = Self> + ?Sized + 'morphix
    });

    let mut input_impl_observe_where_predicates = ob_where_predicates.clone();
    input_impl_observe_where_predicates.push(syn::parse_quote! { Self: ::serde::Serialize });
    let (input_impl_generics, type_generics, _) = input.generics.split_for_impl();
    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
    let mut ob_impl_default_where_predicates = ob_where_predicates.clone();
    let mut ob_impl_observer_where_predicates = ob_where_predicates.clone();
    ob_impl_observer_where_predicates.push(syn::parse_quote! { #input_ident #type_generics: ::serde::Serialize });
    ob_impl_observer_where_predicates.push(syn::parse_quote! { __N: ::morphix::helper::Unsigned });
    ob_impl_observer_where_predicates.push(syn::parse_quote! {
        __S: ::morphix::helper::AsDerefMut<__N, Target = #input_ident #type_generics> + 'morphix
    });
    let mut ob_impl_serialize_observer_where_predicates = ob_impl_observer_where_predicates.clone();

    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            for field in named {
                let field_meta = ObserveMeta::parse_attrs(&field.attrs, &mut errors);
                let field_ident = field.ident.as_ref().unwrap();
                let mut field_cloned = field.clone();
                field_cloned.attrs = vec![];
                let field_span = field_cloned.span();
                let field_ty = &field.ty;
                let ob_field_ty = match &field_meta.ob_with_spec {
                    None => {
                        inst_fields.push(quote_spanned! { field_span =>
                            #field_ident: ::morphix::observe::ObserveExt::observe(&mut __value.#field_ident),
                        });
                        quote_spanned! { field_span =>
                            ::morphix::helper::DefaultObserver<'morphix, #field_ty>
                        }
                    }
                    Some((ob_ident, _, _)) => {
                        inst_fields.push(quote_spanned! { field_span =>
                            #field_ident: ::morphix::observe::#ob_ident::<'morphix, #field_ty>::observe(
                                &mut __value.#field_ident
                            ),
                        });
                        quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<'morphix, #field_ty>
                        }
                    }
                };
                type_fields.push(quote_spanned! { field_span =>
                    pub #field_ident: #ob_field_ty,
                });
                ob_impl_default_where_predicates.push(syn::parse_quote_spanned! { field_span =>
                    #ob_field_ty: Default
                });
                default_fields.push(quote_spanned! { field_span =>
                    #field_ident: Default::default(),
                });
                collect_stmts.push(quote_spanned! { field_span =>
                    if let Some(mut mutation) = ::morphix::observe::SerializeObserver::collect::<A>(&mut this.#field_ident)? {
                        mutation.path.push(stringify!(#field_ident).into());
                        mutations.push(mutation);
                    }
                });
                ob_impl_serialize_observer_where_predicates.push(syn::parse_quote! {
                    #ob_field_ty: ::morphix::observe::SerializeObserver<'morphix>
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

    quote! {
        const _: () = {
            #[allow(private_interfaces)]
            #input_vis struct #ob_ident #ob_impl_generics where #ob_where_predicates_with_lifetime_bounds {
                __ptr: ::morphix::helper::Pointer<__S>,
                __mutated: bool,
                __phantom: ::std::marker::PhantomData<&'morphix mut __N>,
                #(#type_fields)*
            }

            #[automatically_derived]
            impl #input_impl_generics ::morphix::Observe
            for #input_ident #type_generics
            where #input_impl_observe_where_predicates {
                type Observer<'morphix, __S, __N> = #ob_ident #ob_type_generics
                where #input_impl_observe_observer_where_predicates;
                type Spec = ::morphix::observe::DefaultSpec;
            }

            #[automatically_derived]
            impl #ob_impl_generics Default
            for #ob_ident #ob_type_generics
            where #ob_impl_default_where_predicates {
                fn default() -> Self {
                    Self {
                        __ptr: ::morphix::helper::Pointer::default(),
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
                type Target = ::morphix::helper::Pointer<__S>;
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
            impl #ob_impl_generics ::morphix::observe::Observer<'morphix>
            for #ob_ident #ob_type_generics
            where #ob_impl_observer_where_predicates {
                type Head = __S;
                type UpperDepth = __N;
                type LowerDepth = ::morphix::helper::Zero;

                fn observe(value: &'morphix mut __S) -> Self {
                    let __ptr = ::morphix::helper::Pointer::new(value);
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
            impl #ob_impl_generics ::morphix::observe::SerializeObserver<'morphix>
            for #ob_ident #ob_type_generics
            where #ob_impl_serialize_observer_where_predicates {
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
        };
    }
}
