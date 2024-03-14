use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, DataStruct, Field, LitStr};

use crate::utils::get_unique_attribute;

pub fn prototype_element_helper(
    data_struct: DataStruct,
    mut attributes: Vec<Attribute>,
    name: String,
) -> (Vec<TokenStream>, bool, TokenStream, Option<TokenStream>) {
    let (fields, is_unnamed): (Vec<Field>, bool) = match data_struct.fields {
        syn::Fields::Named(named_fields) => (named_fields.named.into_iter().collect(), false),
        syn::Fields::Unnamed(unnamed_fields) => (unnamed_fields.unnamed.into_iter().collect(), true),
        syn::Fields::Unit => panic!("unit types are not supported"),
    };

    let window_title = get_unique_attribute(&mut attributes, "window_title")
        .map(|attribute| attribute.parse_args().expect("failed to parse window title"))
        .map(|window_title: LitStr| quote!(#window_title))
        .unwrap_or(quote!(#name));

    let window_class = get_unique_attribute(&mut attributes, "window_class")
        .map(|attribute| attribute.parse_args().expect("failed to parse window class"))
        .map(|window_class: LitStr| quote!(#window_class));

    let mut initializers = vec![];

    let mut counter: usize = 0;
    for mut field in fields {
        if get_unique_attribute(&mut field.attrs, "hidden_element").is_some() {
            continue;
        }

        let counter_ident = format_ident!("_{}", counter);
        let counter_index = syn::Index::from(counter);
        let field_variable = field.ident.as_ref().map(|ident| quote!(#ident)).unwrap_or(quote!(#counter_ident));
        let field_identifier = field.ident.as_ref().map(|ident| quote!(#ident)).unwrap_or(quote!(#counter_index));
        counter += 1;

        let display_name = get_unique_attribute(&mut field.attrs, "name")
            .map(|attribute| attribute.parse_args().expect(""))
            .map(|name: LitStr| name.value())
            .unwrap_or_else(|| str::replace(&field_variable.to_string(), "_", " "));

        initializers
            .push(quote!(korangar_interface::elements::PrototypeElement::to_element(&self.#field_identifier, #display_name.to_string())));
    }

    (initializers, is_unnamed, window_title, window_class)
}
