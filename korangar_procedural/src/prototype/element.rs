use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataEnum, DataStruct, Generics, Ident};

use super::helper::prototype_element_helper;

pub fn derive_prototype_element_struct(
    data_struct: DataStruct,
    generics: Generics,
    attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (initializers, is_unnamed, _window_title, _window_class) = prototype_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    if std::env::var("CARGO_PKG_NAME").unwrap() == "korangar" {
        if initializers.len() == 1 && is_unnamed {
            return quote! {
                impl #impl_generics korangar_interface::elements::PrototypeElement<crate::interface::application::InterfaceSettings> for #name #type_generics #where_clause {
                    fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<crate::interface::application::InterfaceSettings> {
                        korangar_interface::elements::PrototypeElement::to_element(&self.0, display)
                    }
                }
            }
            .into();
        }

        return quote! {
            impl #impl_generics korangar_interface::elements::PrototypeElement<crate::interface::application::InterfaceSettings> for #name #type_generics #where_clause {
                fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<crate::interface::application::InterfaceSettings> {
                    let elements: Vec<korangar_interface::elements::ElementCell<crate::interface::application::InterfaceSettings>> = vec![#(#initializers),*];
                    std::rc::Rc::new(std::cell::RefCell::new(korangar_interface::elements::Expandable::new(display, elements, false)))
                }
            }
        }
        .into();
    }

    if initializers.len() == 1 && is_unnamed {
        return quote! {
            impl<App: korangar_interface::settings::Application> #impl_generics korangar_interface::elements::PrototypeElement<App> for #name #type_generics #where_clause {
                fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<App> {
                    korangar_interface::elements::PrototypeElement::to_element(&self.0, display)
                }
            }
        }
        .into();
    }

    quote! {
        impl<App: korangar_interface::settings::Application> #impl_generics korangar_interface::elements::PrototypeElement<App> for #name #type_generics #where_clause {
            fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<App> {
                let elements: Vec<korangar_interface::elements::ElementCell<App>> = vec![#(#initializers),*];
                std::rc::Rc::new(std::cell::RefCell::new(korangar_interface::elements::Expandable::new(display, elements, false)))
            }
        }
    }
    .into()
}

pub fn derive_prototype_element_enum(data_enum: DataEnum, generics: Generics, name: Ident) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let mut variants = Vec::new();
    let mut variant_strings = Vec::new();

    for variant in data_enum.variants.into_iter() {
        variants.push(variant.ident.clone());
        variant_strings.push(variant.ident.to_string());
    }

    if std::env::var("CARGO_PKG_NAME").unwrap() == "korangar" {
        return quote! {
            impl #impl_generics korangar_interface::elements::PrototypeElement<crate::interface::application::InterfaceSettings> for #name #type_generics #where_clause {
                fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<crate::interface::application::InterfaceSettings> {
                    match self {
                        #( Self::#variants => korangar_interface::elements::PrototypeElement::to_element(&#variant_strings, display), )*
                    }
                }
            }
        }
        .into();
    }

    quote! {
        impl<App: korangar_interface::settings::Application> #impl_generics korangar_interface::elements::PrototypeElement<App> for #name #type_generics #where_clause {
            fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<App> {
                match self {
                    #( Self::#variants => korangar_interface::elements::PrototypeElement::to_element(&#variant_strings, display), )*
                }
            }
        }
    }
    .into()
}
