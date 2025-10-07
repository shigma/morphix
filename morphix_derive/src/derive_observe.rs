use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::spanned::Spanned;

pub fn derive_observe(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let input_ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let ob_ident = format_ident!("{}Ob", input_ident);
    let input_vis = &input.vis;
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    let mut collect_stmts = vec![];
    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            for name in named {
                let ident = name.ident.as_ref().unwrap();
                let ty = &name.ty;
                type_fields.push(quote! {
                    pub #ident: <#ty as ::morphix::Observe>::Observer<'morphix>,
                });
                inst_fields.push(quote! {
                    #ident: value.#ident.observe(),
                });
                collect_stmts.push(quote! {
                    if let Some(mut change) = ::morphix::Observer::collect::<A>(&mut this.#ident)? {
                        change.path_rev.push(stringify!(#ident).into());
                        changes.push(change);
                    }
                });
            }
        }
        _ => {
            return Err(syn::Error::new(
                input.span(),
                "Observe can only be derived for named structs",
            ));
        }
    };
    Ok(quote! {
        const _: () = {
            #input_vis struct #ob_ident<'morphix> {
                ptr: *mut #input_ident,
                replaced: bool,
                phantom: ::std::marker::PhantomData<&'morphix mut #input_ident>,
                #(#type_fields)*
            }

            #[automatically_derived]
            impl #impl_generics Observe for #input_ident #type_generics #where_clause {
                type Observer<'morphix> = #ob_ident<'morphix>;
            }

            #[automatically_derived]
            impl<'morphix> ::morphix::Observer<'morphix, #input_ident> for #ob_ident<'morphix> {
                fn observe(value: &'morphix mut #input_ident) -> Self {
                    Self {
                        ptr: value as *mut #input_ident,
                        replaced: false,
                        phantom: ::std::marker::PhantomData,
                        #(#inst_fields)*
                    }
                }

                fn collect<A: ::morphix::Adapter>(
                    this: &mut Self,
                ) -> ::std::result::Result<::std::option::Option<::morphix::Change<A>>, A::Error> {
                    let mut changes = vec![];
                    if this.replaced {
                        changes.push(::morphix::Change {
                            path_rev: vec![],
                            operation: ::morphix::Operation::Replace(A::new_replace(&**this)?),
                        });
                    };
                    #(#collect_stmts)*
                    Ok(::morphix::Batch::build(changes))
                }
            }

            #[automatically_derived]
            impl<'morphix> ::std::ops::Deref for #ob_ident<'morphix> {
                type Target = #input_ident;
                fn deref(&self) -> &Self::Target {
                    unsafe { &*self.ptr }
                }
            }

            #[automatically_derived]
            impl<'morphix> ::std::ops::DerefMut for #ob_ident<'morphix> {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    self.replaced = true;
                    unsafe { &mut *self.ptr }
                }
            }
        };
    })
}
