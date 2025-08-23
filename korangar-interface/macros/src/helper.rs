use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Attribute, DataStruct, Field, LitStr};

use crate::utils::get_unique_attribute;

pub fn state_element_helper(
    data_struct: DataStruct,
    mut attributes: Vec<Attribute>,
    name: String,
) -> (Vec<TokenStream>, Vec<TokenStream>, bool, TokenStream, Option<TokenStream>) {
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
        .map(|attribute| attribute.parse_args().expect("failed to parse window class"));

    let mut initializers = vec![];
    let mut initializers_mut = vec![];

    let mut counter: usize = 0;
    for mut field in fields {
        if get_unique_attribute(&mut field.attrs, "hidden_element").is_some() {
            continue;
        }

        let counter_ident = format_ident!("_{}", counter);
        let field_variable = field.ident.as_ref().map(|ident| quote!(#ident)).unwrap_or(quote!(#counter_ident));
        let field_identifier = field.ident.as_ref().map(|ident| quote!(#ident)).unwrap_or(quote!(#counter_ident));
        counter += 1;

        let display_name = get_unique_attribute(&mut field.attrs, "name")
            .map(|attribute| attribute.parse_args().expect(""))
            .map(|name: LitStr| name.value())
            .unwrap_or_else(|| str::replace(&field_variable.to_string(), "_", " "));

        initializers
            .push(quote!(korangar_interface::element::StateElement::to_element(self_path.#field_identifier(), #display_name.to_string())));

        initializers_mut.push(
            quote!(korangar_interface::element::StateElement::to_element_mut(self_path.#field_identifier(), #display_name.to_string())),
        );
    }

    (initializers, initializers_mut, is_unnamed, window_title, window_class)
}
