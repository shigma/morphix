use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parse;
use syn::parse_quote;
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
    observer: Option<(syn::Ident, syn::Type)>,
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
                    meta.observer = Some((
                        syn::Ident::new("HashObserver", ident.span()),
                        parse_quote! { ::morphix::observe::HashSpec },
                    ));
                } else if ident == "noop" {
                    meta.observer = Some((
                        syn::Ident::new("NoopObserver", ident.span()),
                        parse_quote! { ::morphix::observe::DefaultSpec },
                    ));
                } else if ident == "shallow" {
                    meta.observer = Some((
                        syn::Ident::new("ShallowObserver", ident.span()),
                        parse_quote! { ::morphix::observe::DefaultSpec },
                    ));
                } else if ident == "snapshot" {
                    meta.observer = Some((
                        syn::Ident::new("SnapshotObserver", ident.span()),
                        parse_quote! { ::morphix::observe::SnapshotSpec },
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

pub fn derive_observe(mut input: syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let input_ident = &input.ident;
    let mut errors = vec![];
    let input_meta = ObserveMeta::parse_attrs(&input.attrs, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }
    if let Some((ob_ident, ob_spec)) = input_meta.observer {
        let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
        return Ok(quote! {
            const _: () = {
                #[automatically_derived]
                impl #impl_generics ::morphix::Observe for #input_ident #type_generics #where_clause {
                    type Observer<'morphix>
                        = ::morphix::observe::#ob_ident<'morphix, Self>
                    where
                        Self: 'morphix;

                    type Spec = #ob_spec;
                }
            };
        });
    }

    let ob_ident = format_ident!("{}Observer", input_ident);
    let input_vis = &input.vis;
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    let mut default_fields = vec![];
    let mut collect_stmts = vec![];

    for param in &mut input.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            type_param.bounds.push(syn::parse_quote! { ::morphix::Observe });
        }
    }
    let mut ob_generics = input.generics.clone();
    ob_generics.params.insert(0, syn::parse_quote! { 'morphix });
    let (input_impl_generics, type_generics, input_where_clause) = input.generics.split_for_impl();
    let (ob_impl_generics, ob_type_generics, ob_where_clause) = ob_generics.split_for_impl();
    let mut ob_default_where_predicates = match ob_where_clause {
        Some(where_clause) => where_clause.predicates.clone(),
        None => Default::default(),
    };

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
                let ob_field_ty = match &field_meta.observer {
                    None => {
                        inst_fields.push(quote_spanned! { field_span =>
                            #field_ident: ::morphix::Observe::__observe(&mut value.#field_ident),
                        });
                        quote_spanned! { field_span =>
                            <#field_ty as ::morphix::Observe>::Observer<'morphix>
                        }
                    }
                    Some((ob_ident, _)) => {
                        inst_fields.push(quote_spanned! { field_span =>
                            #field_ident: ::morphix::observe::#ob_ident::observe(&mut value.#field_ident),
                        });
                        quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<'morphix, #field_ty>
                        }
                    }
                };
                type_fields.push(quote_spanned! { field_span =>
                    pub #field_ident: #ob_field_ty,
                });
                ob_default_where_predicates.push(syn::parse_quote_spanned! { field_span =>
                    #ob_field_ty: Default
                });
                default_fields.push(quote_spanned! { field_span =>
                    #field_ident: Default::default(),
                });
                collect_stmts.push(quote_spanned! { field_span =>
                    if let Some(mut mutation) = ::morphix::Observer::collect::<A>(this.#field_ident)? {
                        mutation.path_rev.push(stringify!(#field_ident).into());
                        mutations.push(mutation);
                    }
                });
            }
        }
        _ => {
            return Err(vec![syn::Error::new(
                input.span(),
                "Observe can only be derived for named structs",
            )]);
        }
    };
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut lifetime_where_predicates = vec![];
    for param in &input.generics.params {
        if let syn::GenericParam::Type(type_param) = param {
            let ident = &type_param.ident;
            lifetime_where_predicates.push(quote! { #ident: 'morphix });
        }
    }

    Ok(quote! {
        const _: () = {
            #[allow(private_interfaces)]
            #input_vis struct #ob_ident #ob_impl_generics #ob_where_clause {
                __ptr: *mut #input_ident #type_generics,
                __mutated: bool,
                __phantom: ::std::marker::PhantomData<&'morphix mut #input_ident #type_generics>,
                #(#type_fields)*
            }

            #[automatically_derived]
            impl #input_impl_generics Observe for #input_ident #type_generics #input_where_clause {
                type Observer<'morphix> = #ob_ident #ob_type_generics where #(#lifetime_where_predicates,)*;

                type Spec = ::morphix::observe::DefaultSpec;
            }

            #[automatically_derived]
            impl #ob_impl_generics Default for #ob_ident #ob_type_generics where #ob_default_where_predicates {
                fn default() -> Self {
                    Self {
                        __ptr: ::std::ptr::null_mut(),
                        __mutated: false,
                        __phantom: ::std::marker::PhantomData,
                        #(#default_fields)*
                    }
                }
            }

            #[automatically_derived]
            impl #ob_impl_generics ::std::ops::Deref for #ob_ident #ob_type_generics #ob_where_clause {
                type Target = #input_ident #type_generics;
                fn deref(&self) -> &Self::Target {
                    unsafe { &*self.__ptr }
                }
            }

            #[automatically_derived]
            impl #ob_impl_generics ::std::ops::DerefMut for #ob_ident #ob_type_generics #ob_where_clause {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.__mutated = true;
                    unsafe { &mut *self.__ptr }
                }
            }

            #[automatically_derived]
            impl #ob_impl_generics ::morphix::Observer<'morphix> for #ob_ident #ob_type_generics #ob_where_clause {
                fn inner(this: &Self) -> *mut #input_ident #type_generics {
                    this.__ptr
                }

                fn observe(value: &'morphix mut #input_ident #type_generics) -> Self {
                    Self {
                        __ptr: value as *mut #input_ident #type_generics,
                        __mutated: false,
                        __phantom: ::std::marker::PhantomData,
                        #(#inst_fields)*
                    }
                }

                unsafe fn collect_unchecked<A: ::morphix::Adapter>(
                    this: Self,
                ) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A>>, A::Error> {
                    if this.__mutated {
                        return Ok(Some(::morphix::Mutation {
                            path_rev: ::std::vec::Vec::new(),
                            operation: ::morphix::MutationKind::Replace(A::serialize_value(&*this)?),
                        }));
                    };
                    let mut mutations = ::std::vec::Vec::new();
                    #(#collect_stmts)*
                    Ok(::morphix::Batch::build(mutations))
                }
            }
        };
    })
}
