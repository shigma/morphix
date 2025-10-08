use proc_macro2::TokenStream;
use quote::{format_ident, quote};
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

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ObserveMode {
    #[default]
    Default,
    Ignore,
    Shallow,
}

#[derive(Default)]
struct ObserveMeta {
    mode: ObserveMode,
}

pub fn derive_observe(input: syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let input_ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let ob_ident = format_ident!("{}Ob", input_ident);
    let input_vis = &input.vis;
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    let mut collect_stmts = vec![];
    let mut errors = vec![];
    match &input.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            for field in named {
                let mut meta = ObserveMeta::default();
                for attr in &field.attrs {
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
                        if ident == "ignore" {
                            meta.mode = ObserveMode::Ignore;
                        } else if ident == "shallow" {
                            meta.mode = ObserveMode::Shallow;
                        } else {
                            errors.push(syn::Error::new(
                                ident.span(),
                                "unknown argument, expected 'ignore' or 'shallow'",
                            ));
                        }
                    }
                }
                let field_ident = field.ident.as_ref().unwrap();
                let field_ty = &field.ty;
                match meta.mode {
                    ObserveMode::Default => {
                        type_fields.push(quote! {
                            pub #field_ident: <#field_ty as ::morphix::Observe>::Observer<'morphix>,
                        });
                        inst_fields.push(quote! {
                            #field_ident: ::morphix::Observe::observe(&mut value.#field_ident),
                        });
                    }
                    ObserveMode::Ignore => {
                        type_fields.push(quote! {
                            pub #field_ident: &'morphix mut #field_ty,
                        });
                        inst_fields.push(quote! {
                            #field_ident: &mut value.#field_ident,
                        });
                    }
                    ObserveMode::Shallow => {
                        type_fields.push(quote! {
                            pub #field_ident: ::morphix::ShallowObserver<'morphix, #field_ty>,
                        });
                        inst_fields.push(quote! {
                            #field_ident: ::morphix::ShallowObserver::new(&mut value.#field_ident),
                        });
                    }
                }
                if meta.mode != ObserveMode::Ignore {
                    collect_stmts.push(quote! {
                        if let Some(mut mutation) = ::morphix::Observer::collect::<A>(this.#field_ident)? {
                            mutation.path_rev.push(stringify!(#field_ident).into());
                            mutations.push(mutation);
                        }
                    });
                }
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
            impl<'morphix> ::morphix::Observer<'morphix> for #ob_ident<'morphix> {
                fn observe(value: &'morphix mut #input_ident) -> Self {
                    Self {
                        ptr: value as *mut #input_ident,
                        replaced: false,
                        phantom: ::std::marker::PhantomData,
                        #(#inst_fields)*
                    }
                }

                fn collect<A: ::morphix::Adapter>(
                    this: Self,
                ) -> ::std::result::Result<::std::option::Option<::morphix::Mutation<A>>, A::Error> {
                    let mut mutations = vec![];
                    if this.replaced {
                        mutations.push(::morphix::Mutation {
                            path_rev: vec![],
                            operation: ::morphix::MutationKind::Replace(A::serialize_value(&*this)?),
                        });
                    };
                    #(#collect_stmts)*
                    Ok(::morphix::Batch::build(mutations))
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
