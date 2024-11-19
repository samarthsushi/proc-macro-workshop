use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    let builder_name = format!("{}Builder", name);
    let builder_ident = syn::Ident::new(&builder_name, name.span());
    let fields = if let syn::Data::Struct(syn::DataStruct { 
        fields: syn::Fields::Named(syn::FieldsNamed {ref named, ..}), 
        ..
    }) = ast.data {
        named
    } else {
        unimplemented!();
    };

    let fields_after_option_types = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        quote! { #field_name: std::option::Option<#ty> }
    });
    let setter_methods = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        quote! {
            fn #field_name(&mut self, #field_name: #ty) -> &mut Self {
                self.#field_name = Some(#field_name);
                self
            }
        }
    });
    let build_method = fields.iter().map(|f| {
        let field_name = &f.ident;
        quote! {
            #field_name: self.#field_name.clone().ok_or(concat!(stringify!(#field_name), "is not set"))?
        }
    });
    let expanded = quote! {
        struct #builder_ident {
            #(#fields_after_option_types,)*
        }
        impl #builder_ident {
            #(#setter_methods)*

            fn build(&self) -> Result<#name, Box<dyn std::error::Error>> {
                Ok (#name {
                    #(#build_method,)*
                })
            }
        }
        impl #name {
            fn builder() -> #builder_ident {
                #builder_ident {
                    executable: None,
                    args: None,
                    env: None,
                    current_dir: None,
                }
            }
        }
    };
    TokenStream::from(expanded)
}
