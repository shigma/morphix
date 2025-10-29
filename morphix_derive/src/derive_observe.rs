use std::borrow::Cow;
use std::collections::HashSet;
use std::mem::take;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
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

    let mut generics_allocator = GenericsAllocator::default();
    generics_allocator.visit_derive_input(&input);
    let head = generics_allocator.allocate_ty(parse_quote!(S));
    let depth = generics_allocator.allocate_ty(parse_quote!(N));
    let inner = generics_allocator.allocate_ty(parse_quote!(O));
    let ob_lt = generics_allocator.allocate_lt(parse_quote!('ob));

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
                    type Observer<#ob_lt, #head, #depth>
                        = ::morphix::observe::#ob_ident<#ob_lt, #head, #depth>
                    where
                        Self: #ob_lt,
                        #depth: ::morphix::helper::Unsigned,
                        #head: ::morphix::helper::AsDerefMut<#depth, Target = Self> + ?Sized + #ob_lt;

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
    let mut ob_extra_predicates = WherePredicates::default();
    let mut ob_struct_extra_predicates = WherePredicates::default();
    let mut input_observe_observer_predicates = WherePredicates::default();
    let mut ob_default_extra_predicates = WherePredicates::default();
    let mut ob_serialize_observer_extra_predicates = WherePredicates::default();
    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            let mut deref_fields = vec![];
            for field in named {
                let field_meta = ObserveMeta::parse_attrs(&field.attrs, &mut errors);
                let field_ident = field.ident.as_ref().unwrap();
                let mut field_cloned = field.clone();
                field_cloned.attrs = vec![];
                let field_span = field_cloned.span();
                let field_ty = &field.ty;
                let field_trivial = !GenericsDetector::detect(field_ty, &input.generics);
                let ob_field_ty = if let Some(span) = field_meta.deref {
                    let ob_field_ty = match &field_meta.general_impl {
                        None => quote_spanned! { field_span =>
                            DefaultObserver<#ob_lt, #field_ty, #head, Succ<#depth>>
                        },
                        Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<#ob_lt, #head, Succ<#depth>>
                        },
                    };
                    deref_fields.push((field, ob_field_ty, span));
                    inst_fields.push(quote_spanned! { field_span =>
                        #field_ident: __inner,
                    });
                    ob_default_extra_predicates.push(parse_quote_spanned! { field_span =>
                        #inner: Default
                    });
                    quote! { #inner }
                } else {
                    inst_fields.push(quote_spanned! { field_span =>
                        #field_ident: Observer::observe(&mut __value.#field_ident),
                    });
                    let ob_field_ty = match &field_meta.general_impl {
                        None => quote_spanned! { field_span =>
                            DefaultObserver<#ob_lt, #field_ty>
                        },
                        Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                            ::morphix::observe::#ob_ident<#ob_lt, #field_ty>
                        },
                    };
                    if !field_trivial {
                        ob_extra_predicates.push(parse_quote_spanned! { field_span =>
                            #field_ty: Observe
                        });
                        ob_struct_extra_predicates.push(parse_quote_spanned! { field_span =>
                            #field_ty: Observe + #ob_lt
                        });
                        input_observe_observer_predicates.push(parse_quote_spanned! { field_span =>
                            #field_ty: #ob_lt
                        });
                        ob_serialize_observer_extra_predicates.push(parse_quote_spanned! { field_span =>
                            #ob_field_ty: SerializeObserver<#ob_lt>
                        });
                    }
                    ob_field_ty
                };
                type_fields.push(quote_spanned! { field_span =>
                    pub #field_ident: #ob_field_ty,
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
            }
            if !errors.is_empty() {
                return errors.into_iter().map(|error| error.to_compile_error()).collect();
            }

            let input_predicates = match take(&mut input.generics.where_clause) {
                Some(where_clause) => where_clause.predicates,
                None => Default::default(),
            };
            let (input_impl_generics, type_generics, _) = input.generics.split_for_impl();

            let mut ob_predicates = input_predicates.clone();
            ob_predicates.extend(ob_extra_predicates);

            let mut ob_struct_predicates = input_predicates.clone();
            ob_struct_predicates.extend(ob_struct_extra_predicates);

            let mut ob_default_predicates = ob_predicates.clone();
            ob_default_predicates.extend(ob_default_extra_predicates);

            let mut ob_assignable_generics = input.generics.clone();
            let mut ob_assignable_predicates = ob_predicates.clone();
            ob_assignable_generics.params.insert(0, parse_quote! { #ob_lt });
            let mut ob_generics = ob_assignable_generics.clone();
            let mut ob_observer_extra_generics = Punctuated::<syn::GenericParam, syn::Token![,]>::default();

            let deref_impl;
            let deref_mut_impl;
            let assignable_impl;
            let observer_impl;
            let serialize_observer_impl_prefix;
            let input_observer_type_generics;

            let mut ob_observer_extra_predicates = WherePredicates::default();
            if deref_fields.is_empty() {
                ob_generics.params.push(parse_quote! { #head: ?Sized });
                ob_generics.params.push(parse_quote! { #depth = Zero });
                ob_assignable_generics.params.push(parse_quote! { #head });

                type_fields.insert(
                    0,
                    quote! {
                        __ptr: ObserverPointer<#head>,
                        __mutated: bool,
                        __phantom: ::std::marker::PhantomData<&#ob_lt mut #depth>,
                    },
                );

                default_fields.insert(
                    0,
                    quote! {
                        __ptr: ObserverPointer::default(),
                        __mutated: false,
                        __phantom: ::std::marker::PhantomData,
                    },
                );

                ob_observer_extra_predicates.push(parse_quote! {
                    #head: AsDerefMut<#depth, Target = #input_ident #type_generics> + #ob_lt
                });
                ob_observer_extra_predicates.push(parse_quote! { #depth: Unsigned });
                ob_serialize_observer_extra_predicates.push(parse_quote! {
                    #head: AsDerefMut<#depth, Target = #input_ident #type_generics> + #ob_lt
                });
                ob_serialize_observer_extra_predicates.push(parse_quote! { #depth: Unsigned });

                deref_impl = quote! {
                    type Target = ObserverPointer<#head>;
                    fn deref(&self) -> &Self::Target {
                        &self.__ptr
                    }
                };

                deref_mut_impl = quote! {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.__ptr
                    }
                };

                assignable_impl = quote! {
                    type Depth = Succ<Zero>;
                };

                observer_impl = quote! {
                    type Head = #head;
                    type InnerDepth = #depth;
                    type OuterDepth = Zero;

                    fn observe(value: &#ob_lt mut #head) -> Self {
                        let __ptr = ObserverPointer::new(value);
                        let __value = value.as_deref_mut();
                        Self {
                            __ptr,
                            __mutated: false,
                            __phantom: ::std::marker::PhantomData,
                            #(#inst_fields)*
                        }
                    }
                };

                serialize_observer_impl_prefix = quote! {
                    if this.__mutated {
                        return Ok(Some(::morphix::Mutation {
                            path: ::morphix::Path::new(),
                            kind: ::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?),
                        }));
                    };
                };

                let (_, ob_type_generics, _) = ob_generics.split_for_impl();
                input_observer_type_generics = quote! { #ob_type_generics };
            } else {
                let (field, ob_field_ty, _) = deref_fields.swap_remove(0);
                let field_ident = &field.ident;

                ob_generics.params.push(parse_quote! { #inner });
                ob_assignable_generics.params.push(parse_quote! { #inner });
                // FIXME: spanned
                ob_assignable_predicates.push(parse_quote! {
                    #inner: Observer<#ob_lt>
                });
                ob_observer_extra_generics.push(parse_quote! { #depth });

                type_fields.insert(
                    0,
                    quote! {
                        __phantom: ::std::marker::PhantomData<&#ob_lt mut ()>,
                    },
                );

                default_fields.insert(
                    0,
                    quote! {
                        __phantom: ::std::marker::PhantomData,
                    },
                );

                ob_observer_extra_predicates.push(parse_quote! {
                    #inner: Observer<#ob_lt, InnerDepth = Succ<#depth>>
                });
                ob_observer_extra_predicates.push(parse_quote! {
                    #inner::Head: AsDerefMut<#depth, Target = #input_ident #type_generics>
                });
                ob_observer_extra_predicates.push(parse_quote! { #depth: Unsigned });

                ob_serialize_observer_extra_predicates.push(parse_quote! {
                    #inner: SerializeObserver<#ob_lt, InnerDepth = Succ<#depth>>
                });
                ob_serialize_observer_extra_predicates.push(parse_quote! {
                    #inner::Head: AsDerefMut<#depth, Target = #input_ident #type_generics>
                });
                ob_serialize_observer_extra_predicates.push(parse_quote! { #depth: Unsigned });

                deref_impl = quote! {
                    type Target = #inner;
                    fn deref(&self) -> &Self::Target {
                        &self.#field_ident
                    }
                };

                deref_mut_impl = quote! {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        &mut self.#field_ident
                    }
                };

                assignable_impl = quote! {
                    type Depth = Succ<#inner::OuterDepth>;
                };

                observer_impl = quote! {
                    type Head = #inner::Head;
                    type InnerDepth = #depth;
                    type OuterDepth = Succ<#inner::OuterDepth>;

                    fn observe(value: &#ob_lt mut #inner::Head) -> Self {
                        let __inner = Observer::observe(unsafe { &mut *(value as *mut #inner::Head) });
                        let __value = AsDerefMut::<#depth>::as_deref_mut(value);
                        Self {
                            __phantom: ::std::marker::PhantomData,
                            #(#inst_fields)*
                        }
                    }
                };

                serialize_observer_impl_prefix = quote! {};

                let ob_type_arguments = ob_generics.params.iter().map(|param| match param {
                    syn::GenericParam::Type(ty_param) if ty_param.ident == inner => quote! { #ob_field_ty },
                    _ => quote! { #param },
                });
                input_observer_type_generics = quote! { <#(#ob_type_arguments),*> };
            }

            let mut ob_observer_predicates = ob_predicates.clone();
            ob_observer_predicates.extend(ob_observer_extra_predicates);
            let mut ob_serialize_observer_predicates = ob_predicates.clone();
            ob_serialize_observer_predicates.extend(ob_serialize_observer_extra_predicates);
            let input_trivial = input.generics.params.is_empty();
            if !input_trivial {
                ob_serialize_observer_predicates.insert(
                    0,
                    parse_quote! {
                        #input_ident #type_generics: ::serde::Serialize
                    },
                );
            }

            input_observe_observer_predicates.push(parse_quote! { Self: #ob_lt });
            input_observe_observer_predicates.push(parse_quote! { #depth: Unsigned });
            input_observe_observer_predicates.push(parse_quote! {
                #head: AsDerefMut<#depth, Target = Self> + ?Sized + #ob_lt
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

            quote! {
                const _: () = {
                    // #[allow(unused_imports)]
                    use ::morphix::helper::{AsDerefMut, Succ, Unsigned, Zero};
                    // #[allow(unused_imports)]
                    use ::morphix::observe::{DefaultObserver, Observe, Observer, ObserverPointer, SerializeObserver};

                    #[allow(private_interfaces)]
                    #input_vis struct #ob_ident #ob_generics
                    where #ob_struct_predicates {
                        #(#type_fields)*
                    }

                    #[automatically_derived]
                    impl #ob_impl_generics Default
                    for #ob_ident #ob_type_generics
                    where #ob_default_predicates {
                        fn default() -> Self {
                            Self {
                                #(#default_fields)*
                            }
                        }
                    }

                    #[automatically_derived]
                    impl #ob_impl_generics ::std::ops::Deref
                    for #ob_ident #ob_type_generics
                    where #ob_predicates {
                        #deref_impl
                    }

                    #[automatically_derived]
                    impl #ob_impl_generics ::std::ops::DerefMut
                    for #ob_ident #ob_type_generics
                    where #ob_predicates {
                        #deref_mut_impl
                    }

                    #[automatically_derived]
                    impl #ob_assignable_impl_generics ::morphix::helper::Assignable
                    for #ob_ident #ob_assignable_type_generics
                    where #ob_assignable_predicates {
                        #assignable_impl
                    }

                    #[automatically_derived]
                    impl #ob_observer_impl_generics Observer<#ob_lt>
                    for #ob_ident #ob_type_generics
                    where #ob_observer_predicates {
                        #observer_impl
                    }

                    #[automatically_derived]
                    impl #ob_observer_impl_generics SerializeObserver<#ob_lt>
                    for #ob_ident #ob_type_generics
                    where #ob_serialize_observer_predicates {
                        unsafe fn collect_unchecked<A: ::morphix::Adapter>(
                            this: &mut Self,
                        ) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A>>, A::Error> {
                            #serialize_observer_impl_prefix
                            let mut mutations = ::std::vec::Vec::new();
                            #(#collect_stmts)*
                            Ok(::morphix::Mutation::coalesce(mutations))
                        }
                    }

                    #[automatically_derived]
                    impl #input_impl_generics Observe
                    for #input_ident #type_generics
                    where #input_observe_predicates {
                        type Observer<#ob_lt, #head, #depth> = #ob_ident #input_observer_type_generics
                        where #input_observe_observer_predicates;
                        type Spec = ::morphix::observe::DefaultSpec;
                    }
                };
            }
        }
        _ => syn::Error::new(input.span(), "Observe can only be derived for named structs").to_compile_error(),
    }
}

#[derive(Default)]
struct GenericsAllocator<'i> {
    ty_idents: HashSet<Cow<'i, syn::Ident>>,
    lt_idents: HashSet<Cow<'i, syn::Ident>>,
}

impl<'i> GenericsAllocator<'i> {
    fn allocate_ty(&mut self, ident: syn::Ident) -> syn::Ident {
        let mut ident: Cow<'i, syn::Ident> = Cow::Owned(ident);
        while !self.ty_idents.insert(ident.clone()) {
            let new_ident = format_ident!("_{}", ident);
            ident = Cow::Owned(new_ident);
        }
        ident.into_owned()
    }

    fn allocate_lt(&mut self, mut lifetime: syn::Lifetime) -> syn::Lifetime {
        let mut ident: Cow<'i, syn::Ident> = Cow::Owned(lifetime.ident);
        while !self.lt_idents.insert(ident.clone()) {
            let new_ident = format_ident!("_{}", ident);
            ident = Cow::Owned(new_ident);
        }
        lifetime.ident = ident.into_owned();
        lifetime
    }
}

impl<'i, 'ast: 'i> Visit<'ast> for GenericsAllocator<'i> {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(ident) = path.get_ident() {
            self.ty_idents.insert(Cow::Borrowed(ident));
        }
    }

    fn visit_lifetime_param(&mut self, lt_param: &'ast syn::LifetimeParam) {
        self.lt_idents.insert(Cow::Borrowed(&lt_param.lifetime.ident));
    }
}

struct GenericsDetector<'i> {
    is_detected: bool,
    params: &'i Punctuated<syn::GenericParam, syn::Token![,]>,
}

impl<'i> GenericsDetector<'i> {
    fn detect(ty: &syn::Type, generics: &'i syn::Generics) -> bool {
        let mut checker = GenericsDetector {
            is_detected: false,
            params: &generics.params,
        };
        syn::visit::visit_type(&mut checker, ty);
        checker.is_detected
    }
}

impl<'i> Visit<'_> for GenericsDetector<'i> {
    fn visit_type_path(&mut self, type_path: &syn::TypePath) {
        if type_path.qself.is_none()
            && let Some(ident) = type_path.path.get_ident()
        {
            for param in self.params {
                match param {
                    syn::GenericParam::Type(ty_param) => {
                        if &ty_param.ident == ident {
                            self.is_detected = true;
                        }
                    }
                    syn::GenericParam::Lifetime(_lt_param) => {}
                    syn::GenericParam::Const(const_param) => {
                        if &const_param.ident == ident {
                            self.is_detected = true;
                        }
                    }
                }
            }
        }
        syn::visit::visit_type_path(self, type_path);
    }

    fn visit_lifetime(&mut self, lifetime: &syn::Lifetime) {
        for param in self.params {
            if let syn::GenericParam::Lifetime(lt_param) = param
                && &lt_param.lifetime == lifetime
            {
                self.is_detected = true;
            }
        }
    }
}
