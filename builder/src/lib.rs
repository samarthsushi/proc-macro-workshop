use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};
use quote::quote;

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
        if unwrap_wrapper_t("Option", ty).is_some() || builder_of(&f) {
            return quote! { #field_name: #ty };
        }
        quote! { #field_name: std::option::Option<#ty> }
    });
    let methods = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        let setter_method = if let Some(inner_ty) = unwrap_wrapper_t("Option",ty) {
            quote! {
                fn #field_name(&mut self, #field_name: #inner_ty) -> &mut Self {
                    self.#field_name = std::option::Option::Some(#field_name);
                    self
                }
            }
        } else if builder_of(&f) {
            quote! {
                fn #field_name(&mut self, #field_name: #ty) -> &mut Self {
                    self.#field_name = #field_name;
                    self
                }
            }
        } else {
            quote! {
                fn #field_name(&mut self, #field_name: #ty) -> &mut Self {
                    self.#field_name = std::option::Option::Some(#field_name);
                    self
                }
            }
        };
        match extended_methods(&f) {
            None => setter_method,
            Some((true, extend_method)) => extend_method,
            Some((false, extend_method)) =>  quote! {
                #setter_method
                #extend_method
            }
        }

    });
   
    let build_method = fields.iter().map(|f| {
        let field_name = &f.ident;
        let ty = &f.ty;
        if unwrap_wrapper_t("Option", ty).is_some() || builder_of(&f) {
            let expr = quote! {
                #field_name: self.#field_name.clone()
            };
            return expr;
        }
        quote! {
            #field_name: self.#field_name.clone().ok_or(concat!(stringify!(#field_name), " is not set"))?
        }
    });
    let build_empty = fields.iter().map(|f| {
        let field_name = &f.ident;
        if builder_of(&f) {
            quote! { #field_name: std::vec::Vec::new() }
        } else {
            quote! { #field_name: std::option::Option::None }
        }
    });
    let expanded = quote! {
        struct #builder_ident {
            #(#fields_after_option_types,)*
        }
        impl #builder_ident {
            #(#methods)*

            fn build(&self) -> std::result::Result<#name, std::boxed::Box<dyn std::error::Error>> {
                std::result::Result::Ok (#name {
                    #(#build_method,)*
                })
            }
        }
        impl #name {
            fn builder() -> #builder_ident {
                #builder_ident {
                    #(#build_empty,)*
                }
            }
        }
    };
    TokenStream::from(expanded)
}

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

fn builder_of(f: &syn::Field) -> bool {
    for attr in &f.attrs {
        if attr.path().is_ident("builder") {
            return true;
        }
    }
    false 
}

fn extended_methods(f: &syn::Field) -> Option<(bool, proc_macro2::TokenStream)> {
    let field_name = &f.ident;
    let mut avoid_conflict = false;

    for attr in &f.attrs {
        if attr.path().is_ident("builder") {
            let mut lit = None;

            let result = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("each") {
                    lit = Some(meta.value()?.parse::<syn::LitStr>()?);
                    Ok(())
                } else {
                    Err(meta.error("expected `builder(each = \"...\")`"))
                }
            });

            if let Err(err) = result {
                return Some((false, err.to_compile_error()));
            }

            if let Some(lit) = lit {
                let extend_fn_name = syn::Ident::new(&lit.value(), lit.span());

                if field_name.as_ref() == Some(&extend_fn_name) {
                    avoid_conflict = true;
                }

                let inner_ty = unwrap_wrapper_t("Vec", &f.ty).unwrap_or_else(|| {
                    panic!(
                        "Field with `builder(each = ...)` must be of type `Vec`. Field: {:?}",
                        field_name
                    );
                });

                let expanded = quote! {
                    fn #extend_fn_name(&mut self, #extend_fn_name: #inner_ty) -> &mut Self {
                        self.#field_name.push(#extend_fn_name);
                        self
                    }
                };

                return Some((avoid_conflict, expanded));
            }
        }
    }

    None
}
