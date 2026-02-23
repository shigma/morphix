use std::mem::take;

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::derive::meta::{AttributeKind, DeriveKind, GeneralImpl, ObserveMeta};
use crate::derive::{FMT_TRAITS, GenericsDetector, GenericsVisitor};

pub fn derive_observe_for_struct(
    input: &syn::DeriveInput,
    fields: &Punctuated<syn::Field, syn::Token![,]>,
    input_meta: &ObserveMeta,
    is_named: bool,
) -> TokenStream {
    let input_ident = &input.ident;
    let ob_name = format!("{}Observer", input_ident);
    let ob_ident = format_ident!("{}Observer", input_ident);
    let input_vis = &input.vis;

    let mut errors = quote! {};
    let mut generics_visitor = GenericsVisitor::default();
    generics_visitor.visit_derive_input(input);
    let head = generics_visitor.allocate_ty(parse_quote!(S));
    let depth = generics_visitor.allocate_ty(parse_quote!(N));
    let inner = generics_visitor.allocate_ty(parse_quote!(O));
    let ob_lt = generics_visitor.allocate_lt(parse_quote!('ob));

    let if_named = match is_named {
        true => vec![quote! {}],
        false => vec![],
    };

    let mut ob_fields = quote! {};
    let mut observe_fields = quote! {};
    let mut uninit_fields = quote! {};
    let mut refresh_stmts = quote! {};
    let mut pre_flush_stmts = quote! {};
    let mut post_flush_stmts = quote! {};
    let mut flush_capacity = vec![];
    let mut debug_chain = quote! {};

    let mut field_tys = vec![];
    let mut skipped_tys = vec![];
    let mut ob_field_tys = vec![];
    let mut deref_fields = vec![];
    let field_count = fields.len();
    for (i, field) in fields.iter().enumerate() {
        let field_meta = ObserveMeta::parse_attrs(&field.attrs, &mut errors, AttributeKind::Field, DeriveKind::Struct);
        let field_vis = &field.vis;
        let field_ident = &field.ident;
        let field_member = match &field.ident {
            Some(ident) => quote! { #ident },
            None => syn::Index::from(i).to_token_stream(),
        };
        let field_span = {
            let mut field_cloned = field.clone();
            field_cloned.attrs = vec![];
            field_cloned.span()
        };
        let field_ty = &field.ty;
        let field_trivial = !GenericsDetector::detect(field_ty, &input.generics);
        if field_meta.skip || field_meta.serde.skip || field_meta.serde.skip_serializing {
            if !field_trivial {
                skipped_tys.push(quote! { #field_ty });
            }
            ob_fields.extend(quote_spanned! { field_span =>
                #field_vis #(#if_named #field_ident:)* ::morphix::helper::Pointer<#field_ty>,
            });
            observe_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* ::morphix::helper::Pointer::new(&__value.#field_member),
            });
            uninit_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* ::morphix::helper::Pointer::uninit(),
            });
            refresh_stmts.extend(quote_spanned! { field_span =>
                ::morphix::helper::Pointer::set(&this.#field_member, &__value.#field_member);
            });
            continue;
        }

        if let Some(deref_ident) = field_meta.deref {
            let ob_field_ty = match &field_meta.general_impl {
                None => quote_spanned! { field_span =>
                    ::morphix::observe::DefaultObserver<#ob_lt, #field_ty, #head, ::morphix::helper::Succ<#depth>>
                },
                Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                    ::morphix::builtin::#ob_ident<#ob_lt, #head, ::morphix::helper::Succ<#depth>>
                },
            };
            if !field_trivial {
                skipped_tys.push(quote! { #field_ty });
            }
            deref_fields.push((i, field, ob_field_ty, deref_ident));
            ob_field_tys.push(quote! { #inner });
            ob_fields.extend(quote_spanned! { field_span =>
                #field_vis #(#if_named #field_ident:)* #inner,
            });
            observe_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* __inner,
            });
        } else {
            let ob_field_ty = match &field_meta.general_impl {
                None => quote_spanned! { field_span =>
                    ::morphix::observe::DefaultObserver<#ob_lt, #field_ty>
                },
                Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                    ::morphix::builtin::#ob_ident<#ob_lt, #field_ty>
                },
            };
            if !field_trivial {
                field_tys.push(quote! { #field_ty });
                ob_field_tys.push(quote! { #ob_field_ty });
            }
            refresh_stmts.extend(quote_spanned! { field_span =>
                ::morphix::observe::Observer::refresh(&mut this.#field_member, &__value.#field_member);
            });
            ob_fields.extend(quote_spanned! { field_span =>
                #field_vis #(#if_named #field_ident:)* #ob_field_ty,
            });
            observe_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* ::morphix::observe::Observer::observe(&__value.#field_member),
            });
        };
        uninit_fields.extend(quote_spanned! { field_span =>
            #(#if_named #field_ident:)* ::morphix::observe::Observer::uninit(),
        });

        let mutable_ident;
        let default_segment;
        if let Some(ident) = &field.ident {
            let mut field_name = ident.to_string();
            if field_name.starts_with("r#") {
                field_name = field_name[2..].to_string();
            }
            debug_chain.extend(quote_spanned! { field_span =>
                .field(#field_name, &self.#field_member)
            });
            mutable_ident = syn::Ident::new(&format!("mutations_{field_name}"), field_span);
            let segment = input_meta.serde.rename_all.apply(&field_name);
            default_segment = quote! { #segment };
        } else {
            debug_chain.extend(quote_spanned! { field_span =>
                .field(&self.#field_member)
            });
            mutable_ident = syn::Ident::new(&format!("mutations_{i}"), field_span);
            default_segment = quote! { #i };
        }

        pre_flush_stmts.extend(if cfg!(feature = "delete")
            && let Some(path) = field_meta.serde.skip_serializing_if
        {
            quote_spanned! { field_span =>
                let mut #mutable_ident = ::morphix::observe::SerializeObserver::flush::<A>(&mut this.#field_member)?;
                if !#mutable_ident.is_empty() && #path(::morphix::observe::Observer::as_inner(&this.#field_member)) {
                    #mutable_ident = ::morphix::MutationKind::Delete.into();
                }
            }
        } else {
            quote_spanned! { field_span =>
                let #mutable_ident = ::morphix::observe::SerializeObserver::flush::<A>(&mut this.#field_member)?;
            }
        });
        flush_capacity.push(quote_spanned! { field_span =>
            #mutable_ident.len()
        });
        if !field_meta.serde.flatten && (is_named || fields.len() > 1) {
            let segment = if let Some(rename) = &field_meta.serde.rename {
                quote! { #rename }
            } else {
                default_segment
            };
            post_flush_stmts.extend(quote_spanned! { field_span =>
                mutations.insert(#segment, #mutable_ident);
            });
        } else {
            post_flush_stmts.extend(quote_spanned! { field_span =>
                mutations.extend(#mutable_ident);
            });
        }
    }
    if !errors.is_empty() {
        return errors;
    }

    let mut input_generics = input.generics.clone();
    let input_predicates = match take(&mut input_generics.where_clause) {
        Some(where_clause) => where_clause.predicates.into_iter().collect::<Vec<_>>(),
        None => Default::default(),
    };
    let (input_impl_generics, input_type_generics, _) = input_generics.split_for_impl();

    let mut ob_generics = input_generics.clone();
    let mut ob_quasi_generics;
    let mut ob_observer_generics = input_generics.clone();

    let deref_ident;
    let deref_target;
    let deref_member;
    let deref_mut_impl;
    let assignable_impl;
    let observer_impl;
    let serialize_observer_impl_prefix;
    let ob_quasi_predicates;
    let ob_observer_predicates;
    let input_observe_predicates;
    let input_observer_type_generics;

    if deref_fields.is_empty() {
        ob_generics.params.insert(0, parse_quote! { #ob_lt });
        ob_generics.params.push(parse_quote! { #head: ?Sized });
        ob_generics
            .params
            .push(parse_quote! { #depth = ::morphix::helper::Zero });
        ob_quasi_generics = ob_generics.clone();
        ob_quasi_predicates = quote! {
            #head: ::morphix::helper::AsDeref<#depth>,
        };
        ob_observer_generics.params.insert(0, parse_quote! { #ob_lt });
        ob_observer_generics.params.push(parse_quote! { #head: ?Sized });
        ob_observer_generics
            .params
            .push(parse_quote! { #depth = ::morphix::helper::Zero });
        ob_observer_predicates = quote! {
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt,
        };

        ob_fields.extend(quote! {
            #(#if_named __ptr:)* ::morphix::helper::Pointer<#head>,
            #(#if_named __mutated:)* bool,
            #(#if_named __phantom:)* ::std::marker::PhantomData<&#ob_lt mut #depth>,
        });

        deref_ident = format_ident!("Deref");
        deref_target = quote! { ::morphix::helper::Pointer<#head> };
        deref_member = match is_named {
            true => quote! { __ptr },
            false => syn::Index::from(fields.len()).to_token_stream(),
        };
        let mutated_field = match is_named {
            true => quote! { __mutated },
            false => syn::Index::from(fields.len() + 1).to_token_stream(),
        };
        deref_mut_impl = quote! {
            self.#mutated_field = true;
        };

        assignable_impl = quote! {
            type OuterDepth = ::morphix::helper::Succ<::morphix::helper::Zero>;
        };

        let observer_uninit_expr = match is_named {
            true => quote! {
                Self {
                    #uninit_fields
                    __ptr: ::morphix::helper::Pointer::uninit(),
                    __mutated: false,
                    __phantom: ::std::marker::PhantomData,
                }
            },
            false => quote! {
                Self (
                    #uninit_fields
                    ::morphix::helper::Pointer::uninit(),
                    false,
                    ::std::marker::PhantomData,
                )
            },
        };

        let observer_observe_expr = match is_named {
            true => quote! {
                Self {
                    #observe_fields
                    __ptr,
                    __mutated: false,
                    __phantom: ::std::marker::PhantomData,
                }
            },
            false => quote! {
                Self (
                    #observe_fields
                    __ptr,
                    false,
                    ::std::marker::PhantomData,
                )
            },
        };

        observer_impl = quote! {
            fn uninit() -> Self {
                #observer_uninit_expr
            }

            fn observe(value: &#head) -> Self {
                let __ptr = ::morphix::helper::Pointer::new(value);
                let __value = value.as_deref();
                #observer_observe_expr
            }

            unsafe fn refresh(this: &mut Self, value: &#head) {
                ::morphix::helper::Pointer::set(this, value);
                let __value = value.as_deref();
                unsafe {
                    #refresh_stmts
                }
            }
        };

        serialize_observer_impl_prefix = quote! {
            if this.#mutated_field {
                this.#mutated_field = false;
                return Ok(::morphix::MutationKind::Replace(A::serialize_value(this.as_deref())?).into());
            };
        };

        input_observe_predicates = quote! {};
        let (_, ob_type_generics, _) = ob_generics.split_for_impl();
        input_observer_type_generics = quote! { #ob_type_generics };
    } else if deref_fields.len() > 1 {
        return deref_fields
            .into_iter()
            .map(|(_, _, _, ident)| {
                syn::Error::new(ident.span(), "only one field can be marked as `deref`").to_compile_error()
            })
            .collect();
    } else {
        let (i, field, ob_field_ty, meta_deref_ident) = deref_fields.swap_remove(0);
        let field_ty = &field.ty;
        let field_member = match &field.ident {
            Some(ident) => quote! { #ident },
            None => syn::Index::from(i).to_token_stream(),
        };

        let mut generics_visitor = GenericsVisitor::default();
        for other_field in fields {
            if field.ident == other_field.ident {
                continue;
            }
            generics_visitor.visit_type(&other_field.ty);
        }
        ob_generics.params = ob_generics
            .params
            .into_iter()
            .filter(|param| match param {
                syn::GenericParam::Const(param) => generics_visitor.contains_ty(&param.ident),
                syn::GenericParam::Type(param) => generics_visitor.contains_ty(&param.ident),
                syn::GenericParam::Lifetime(param) => generics_visitor.contains_lt(&param.lifetime),
            })
            .collect();
        if field_count > 1 {
            ob_generics.params.insert(0, parse_quote! { #ob_lt });
            ob_observer_generics.params.insert(0, parse_quote! { #ob_lt });
        }
        ob_generics.params.push(parse_quote! { #inner });
        ob_quasi_generics = ob_generics.clone();
        ob_quasi_generics.params.push(parse_quote! { #depth });
        ob_quasi_predicates = quote! {
            #inner: ::morphix::helper::QuasiObserver<InnerDepth = ::morphix::helper::Succ<#depth>>,
            #inner::Target: ::std::ops::Deref<Target: ::morphix::helper::AsDeref<#depth> + ::morphix::helper::AsDeref<::morphix::helper::Succ<#depth>>>,
        };
        ob_observer_generics.params.push(parse_quote! { #inner });
        ob_observer_generics.params.push(parse_quote! { #depth });
        ob_observer_predicates = quote! {
            #inner: ::morphix::observe::Observer<InnerDepth = ::morphix::helper::Succ<#depth>>,
            #inner::Head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics>,
        };

        deref_ident = syn::Ident::new("Deref", meta_deref_ident.span());
        deref_target = quote! { #inner };
        deref_member = quote! { #field_member };
        deref_mut_impl = quote! {};

        assignable_impl = quote! {
            type OuterDepth = ::morphix::helper::Succ<#inner::OuterDepth>;
        };

        let observer_uninit_expr = match is_named {
            true => quote! { Self { #uninit_fields } },
            false => quote! { Self (#uninit_fields) },
        };

        let observer_observe_expr = match is_named {
            true => quote! { Self { #observe_fields } },
            false => quote! { Self (#observe_fields) },
        };

        let prepare_value = if field_count > 1 {
            quote! {
                let __value = ::morphix::helper::AsDeref::<#depth>::as_deref(value);
            }
        } else {
            quote! {}
        };

        observer_impl = quote! {
            fn uninit() -> Self {
                #observer_uninit_expr
            }

            fn observe(value: &#inner::Head) -> Self {
                let __inner = ::morphix::observe::Observer::observe(value);
                #prepare_value
                #observer_observe_expr
            }

            unsafe fn refresh(this: &mut Self, value: &#inner::Head) {
                #prepare_value
                unsafe {
                    ::morphix::observe::Observer::refresh(&mut this.#field_member, value);
                    #refresh_stmts
                }
            }
        };

        serialize_observer_impl_prefix = quote! {};

        input_observe_predicates = quote! { #field_ty: ::morphix::Observe, };

        let ob_type_arguments = ob_generics.params.iter().map(|param| match param {
            syn::GenericParam::Type(ty_param) if ty_param.ident == inner => quote! { #ob_field_ty },
            _ => quote! { #param },
        });
        input_observer_type_generics = quote! { <#(#ob_type_arguments),*> };
    }

    let serialize_observer_impl = if flush_capacity.is_empty() {
        quote! {
            Ok(::morphix::Mutations::new())
        }
    } else {
        quote! {
            #pre_flush_stmts
            let mut mutations = ::morphix::Mutations::with_capacity(#(#flush_capacity)+*);
            #post_flush_stmts
            Ok(mutations)
        }
    };

    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
    let (ob_quasi_impl_generics, _, _) = ob_quasi_generics.split_for_impl();
    let (ob_observer_impl_generics, _, _) = ob_observer_generics.split_for_impl();

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

    let ob_item = match is_named {
        true => quote! {
            #input_vis struct #ob_ident #ob_generics
            where
                #(#input_predicates,)*
                #(#field_tys: ::morphix::Observe + #ob_lt),*
            {
                #ob_fields
            }
        },
        false => quote! {
            #input_vis struct #ob_ident #ob_generics (#ob_fields)
            where
                #(#input_predicates,)*
                #(#field_tys: ::morphix::Observe + #ob_lt),*;
        },
    };

    let mut output = quote! {
        #ob_item

        #[automatically_derived]
        impl #ob_impl_generics ::std::ops::#deref_ident
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            type Target = #deref_target;
            fn deref(&self) -> &Self::Target {
                &self.#deref_member
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
                #deref_mut_impl
                &mut self.#deref_member
            }
        }

        #[automatically_derived]
        impl #ob_quasi_impl_generics ::morphix::helper::QuasiObserver
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #ob_quasi_predicates
            #(#field_tys: ::morphix::Observe,)*
            #depth: ::morphix::helper::Unsigned,
        {
            #assignable_impl
            type InnerDepth = #depth;
        }

        #[automatically_derived]
        impl #ob_observer_impl_generics ::morphix::observe::Observer
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#skipped_tys: #ob_lt,)*
            #(#field_tys: ::morphix::Observe,)*
            #ob_observer_predicates
            #depth: ::morphix::helper::Unsigned,
        {
            #observer_impl
        }

        #[automatically_derived]
        impl #ob_observer_impl_generics ::morphix::observe::SerializeObserver
        for #ob_ident #ob_type_generics
        where
            #input_serialize_predicates
            #(#input_predicates,)*
            #(#skipped_tys: #ob_lt,)*
            #(#field_tys: ::morphix::Observe,)*
            #ob_observer_predicates
            #depth: ::morphix::helper::Unsigned,
            #(#ob_field_tys: ::morphix::observe::SerializeObserver,)*
        {
            unsafe fn flush_unchecked<A: ::morphix::Adapter>(
                this: &mut Self,
            ) -> ::std::result::Result<::morphix::Mutations<A::Value>, A::Error> {
                #serialize_observer_impl_prefix
                #serialize_observer_impl
            }
        }

        #[automatically_derived]
        impl #input_impl_generics ::morphix::Observe
        for #input_ident #input_type_generics
        where
            #self_serialize_predicates
            #input_observe_predicates
            #(#input_predicates,)*
            #(#field_tys: ::morphix::Observe,)*
        {
            type Observer<#ob_lt, #head, #depth> = #ob_ident #input_observer_type_generics
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
                impl #ob_observer_impl_generics ::std::fmt::#path
                for #ob_ident #ob_type_generics
                where
                    #(#input_predicates,)*
                    #(#skipped_tys: #ob_lt,)*
                    #(#field_tys: ::morphix::Observe,)*
                    #ob_observer_predicates
                    #depth: ::morphix::helper::Unsigned,
                {
                    #[inline]
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        let head = &**::morphix::helper::QuasiObserver::as_normalized_ref(self);
                        let value = ::morphix::helper::AsDeref::<N>::as_deref(head);
                        ::std::fmt::Display::fmt(value, f)
                    }
                }
            });
        } else if path.is_ident("Debug") {
            let method = match is_named {
                true => quote! { debug_struct },
                false => quote! { debug_tuple },
            };
            output.extend(quote! {
                #[automatically_derived]
                impl #ob_impl_generics ::std::fmt::Debug
                for #ob_ident #ob_type_generics
                where
                    #(#input_predicates,)*
                    #(#field_tys: ::morphix::Observe,)*
                    #(#ob_field_tys: ::std::fmt::Debug,)*
                {
                    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                        f.#method(#ob_name) #debug_chain .finish()
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
