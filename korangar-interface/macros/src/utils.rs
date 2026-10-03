use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Generics, Type, parse_quote};

pub fn get_unique_attribute(attributes: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
    let mut matching_attributes = attributes.extract_if(.., |attribute| attribute.path().segments[0].ident == name);
    let return_attribute = matching_attributes.next();

    if matching_attributes.next().is_some() {
        panic!("attribute {} may only be specified once per field", name);
    }

    return_attribute
}

/// Everything needed to write the header of an implementation for an
/// application.
pub struct ImplTarget {
    pub impl_generics: TokenStream,
    pub type_generics: TokenStream,
    pub where_clause: TokenStream,
    /// The application type to implement for. Either the type specified with
    /// `#[impl_for(...)]`, or a generic `App` parameter that is part of
    /// `impl_generics`.
    pub app: TokenStream,
}

/// Determine the application type to implement a trait for.
///
/// By default, the trait is implemented for any application, using a generic
/// `App` parameter. Adding `#[impl_for(MyApplication)]` to the type instead
/// implements it only for `MyApplication`. This is needed if the elements of
/// any field are only available for a specific application.
pub fn get_impl_target(attributes: &mut Vec<Attribute>, generics: &Generics) -> ImplTarget {
    let impl_for = get_unique_attribute(attributes, "impl_for").map(|attribute| {
        attribute
            .parse_args::<Type>()
            .expect("failed to parse impl_for, expected `#[impl_for(MyApplication)]`")
    });

    let (_, type_generics, _) = generics.split_for_impl();
    let type_generics = quote!(#type_generics);

    match impl_for {
        Some(impl_for) => {
            let (impl_generics, _, where_clause) = generics.split_for_impl();

            ImplTarget {
                impl_generics: quote!(#impl_generics),
                type_generics,
                where_clause: quote!(#where_clause),
                app: quote!(#impl_for),
            }
        }
        None => {
            // Lifetimes need to come before type parameters.
            let mut generics = generics.clone();
            let position = generics.lifetimes().count();
            generics
                .params
                .insert(position, parse_quote!(App: korangar_interface::application::Application));
            let (impl_generics, _, where_clause) = generics.split_for_impl();

            ImplTarget {
                impl_generics: quote!(#impl_generics),
                type_generics,
                where_clause: quote!(#where_clause),
                app: quote!(App),
            }
        }
    }
}
