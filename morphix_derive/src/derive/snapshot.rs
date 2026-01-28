use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse_quote;
use syn::spanned::Spanned;

use crate::derive::GenericsDetector;

pub fn derive_snapshot(input: &syn::DeriveInput) -> TokenStream {
    let mut generics = input.generics.clone();
    let where_predicates = &mut generics.make_where_clause().predicates;
    match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields) => {
                for field in &fields.named {
                    let field_ty = &field.ty;
                    if GenericsDetector::detect(field_ty, &input.generics) {
                        where_predicates.push(parse_quote! {
                            #field_ty: ::morphix::builtin::Snapshot
                        });
                    }
                }
            }
            syn::Fields::Unnamed(fields) => {
                for field in &fields.unnamed {
                    let field_ty = &field.ty;
                    if GenericsDetector::detect(field_ty, &input.generics) {
                        where_predicates.push(parse_quote! {
                            #field_ty: ::morphix::builtin::Snapshot
                        });
                    }
                }
            }
            syn::Fields::Unit => {}
        },
        syn::Data::Enum(data_enum) => {
            for variant in &data_enum.variants {
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        for field in &fields.named {
                            let field_ty = &field.ty;
                            if GenericsDetector::detect(field_ty, &input.generics) {
                                where_predicates.push(parse_quote! {
                                    #field_ty: ::morphix::builtin::Snapshot
                                });
                            }
                        }
                    }
                    syn::Fields::Unnamed(fields) => {
                        for field in &fields.unnamed {
                            let field_ty = &field.ty;
                            if GenericsDetector::detect(field_ty, &input.generics) {
                                where_predicates.push(parse_quote! {
                                    #field_ty: ::morphix::builtin::Snapshot
                                });
                            }
                        }
                    }
                    syn::Fields::Unit => {}
                }
            }
        }
        syn::Data::Union(_data_union) => {
            return syn::Error::new(input.span(), "PartialEq cannot be derived for unions").to_compile_error();
        }
    }

    let (to_snapshot, cmp_snapshot) = match &input.data {
        syn::Data::Struct(data_struct) => match &data_struct.fields {
            syn::Fields::Named(fields) => {
                let field_values = fields.named.iter().map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let span = field.span();
                    quote_spanned! { span => #ident: ::morphix::builtin::Snapshot::to_snapshot(&self.#ident) }
                });
                let cmp_values = fields.named.iter().map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let span = field.span();
                    quote_spanned! { span => ::morphix::builtin::Snapshot::cmp_snapshot(&self.#ident, &snapshot.#ident) }
                });
                (quote! { Self { #(#field_values),* } }, quote! { #(#cmp_values) &&* })
            }
            syn::Fields::Unnamed(fields) => {
                let field_values = fields.unnamed.iter().enumerate().map(|(i, field)| {
                    let index = syn::Index::from(i);
                    let span = field.span();
                    quote_spanned! { span => ::morphix::builtin::Snapshot::to_snapshot(&self.#index) }
                });
                let cmp_values = fields.unnamed.iter().enumerate().map(|(i, field)| {
                    let index = syn::Index::from(i);
                    let span = field.span();
                    quote_spanned! { span => ::morphix::builtin::Snapshot::cmp_snapshot(&self.#index, &snapshot.#index) }
                });
                (quote! { Self ( #(#field_values),* ) }, quote! { #(#cmp_values) &&* })
            }
            syn::Fields::Unit => (quote! { Self }, quote! { true }),
        },
        syn::Data::Enum(data_enum) => {
            let (to_snapshot, cmp_snapshot): (Vec<_>, Vec<_>) = data_enum.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                match &variant.fields {
                    syn::Fields::Named(fields) => {
                        let field_idents = fields
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect::<Vec<_>>();
                        let field_values = fields.named.iter().map(|field| {
                            let ident = field.ident.as_ref().unwrap();
                            let span = field.span();
                            quote_spanned! { span => #ident: ::morphix::builtin::Snapshot::to_snapshot(#ident) }
                        });
                        let self_idents = fields
                            .named
                            .iter()
                            .enumerate()
                            .map(|(i, f)| syn::Ident::new(&format!("__self_{}", i), f.span()))
                            .collect::<Vec<_>>();
                        let snap_idents = fields
                            .named
                            .iter()
                            .enumerate()
                            .map(|(i, f)| syn::Ident::new(&format!("__snap_{}", i), f.span()))
                            .collect::<Vec<_>>();
                        let cmp_values = fields.named.iter().enumerate().map(|(i, field)| {
                            let span = field.span();
                            let self_ident = &self_idents[i];
                            let snap_ident = &snap_idents[i];
                            quote_spanned! { span => ::morphix::builtin::Snapshot::cmp_snapshot(&#self_ident, &#snap_ident) }
                        });
                        (
                            quote! {
                                Self::#variant_ident { #(#field_idents),* } => Self::#variant_ident { #(#field_values),* }
                            },
                            quote! {
                                (
                                    Self::#variant_ident { #(#field_idents: #self_idents),* },
                                    Self::#variant_ident { #(#field_idents: #snap_idents),* },
                                ) => #(#cmp_values) &&*
                            },
                        )
                    }
                    syn::Fields::Unnamed(fields) => {
                        let field_idents = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, field)| syn::Ident::new(&format!("__self_{}", i), field.span()))
                            .collect::<Vec<_>>();
                        let field_values = field_idents.iter().map(|ident| {
                            let span = ident.span();
                            quote_spanned! { span => ::morphix::builtin::Snapshot::to_snapshot(#ident) }
                        });
                        let self_idents = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, field)| syn::Ident::new(&format!("__self_{}", i), field.span()))
                            .collect::<Vec<_>>();
                        let snap_idents = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(i, field)| syn::Ident::new(&format!("__snap_{}", i), field.span()))
                            .collect::<Vec<_>>();
                        let cmp_values = fields.unnamed.iter().enumerate().map(|(i, field)| {
                            let span = field.span();
                            let self_ident = &self_idents[i];
                            let snap_ident = &snap_idents[i];
                            quote_spanned! { span => ::morphix::builtin::Snapshot::cmp_snapshot(&#self_ident, &#snap_ident) }
                        });
                        (
                            quote! {
                                Self::#variant_ident( #(#field_idents),* ) => Self::#variant_ident( #(#field_values),* )
                            },
                            quote! {
                                (
                                    Self::#variant_ident( #(#self_idents),* ),
                                    Self::#variant_ident( #(#snap_idents),* ),
                                ) => #(#cmp_values) &&*
                            },
                        )
                    }
                    syn::Fields::Unit => (
                        quote! { Self::#variant_ident => Self::#variant_ident },
                        quote! { (Self::#variant_ident, Self::#variant_ident) => true },
                    ),
                }
            }).unzip();
            (
                quote! {
                    match self {
                        #(#to_snapshot),*
                    }
                },
                quote! {
                    match (self, snapshot) {
                        #(#cmp_snapshot),*
                    }
                },
            )
        }
        syn::Data::Union(_data_union) => unreachable!(),
    };

    let input_ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    quote! {
        const _: () = {
            #[automatically_derived]
            impl #impl_generics ::morphix::builtin::Snapshot for #input_ident #ty_generics #where_clause {
                type Value = Self;
                #[inline]
                fn to_snapshot(&self) -> Self {
                    #to_snapshot
                }
                #[inline]
                fn cmp_snapshot(&self, snapshot: &Self) -> bool {
                    #cmp_snapshot
                }
            }
        };
    }
}
