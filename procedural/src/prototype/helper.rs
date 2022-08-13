use proc_macro2::TokenStream;
use syn::{ Attribute, DataStruct, Fields, LitStr };
use quote::quote;

use crate::utils::get_unique_attribute;

pub fn prototype_element_helper(data_struct: DataStruct, mut attributes: Vec<Attribute>) -> (Vec<TokenStream>, TokenStream, Option<TokenStream>) {

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let window_title = get_unique_attribute(&mut attributes, "window_title")
        .map(|attribute| attribute.parse_args().expect("failed to parse window title"))
        .map(|window_title: LitStr| quote!(#window_title))
        .unwrap_or(quote!("#ident"));

    let window_class = get_unique_attribute(&mut attributes, "window_class")
        .map(|attribute| attribute.parse_args().expect("failed to parse window class"))
        .map(|window_class: LitStr| quote!(#window_class));

    let mut initializers = vec![];

    for mut field in named_fields.named {

        if get_unique_attribute(&mut field.attrs, "hidden_element").is_some() {
            continue;
        }

        let field_name = field.ident.unwrap();

        let display_name = get_unique_attribute(&mut field.attrs, "name")
            .map(|attribute| attribute.parse_args().expect(""))
            .map(|name: LitStr| name.value())
            .unwrap_or_else(|| str::replace(&field_name.to_string(), "_", " "));

        initializers.push(quote!(crate::interface::traits::PrototypeElement::to_element(&self.#field_name, #display_name.to_string())));
    }

    (initializers, window_title, window_class)
}
