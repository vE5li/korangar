use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, DataEnum, DataStruct, Fields, Generics, Ident};

use super::helper::byte_convertable_helper;
use crate::utils::*;

pub fn derive_byte_convertable_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {
    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let (from_bytes_implementations, implemented_fields, to_bytes_implementations) = byte_convertable_helper(named_fields);
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics crate::loaders::ByteConvertable for #name #type_generics #where_clause {

            fn from_bytes(byte_stream: &mut crate::loaders::ByteStream, length_hint: Option<usize>) -> Self {
                assert!(length_hint.is_none(), "structs may not have a length hint");
                #(#from_bytes_implementations)*
                Self { #(#implemented_fields),* }
            }

            // Temporary until serialization is always possible
            #[allow(unreachable_code)]
            fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
                assert!(length_hint.is_none(), "structs may not have a length hint");
                [#(#to_bytes_implementations),*].concat()
            }
        }
    }
    .into()
}

pub fn derive_byte_convertable_enum(
    data_enum: DataEnum,
    generics: Generics,
    mut attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let numeric_type = get_unique_attribute(&mut attributes, "numeric_type")
        .map(|attribute| attribute.parse_args().unwrap())
        .unwrap_or_else(|| Ident::new("u8", Span::call_site()));

    let mut current_index = 0usize;
    let mut indices = Vec::new();
    let mut values = Vec::new();

    for mut variant in data_enum.variants.into_iter() {
        if let Some(attribute) = get_unique_attribute(&mut variant.attrs, "numeric_value") {
            current_index = attribute
                .parse_args::<syn::LitInt>()
                .expect("numeric_value requires an integer value")
                .base10_parse()
                .expect("numeric_value failed to parse integer as base 10");
        }

        indices.push(current_index);
        values.push(variant.ident);
        current_index += 1;
    }

    quote! {
        impl #impl_generics ByteConvertable for #name #type_generics #where_clause {

            fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
                assert!(length_hint.is_none(), "length hint may not be given to enums");
                match #numeric_type::from_bytes(byte_stream, None) as usize {
                    #( #indices => Self::#values, )*
                    invalid => panic!("invalid value {}", invalid),
                }
            }

            // Temporary until serialization is always possible
            #[allow(unreachable_code)]
            fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
                assert!(length_hint.is_none(), "length hint may not be given to enums");
                match self {
                    #( #name::#values => (#indices as #numeric_type).to_bytes(None), )*
                }
            }
        }
    }
    .into()
}
