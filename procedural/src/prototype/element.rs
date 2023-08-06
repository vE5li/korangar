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

    if initializers.len() == 1 && is_unnamed {
        return quote! {
            impl #impl_generics crate::interface::PrototypeElement for #name #type_generics #where_clause {
                fn to_element(&self, display: String) -> crate::interface::ElementCell {
                    crate::interface::PrototypeElement::to_element(&self.0, display)
                }
            }
        }
        .into();
    }

    quote! {
        impl #impl_generics crate::interface::PrototypeElement for #name #type_generics #where_clause {
            fn to_element(&self, display: String) -> crate::interface::ElementCell {
                let elements: Vec<crate::interface::ElementCell> = vec![#(#initializers),*];
                std::rc::Rc::new(std::cell::RefCell::new(crate::interface::Expandable::new(display, elements, false)))
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

    quote! {
        impl #impl_generics crate::interface::PrototypeElement for #name #type_generics #where_clause {
            fn to_element(&self, display: String) -> crate::interface::ElementCell {
                match self {
                    #( Self::#variants => crate::interface::PrototypeElement::to_element(&#variant_strings, display), )*
                }
            }
        }
    }
    .into()
}
