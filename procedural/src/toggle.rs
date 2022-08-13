use proc_macro::TokenStream as InterfaceTokenStream;
use syn::{ Ident, Fields, DataStruct, Generics };
use quote::quote;

use crate::utils::get_unique_attribute;

pub fn derive_toggle_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let toggle_fields: Vec<Ident> = named_fields.named
        .into_iter()
        .filter_map(|mut field| get_unique_attribute(&mut field.attrs, "toggle").map(|_| field.ident.unwrap()))
        .collect();

    let function_names: Vec<Ident> = toggle_fields
        .iter()
        .map(|field| Ident::new(&format!("toggle_{}", field), field.span()))
        .collect();

    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #( pub fn #function_names(&mut self) {
                self.#toggle_fields = !self.#toggle_fields;
            })*
        }
    }.into()
}
