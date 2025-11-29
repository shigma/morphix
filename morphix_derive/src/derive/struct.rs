use std::mem::take;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{parse_quote, parse_quote_spanned};

use crate::derive::meta::{AttributeKind, DeriveKind, GeneralImpl, ObserveMeta};
use crate::derive::{GenericsAllocator, GenericsDetector};

type WherePredicates = Punctuated<syn::WherePredicate, syn::Token![,]>;

pub fn derive_observe_for_struct_fields(
    input: &syn::DeriveInput,
    fields: &Punctuated<syn::Field, syn::Token![,]>,
    input_meta: &ObserveMeta,
) -> TokenStream {
    let input_ident = &input.ident;
    let ob_name = format!("{}Observer", input_ident);
    let ob_ident = format_ident!("{}Observer", input_ident);
    let input_vis = &input.vis;

    let mut errors = quote! {};
    let mut generics_allocator = GenericsAllocator::default();
    generics_allocator.visit_derive_input(input);
    let head = generics_allocator.allocate_ty(parse_quote!(S));
    let depth = generics_allocator.allocate_ty(parse_quote!(N));
    let inner = generics_allocator.allocate_ty(parse_quote!(O));
    let ob_lt = generics_allocator.allocate_lt(parse_quote!('ob));

    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    let mut default_fields = vec![];
    let mut refresh_stmts = vec![];
    let mut collect_stmts = vec![];
    let mut debug_chain = quote! {};
    let mut ob_extra_predicates = WherePredicates::default();
    let mut ob_struct_extra_predicates = WherePredicates::default();
    let mut input_observe_observer_predicates = WherePredicates::default();
    let mut ob_default_extra_predicates = WherePredicates::default();
    let mut ob_debug_extra_predicates = WherePredicates::default();
    let mut ob_serialize_observer_extra_predicates = WherePredicates::default();

    let mut deref_fields = vec![];
    let field_count = fields.len();
    for field in fields {
        let field_meta = ObserveMeta::parse_attrs(&field.attrs, &mut errors, AttributeKind::Field, DeriveKind::Struct);
        let field_ident = field.ident.as_ref().unwrap();
        let field_name = field_ident.to_string();
        let mut field_cloned = field.clone();
        field_cloned.attrs = vec![];
        let field_span = field_cloned.span();
        let field_ty = &field.ty;
        let ob_field_ty = if let Some(span) = field_meta.deref {
            let ob_field_ty = match &field_meta.general_impl {
                None => quote_spanned! { field_span =>
                    ::morphix::observe::DefaultObserver<#ob_lt, #field_ty, #head, ::morphix::helper::Succ<#depth>>
                },
                Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                    ::morphix::observe::#ob_ident<#ob_lt, #head, ::morphix::helper::Succ<#depth>>
                },
            };
            deref_fields.push((field, ob_field_ty, span));
            inst_fields.push(quote_spanned! { field_span =>
                #field_ident: __inner,
            });
            ob_default_extra_predicates.push(parse_quote_spanned! { field_span =>
                #inner: ::std::default::Default
            });
            ob_debug_extra_predicates.push(parse_quote_spanned! { field_span =>
                #inner: ::std::fmt::Debug
            });
            quote! { #inner }
        } else {
            let field_trivial = !GenericsDetector::detect(field_ty, &input.generics);
            inst_fields.push(quote_spanned! { field_span =>
                #field_ident: ::morphix::observe::Observer::observe(&mut __value.#field_ident),
            });
            let ob_field_ty = match &field_meta.general_impl {
                None => quote_spanned! { field_span =>
                    ::morphix::observe::DefaultObserver<#ob_lt, #field_ty>
                },
                Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                    ::morphix::observe::#ob_ident<#ob_lt, #field_ty>
                },
            };
            if !field_trivial {
                ob_extra_predicates.push(parse_quote_spanned! { field_span =>
                    #field_ty: ::morphix::Observe
                });
                ob_struct_extra_predicates.push(parse_quote_spanned! { field_span =>
                    #field_ty: ::morphix::Observe + #ob_lt
                });
                input_observe_observer_predicates.push(parse_quote_spanned! { field_span =>
                    #field_ty: #ob_lt
                });
                ob_serialize_observer_extra_predicates.push(parse_quote_spanned! { field_span =>
                    #ob_field_ty: ::morphix::observe::SerializeObserver<#ob_lt>
                });
                ob_debug_extra_predicates.push(parse_quote_spanned! { field_span =>
                    #ob_field_ty: ::std::fmt::Debug
                });
            }
            refresh_stmts.push(quote_spanned! { field_span =>
                ::morphix::observe::Observer::refresh(&mut this.#field_ident, &mut __value.#field_ident);
            });
            ob_field_ty
        };
        type_fields.push(quote_spanned! { field_span =>
            pub #field_ident: #ob_field_ty,
        });
        default_fields.push(quote_spanned! { field_span =>
            #field_ident: ::std::default::Default::default(),
        });
        debug_chain.extend(quote_spanned! { field_span =>
            .field(#field_name, &self.#field_ident)
        });
        let mut mutability = quote! {};
        let mut body = if field_count == 1 {
            quote! { return Ok(Some(mutation)); }
        } else {
            quote! { mutations.push(mutation); }
        };
        if !field_meta.serde.flatten {
            mutability = quote! { mut };
            body = quote! {
                mutation.path.push(#field_name.into());
                #body
            };
        }
        collect_stmts.push(quote_spanned! { field_span =>
            if let Some(#mutability mutation) = ::morphix::observe::SerializeObserver::collect::<A>(&mut this.#field_ident)? {
                #body
            }
        });
    }
    if !errors.is_empty() {
        return errors;
    }

    let mut input_generics = input.generics.clone();
    let input_predicates = match take(&mut input_generics.where_clause) {
        Some(where_clause) => where_clause.predicates,
        None => Default::default(),
    };
    let (input_impl_generics, type_generics, _) = input_generics.split_for_impl();

    let mut ob_predicates = input_predicates.clone();
    ob_predicates.extend(ob_extra_predicates);

    let mut ob_struct_predicates = input_predicates.clone();
    ob_struct_predicates.extend(ob_struct_extra_predicates);

    let mut ob_default_predicates = ob_predicates.clone();
    ob_default_predicates.extend(ob_default_extra_predicates);

    let mut ob_assignable_generics = input_generics.clone();
    let mut ob_assignable_predicates = ob_predicates.clone();
    ob_assignable_generics.params.insert(0, parse_quote! { #ob_lt });
    let mut ob_generics = ob_assignable_generics.clone();
    let mut ob_observer_extra_generics = Punctuated::<syn::GenericParam, syn::Token![,]>::default();

    let deref_ident;
    let deref_impl;
    let deref_mut_impl;
    let assignable_impl;
    let observer_impl;
    let serialize_observer_impl_prefix;
    let input_observer_type_generics;

    let mut ob_observer_extra_predicates = WherePredicates::default();
    if deref_fields.is_empty() {
        ob_generics.params.push(parse_quote! { #head: ?Sized });
        ob_generics
            .params
            .push(parse_quote! { #depth = ::morphix::helper::Zero });
        ob_assignable_generics.params.push(parse_quote! { #head });

        type_fields.insert(
            0,
            quote! {
                __ptr: ::morphix::observe::ObserverPointer<#head>,
                __mutated: bool,
                __phantom: ::std::marker::PhantomData<&#ob_lt mut #depth>,
            },
        );

        default_fields.insert(
            0,
            quote! {
                __ptr: ::std::default::Default::default(),
                __mutated: false,
                __phantom: ::std::marker::PhantomData,
            },
        );

        ob_observer_extra_predicates.push(parse_quote! {
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #type_generics> + #ob_lt
        });
        ob_observer_extra_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });
        ob_serialize_observer_extra_predicates.push(parse_quote! {
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #type_generics> + #ob_lt
        });
        ob_serialize_observer_extra_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });

        deref_ident = format_ident!("Deref");
        deref_impl = quote! {
            type Target = ::morphix::observe::ObserverPointer<#head>;
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
            type Depth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        };

        observer_impl = quote! {
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
                    #(#inst_fields)*
                }
            }

            unsafe fn refresh(this: &mut Self, value: &mut #head) {
                ::morphix::observe::ObserverPointer::set(&this.__ptr, value);
                let __value = value.as_deref_mut();
                unsafe {
                    #(#refresh_stmts)*
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
        if deref_fields.len() > 1 {
            return deref_fields
                .into_iter()
                .map(|(_, _, ident)| {
                    syn::Error::new(ident.span(), "only one field can be marked as `deref`").to_compile_error()
                })
                .collect();
        }

        let (field, ob_field_ty, meta_ident) = deref_fields.swap_remove(0);
        let field_ident = &field.ident;

        ob_generics.params.push(parse_quote! { #inner });
        ob_assignable_generics.params.push(parse_quote! { #inner });
        ob_assignable_predicates.push(parse_quote! {
            #inner: ::morphix::observe::Observer<#ob_lt>
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
            #inner: ::morphix::observe::Observer<#ob_lt, InnerDepth = ::morphix::helper::Succ<#depth>>
        });
        ob_observer_extra_predicates.push(parse_quote! {
            #inner::Head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #type_generics>
        });
        ob_observer_extra_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });

        ob_serialize_observer_extra_predicates.push(parse_quote! {
            #inner: ::morphix::observe::SerializeObserver<#ob_lt, InnerDepth = ::morphix::helper::Succ<#depth>>
        });
        ob_serialize_observer_extra_predicates.push(parse_quote! {
            #inner::Head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #type_generics>
        });
        ob_serialize_observer_extra_predicates.push(parse_quote! { #depth: ::morphix::helper::Unsigned });

        deref_ident = syn::Ident::new("Deref", meta_ident.span());
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
            type Depth = ::morphix::helper::Succ<#inner::OuterDepth>;
        };

        observer_impl = quote! {
            type Head = #inner::Head;
            type InnerDepth = #depth;
            type OuterDepth = ::morphix::helper::Succ<#inner::OuterDepth>;

            fn observe(value: &#ob_lt mut #inner::Head) -> Self {
                let __inner = ::morphix::observe::Observer::observe(unsafe { &mut *(value as *mut #inner::Head) });
                let __value = ::morphix::helper::AsDerefMut::<#depth>::as_deref_mut(value);
                Self {
                    __phantom: ::std::marker::PhantomData,
                    #(#inst_fields)*
                }
            }

            unsafe fn refresh(this: &mut Self, value: &mut #inner::Head) {
                unsafe {
                    ::morphix::observe::Observer::refresh(&mut this.#field_ident, value);
                    let __value = ::morphix::helper::AsDerefMut::<#depth>::as_deref_mut(value);
                    #(#refresh_stmts)*
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
                #input_ident #type_generics: ::serde::Serialize
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

    let serialize_observer_impl = if field_count == 1 {
        quote! {
            #(#collect_stmts)*
            Ok(None)
        }
    } else {
        quote! {
            let mut mutations = ::std::vec::Vec::with_capacity(#field_count);
            #(#collect_stmts)*
            Ok(::morphix::Mutation::coalesce(mutations))
        }
    };

    let mut output = quote! {
        #input_vis struct #ob_ident #ob_generics
        where #ob_struct_predicates {
            #(#type_fields)*
        }

        #[automatically_derived]
        impl #ob_impl_generics ::std::default::Default
        for #ob_ident #ob_type_generics
        where #ob_default_predicates {
            fn default() -> Self {
                Self {
                    #(#default_fields)*
                }
            }
        }

        #[automatically_derived]
        impl #ob_impl_generics ::std::ops::#deref_ident
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
        impl #ob_observer_impl_generics ::morphix::observe::Observer<#ob_lt>
        for #ob_ident #ob_type_generics
        where #ob_observer_predicates {
            #observer_impl
        }

        #[automatically_derived]
        impl #ob_observer_impl_generics ::morphix::observe::SerializeObserver<#ob_lt>
        for #ob_ident #ob_type_generics
        where #ob_serialize_observer_predicates {
            unsafe fn collect_unchecked<A: ::morphix::Adapter>(
                this: &mut Self,
            ) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A::Value>>, A::Error> {
                #serialize_observer_impl_prefix
                #serialize_observer_impl
            }
        }

        #[automatically_derived]
        impl #input_impl_generics ::morphix::Observe
        for #input_ident #type_generics
        where #input_observe_predicates {
            type Observer<#ob_lt, #head, #depth> = #ob_ident #input_observer_type_generics
            where #input_observe_observer_predicates;
            type Spec = ::morphix::observe::DefaultSpec;
        }
    };

    for derive_ident in &input_meta.derive {
        if derive_ident.is_ident("Debug") {
            output.extend(quote! {
                #[automatically_derived]
                impl #ob_impl_generics ::std::fmt::Debug
                for #ob_ident #ob_type_generics
                where #ob_debug_predicates {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        f.debug_struct(#ob_name) #debug_chain .finish()
                    }
                }
            });
        } else if derive_ident.is_ident("Display") {
            output.extend(quote! {
                #[automatically_derived]
                impl #ob_impl_generics ::std::fmt::Display
                for #ob_ident #ob_type_generics
                where #ob_observer_predicates
                {
                    #[inline]
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        ::std::fmt::Display::fmt(self.as_deref(), f)
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
