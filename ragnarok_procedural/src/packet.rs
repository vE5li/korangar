use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataStruct, Generics, Ident};

use super::helper::byte_convertable_helper;
use crate::utils::*;

pub fn derive_incoming_packet_struct(
    data_struct: DataStruct,
    generics: Generics,
    mut attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let packet_signature = get_unique_attribute(&mut attributes, "header")
        .map(|attribute| attribute.parse_args::<PacketSignature>())
        .expect("packet needs to specify a signature")
        .expect("failed to parse packet header");
    let is_ping = get_unique_attribute(&mut attributes, "ping").is_some();

    let signature = packet_signature.signature;
    let (from_bytes_implementations, implemented_fields, _to_bytes_implementations, delimiter) = byte_convertable_helper(data_struct);

    let instanciate = match delimiter {
        proc_macro2::Delimiter::Brace => quote!(Self { #(#implemented_fields),* }),
        proc_macro2::Delimiter::Parenthesis => quote!(Self ( #(#implemented_fields),* )),
        _ => panic!(),
    };

    quote! {
        impl #impl_generics ragnarok_networking::IncomingPacket for #name #type_generics #where_clause {
            const IS_PING: bool = #is_ping;
            const HEADER: u16 = #signature;

            fn payload_from_bytes<Meta>(byte_stream: &mut ragnarok_bytes::ByteStream<Meta>) -> ragnarok_bytes::ConversionResult<Self> {
                let base_offset = byte_stream.get_offset();
                #(#from_bytes_implementations)*
                let packet = #instanciate;

                Ok(packet)
            }
        }
    }
    .into()
}

pub fn derive_outgoing_packet_struct(
    data_struct: DataStruct,
    generics: Generics,
    mut attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let packet_signature = get_unique_attribute(&mut attributes, "header")
        .map(|attribute| attribute.parse_args::<PacketSignature>())
        .expect("packet needs to specify a signature")
        .expect("failed to parse packet header");
    let is_ping = get_unique_attribute(&mut attributes, "ping").is_some();

    let signature = packet_signature.signature;
    let (_from_bytes_implementations, _implemented_fields, to_bytes_implementations, _delimiter) = byte_convertable_helper(data_struct);
    let to_bytes = quote!([&#signature.to_le_bytes()[..], #(#to_bytes_implementations),*].concat());

    quote! {
        impl #impl_generics ragnarok_networking::OutgoingPacket for #name #type_generics #where_clause {
            const IS_PING: bool = #is_ping;

            // Temporary until serialization is always possible
            #[allow(unreachable_code)]
            fn packet_to_bytes(&self) -> ragnarok_bytes::ConversionResult<Vec<u8>> {
                Ok(#to_bytes)
            }
        }
    }
    .into()
}
