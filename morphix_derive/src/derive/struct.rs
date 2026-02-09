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

    let mut type_fields = quote! {};
    let mut inst_fields = quote! {};
    let mut uninit_fields = quote! {};
    let mut refresh_stmts = vec![];
    let mut flush_fields = quote! {};
    let mut push_mutations = quote! {};
    let mut mutation_batch_capacity = vec![];
    let mut debug_chain = quote! {};

    let mut field_tys = vec![];
    let mut deref_erased_tys = vec![];
    let mut ob_field_tys = vec![];
    let mut deref_fields = vec![];
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
        let ob_field_ty = if let Some(deref_ident) = field_meta.deref {
            let ob_field_ty = match &field_meta.general_impl {
                None => quote_spanned! { field_span =>
                    ::morphix::observe::DefaultObserver<#ob_lt, #field_ty, #head, ::morphix::helper::Succ<#depth>>
                },
                Some(GeneralImpl { ob_ident, .. }) => quote_spanned! { field_span =>
                    ::morphix::builtin::#ob_ident<#ob_lt, #head, ::morphix::helper::Succ<#depth>>
                },
            };
            deref_fields.push((i, field, ob_field_ty, deref_ident));
            inst_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* __inner,
            });
            ob_field_tys.push(quote! { #inner });
            quote! { #inner }
        } else {
            inst_fields.extend(quote_spanned! { field_span =>
                #(#if_named #field_ident:)* ::morphix::observe::Observer::observe(&mut __value.#field_member),
            });
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
            refresh_stmts.push(quote_spanned! { field_span =>
                ::morphix::observe::Observer::refresh(&mut this.#field_member, &mut __value.#field_member);
            });
            ob_field_ty
        };
        type_fields.extend(quote_spanned! { field_span =>
            #field_vis #(#if_named #field_ident:)* #ob_field_ty,
        });
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
        flush_fields.extend(quote_spanned! { field_span =>
            let #mutable_ident = ::morphix::observe::SerializeObserver::flush::<A>(&mut this.#field_member)?;
        });
        mutation_batch_capacity.push(quote_spanned! { field_span =>
            #mutable_ident.len()
        });
        if !field_meta.serde.flatten {
            let segment = if let Some(rename) = &field_meta.serde.rename {
                quote! { #rename }
            } else {
                default_segment
            };
            push_mutations.extend(quote_spanned! { field_span =>
                mutations.insert(#segment, #mutable_ident);
            });
        } else {
            push_mutations.extend(quote_spanned! { field_span =>
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
    let mut ob_observer_generics = input_generics.clone();

    let deref_ident;
    let deref_target;
    let deref_member;
    let deref_mut_impl;
    let assignable_impl;
    let observer_impl;
    let serialize_observer_impl_prefix;
    let ob_assignable_predicates;
    let ob_observer_predicates;
    let input_observe_predicates;
    let input_observer_type_generics;

    if deref_fields.is_empty() {
        ob_generics.params.insert(0, parse_quote! { #ob_lt });
        ob_generics.params.push(parse_quote! { #head: ?Sized });
        ob_generics
            .params
            .push(parse_quote! { #depth = ::morphix::helper::Zero });
        ob_assignable_predicates = quote! {};
        ob_observer_generics.params.insert(0, parse_quote! { #ob_lt });
        ob_observer_generics.params.push(parse_quote! { #head: ?Sized });
        ob_observer_generics
            .params
            .push(parse_quote! { #depth = ::morphix::helper::Zero });
        ob_observer_predicates = quote! {
            #head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics> + #ob_lt,
        };

        type_fields.extend(quote! {
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
                    #inst_fields
                    __ptr,
                    __mutated: false,
                    __phantom: ::std::marker::PhantomData,
                }
            },
            false => quote! {
                Self (
                    #inst_fields
                    __ptr,
                    false,
                    ::std::marker::PhantomData,
                )
            },
        };

        observer_impl = quote! {
            type Head = #head;
            type InnerDepth = #depth;

            fn uninit() -> Self {
                #observer_uninit_expr
            }

            fn observe(value: &#ob_lt mut #head) -> Self {
                let __ptr = ::morphix::helper::Pointer::new(value);
                let __value = value.as_deref_mut();
                #observer_observe_expr
            }

            unsafe fn refresh(this: &mut Self, value: &mut #head) {
                ::morphix::helper::Pointer::set(this, value);
                let __value = value.as_deref_mut();
                unsafe {
                    #(#refresh_stmts)*
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
                syn::GenericParam::Type(ty_param) => {
                    let ident = &ty_param.ident;
                    let is_retain = generics_visitor.contains_ty(ident);
                    if !is_retain {
                        deref_erased_tys.push(quote! { #ident });
                    }
                    is_retain
                }
                syn::GenericParam::Lifetime(lt_param) => generics_visitor.contains_lt(&lt_param.lifetime),
                syn::GenericParam::Const(_) => true,
            })
            .collect();
        ob_generics.params.insert(0, parse_quote! { #ob_lt });
        ob_generics.params.push(parse_quote! { #inner });
        ob_assignable_predicates = quote! {
            #inner: ::morphix::helper::AsNormalized,
        };
        ob_observer_generics.params.insert(0, parse_quote! { #ob_lt });
        ob_observer_generics.params.push(parse_quote! { #inner });
        ob_observer_generics.params.push(parse_quote! { #depth });
        ob_observer_predicates = quote! {
            #inner: ::morphix::observe::Observer<#ob_lt, InnerDepth = ::morphix::helper::Succ<#depth>>,
            #inner::Head: ::morphix::helper::AsDerefMut<#depth, Target = #input_ident #input_type_generics>,
        };

        type_fields.extend(quote! {
            #(#if_named __phantom:)* ::std::marker::PhantomData<&#ob_lt mut ()>,
        });

        deref_ident = syn::Ident::new("Deref", meta_deref_ident.span());
        deref_target = quote! { #inner };
        deref_member = quote! { #field_member };
        deref_mut_impl = quote! {};

        assignable_impl = quote! {
            type OuterDepth = ::morphix::helper::Succ<#inner::OuterDepth>;
        };

        let observer_uninit_expr = match is_named {
            true => quote! {
                Self {
                    #uninit_fields
                    #(#if_named __phantom:)* ::std::marker::PhantomData,
                }
            },
            false => quote! {
                Self (
                    #uninit_fields
                    ::std::marker::PhantomData
                )
            },
        };

        let observer_observe_expr = match is_named {
            true => quote! {
                Self {
                    #inst_fields
                    #(#if_named __phantom:)* ::std::marker::PhantomData,
                }
            },
            false => quote! {
                Self (
                    #inst_fields
                    ::std::marker::PhantomData
                )
            },
        };

        observer_impl = quote! {
            type Head = #inner::Head;
            type InnerDepth = #depth;

            fn uninit() -> Self {
                #observer_uninit_expr
            }

            fn observe(value: &#ob_lt mut #inner::Head) -> Self {
                let __inner = ::morphix::observe::Observer::observe(unsafe { &mut *(value as *mut #inner::Head) });
                let __value = ::morphix::helper::AsDerefMut::<#depth>::as_deref_mut(value);
                #observer_observe_expr
            }

            unsafe fn refresh(this: &mut Self, value: &mut #inner::Head) {
                unsafe {
                    ::morphix::observe::Observer::refresh(&mut this.#field_member, value);
                    let __value = ::morphix::helper::AsDerefMut::<#depth>::as_deref_mut(value);
                    #(#refresh_stmts)*
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

    let serialize_observer_impl = quote! {
        #flush_fields
        let mut mutations = ::morphix::Mutations::with_capacity(#(#mutation_batch_capacity)+*);
        #push_mutations
        Ok(mutations)
    };

    let (ob_impl_generics, ob_type_generics, _) = ob_generics.split_for_impl();
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
                #type_fields
            }
        },
        false => quote! {
            #input_vis struct #ob_ident #ob_generics (#type_fields)
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
        impl #ob_impl_generics ::morphix::helper::AsNormalized
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #ob_assignable_predicates
            #(#field_tys: ::morphix::Observe,)*
        {
            #assignable_impl
        }

        #[automatically_derived]
        impl #ob_observer_impl_generics ::morphix::observe::Observer<#ob_lt>
        for #ob_ident #ob_type_generics
        where
            #(#input_predicates,)*
            #(#deref_erased_tys: #ob_lt,)*
            #(#field_tys: ::morphix::Observe,)*
            #ob_observer_predicates
            #depth: ::morphix::helper::Unsigned,
        {
            #observer_impl
        }

        #[automatically_derived]
        impl #ob_observer_impl_generics ::morphix::observe::SerializeObserver<#ob_lt>
        for #ob_ident #ob_type_generics
        where
            #input_serialize_predicates
            #(#input_predicates,)*
            #(#deref_erased_tys: #ob_lt,)*
            #(#field_tys: ::morphix::Observe,)*
            #ob_observer_predicates
            #depth: ::morphix::helper::Unsigned,
            #(#ob_field_tys: ::morphix::observe::SerializeObserver<#ob_lt>,)*
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
                    #(#deref_erased_tys: #ob_lt,)*
                    #(#field_tys: ::morphix::Observe,)*
                    #ob_observer_predicates
                    #depth: ::morphix::helper::Unsigned,
                {
                    #[inline]
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        let head = &**::morphix::helper::AsNormalized::as_normalized_ref(self);
                        let value = ::morphix::helper::AsDeref::<N>::as_deref(head);
                        ::std::fmt::Display::fmt(value, f)
                    }
                }
            });
        } else if path.is_ident("Debug") {
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
                        f.debug_struct(#ob_name) #debug_chain .finish()
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
