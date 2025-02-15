use proc_macro::TokenStream;
use quote::{format_ident, quote};

#[proc_macro_derive(Observe)]
pub fn derive_observe(input: TokenStream) -> TokenStream {
    let derive: syn::DeriveInput = syn::parse_macro_input!(input);
    let ident = &derive.ident;
    let (impl_generics, type_generics, where_clause) = derive.generics.split_for_impl();
    let ident_ob = format_ident!("{}Ob", ident);
    let mut type_fields = vec![];
    let mut inst_fields = vec![];
    match &derive.data {
        syn::Data::Struct(syn::DataStruct {
            fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
            ..
        }) => {
            for name in named {
                let ident = name.ident.as_ref().unwrap();
                let ty = &name.ty;
                type_fields.push(quote! {
                    pub #ident: Ob<'i, #ty>,
                });
                inst_fields.push(quote! {
                    #ident: Ob {
                        value: &mut self.#ident,
                        path: prefix.to_string() + stringify!(#ident),
                        diff: diff.clone(),
                    },
                });
            }
        },
        _ => unimplemented!("not implemented"),
    };
    quote! {
        #[automatically_derived]
        impl #impl_generics Observe for #ident #type_generics #where_clause {
            type Target<'i> = #ident_ob<'i>;

            fn observe(&mut self, prefix: &str, diff: &Rc<RefCell<Vec<Delta>>>) -> Self::Target<'_> {
                #ident_ob {
                    #(#inst_fields)*
                }
            }
        }

        pub struct #ident_ob<'i> {
            #(#type_fields)*
        }
    }.into()
}
