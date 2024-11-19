use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

fn unwrap_wrapper_t<'a>(wrapper_t: &'a str, ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(ref p) = ty {
        if p.path.segments.len() != 1 || p.path.segments[0].ident != wrapper_t {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(ref inner_ty) = p.path.segments[0].arguments {
            if inner_ty.args.len() != 1 {
                return None;
            }
            let inner_ty = inner_ty.args.first().unwrap();
            if let syn::GenericArgument::Type(ref t) = inner_ty{
                return Some(t);
            }
        }
    }
    None
}

#[proc_macro_derive(Builder, attributes(builder))]
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
        if unwrap_wrapper_t("Option", ty).is_some() {
            return quote! { #field_name: #ty };
        }
        quote! { #field_name: std::option::Option<#ty> }
    });
    let setter_methods = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        if let Some(inner_ty) = unwrap_wrapper_t("Option",ty) {
            quote! {
                fn #field_name(&mut self, #field_name: #inner_ty) -> &mut Self {
                    self.#field_name = Some(#field_name);
                    self
                }
            }
        } else {
            quote! {
                fn #field_name(&mut self, #field_name: #ty) -> &mut Self {
                    self.#field_name = Some(#field_name);
                    self
                }
            }
        }
    });
    let extend_methods = fields.iter().filter_map(|f| {
        let field_name = &f.ident;
        for attr in &f.attrs {
            if attr.path().is_ident("builder") {
                let mut expanded = None;
                let _ = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("each") {
                        let lit: syn::LitStr = meta.value().unwrap().parse().unwrap();
                        let extend_fn_name = syn::Ident::new(&lit.value(), lit.span());
                        let inner_ty = unwrap_wrapper_t("Vec", &f.ty).unwrap();

                        expanded = Some(quote! {
                            fn #extend_fn_name(&mut self, #extend_fn_name: #inner_ty) -> &mut Self {
                                if let Some(ref mut values) = self.#field_name {
                                    values.push(#extend_fn_name);
                                } else {
                                    self.#field_name = Some(vec![#extend_fn_name]);
                                }
                                self
                            }
                        });
                        Ok(())
                    } else {
                        return Err(meta.error("expected 'each'"));
                    }
                });
                return expanded;
            }

        }
        None    
    });
    let build_method = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        if unwrap_wrapper_t("Option", ty).is_some() {
            let expr = quote! {
                #field_name: self.#field_name.clone()
            };
            return expr;
        }
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
            #(#extend_methods)*

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