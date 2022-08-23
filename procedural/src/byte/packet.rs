use proc_macro::TokenStream as InterfaceTokenStream;
use syn::{ Ident, Fields, DataStruct, Attribute, Generics };
use quote::quote;

use super::helper::byte_convertable_helper;
use crate::utils::*;

pub fn derive_packet_struct(data_struct: DataStruct, generics: Generics, mut attributes: Vec<Attribute>, name: Ident) -> InterfaceTokenStream {

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let packet_signature = get_unique_attribute(&mut attributes, "header")
        .map(|attribute| attribute.parse_args::<PacketSignature>())
        .expect("packet needs to specify a signature")
        .expect("failed to parse packet header");

    let (first, second) = (packet_signature.first, packet_signature.second);
    let (from_bytes_implementations, implemented_fields, to_bytes_implementations) = byte_convertable_helper(named_fields);
    let to_bytes = quote!([&[#first, #second][..], #(#to_bytes_implementations),*].concat());

    quote! {

        impl #impl_generics crate::network::Packet for #name #type_generics #where_clause {

            fn header() -> [u8; 2] {
                [#first, #second]
            }

            fn to_bytes(&self) -> Vec<u8> {
                #to_bytes
            }
        }

        impl #impl_generics #name #type_generics #where_clause {

            fn try_from_bytes(byte_stream: &mut crate::loaders::ByteStream) -> Result<Self, String> {
                let result = match byte_stream.match_signature(Self::header()) {
                    true => {
                        #(#from_bytes_implementations)*
                        Ok(Self { #(#implemented_fields),* })
                    },
                    false => Err(format!("invalid signature 0x{:02x} 0x{:02x}", byte_stream.peek(0), byte_stream.peek(1))),
                };

                #[cfg(feature = "debug_network")]
                if let Ok(packet) = &result {
                    print_debug!("{}incoming packet{}: {:?}", YELLOW, NONE, packet);
                }

                result
            }
        }
    }.into()
}
