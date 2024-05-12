use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, DataEnum, DataStruct, Generics, Ident};

use crate::helper::byte_convertable_helper;
use crate::utils::*;

fn derive_for_struct(
    data_struct: DataStruct,
    generics: Generics,
    name: Ident,
    implement_from: bool,
    implement_to: bool,
) -> InterfaceTokenStream {
    let (new_implementation, from_bytes_implementations, implemented_fields, to_bytes_implementations, delimiter) =
        byte_convertable_helper(data_struct);
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let instanciate = match delimiter {
        proc_macro2::Delimiter::Brace => quote!(Self { #(#implemented_fields),* }),
        proc_macro2::Delimiter::Parenthesis => quote!(Self ( #(#implemented_fields),* )),
        _ => panic!(),
    };

    let new = implement_to.then(|| {
        quote! {
            impl #impl_generics #name #type_generics #where_clause {
                #new_implementation
            }
        }
    });

    let from = implement_from.then(|| {
        quote! {
            impl #impl_generics ragnarok_bytes::FromBytes for #name #type_generics #where_clause {
                fn from_bytes<Meta>(byte_stream: &mut ragnarok_bytes::ByteStream<Meta>) -> ragnarok_bytes::ConversionResult<Self> {
                    let base_offset = byte_stream.get_offset();
                    #(#from_bytes_implementations)*
                    Ok(#instanciate)
                }
            }
        }
    });

    let to = implement_to.then(|| {
        quote! {
            impl #impl_generics ragnarok_bytes::ToBytes for #name #type_generics #where_clause {
                // Temporary until serialization is always possible
                #[allow(unreachable_code)]
                fn to_bytes(&self) -> ragnarok_bytes::ConversionResult<Vec<u8>> {
                    Ok([#(#to_bytes_implementations),*].concat())
                }
            }
        }
    });

    quote! {
        #new
        #from
        #to
    }
    .into()
}

fn derive_for_enum(
    data_enum: DataEnum,
    generics: Generics,
    mut attributes: Vec<Attribute>,
    name: Ident,
    add_from: bool,
    add_to: bool,
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

    let from = add_from.then(|| {
        quote! {
            impl #impl_generics ragnarok_bytes::FromBytes for #name #type_generics #where_clause {
                fn from_bytes<Meta>(byte_stream: &mut ragnarok_bytes::ByteStream<Meta>) -> ragnarok_bytes::ConversionResult<Self> {
                    match ragnarok_bytes::ConversionResultExt::trace::<Self>(#numeric_type::from_bytes(byte_stream))? as usize {
                        #( #indices => Ok(Self::#values), )*
                        invalid => Err(ragnarok_bytes::ConversionError::from_message(format!("invalid enum variant {}", invalid))),
                    }
                }
            }
        }
    });

    let to = add_to.then(|| {
        quote! {
            impl #impl_generics ragnarok_bytes::ToBytes for #name #type_generics #where_clause {
                // Temporary until serialization is always possible
                #[allow(unreachable_code)]
                fn to_bytes(&self) -> ragnarok_bytes::ConversionResult<Vec<u8>> {
                    match self {
                        #( #name::#values => ragnarok_bytes::ConversionResultExt::trace::<Self>((#indices as #numeric_type).to_bytes()), )*
                    }
                }
            }
        }
    });

    quote! {
        #from
        #to
    }
    .into()
}

pub fn derive_byte_convertable_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {
    derive_for_struct(data_struct, generics, name, true, true)
}

pub fn derive_byte_convertable_enum(
    data_enum: DataEnum,
    generics: Generics,
    attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    derive_for_enum(data_enum, generics, attributes, name, true, true)
}

pub fn derive_from_bytes_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {
    derive_for_struct(data_struct, generics, name, true, false)
}

pub fn derive_from_bytes_enum(data_enum: DataEnum, generics: Generics, attributes: Vec<Attribute>, name: Ident) -> InterfaceTokenStream {
    derive_for_enum(data_enum, generics, attributes, name, true, false)
}

pub fn derive_to_bytes_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {
    derive_for_struct(data_struct, generics, name, false, true)
}

pub fn derive_to_bytes_enum(data_enum: DataEnum, generics: Generics, attributes: Vec<Attribute>, name: Ident) -> InterfaceTokenStream {
    derive_for_enum(data_enum, generics, attributes, name, false, true)
}
