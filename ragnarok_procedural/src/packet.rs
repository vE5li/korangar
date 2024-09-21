use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataStruct, Generics, Ident};

use super::helper::byte_convertable_helper;
use crate::utils::{get_unique_attribute, PacketSignature};

pub fn derive_packet_struct(
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
    let is_variable_length = get_unique_attribute(&mut attributes, "variable_length").is_some();

    let signature = packet_signature.signature;
    let (new_implementation, from_bytes_implementations, implemented_fields, to_bytes_implementations, delimiter) =
        byte_convertable_helper(data_struct);

    let instanciate = match delimiter {
        proc_macro2::Delimiter::Brace => quote!(Self { #(#implemented_fields),* }),
        proc_macro2::Delimiter::Parenthesis => quote!(Self ( #(#implemented_fields),* )),
        _ => panic!(),
    };

    let insert_packet_length = is_variable_length.then_some(quote! {
        let __packet_length = ragnarok_bytes::ConversionResultExt::trace::<Self>(u16::from_bytes(byte_stream))?;
    });

    let final_to_bytes = match is_variable_length {
        _ if to_bytes_implementations.is_empty() => quote! {
            Ok(Vec::new())
        },
        true => {
            quote! {
                let following_bytes = [#(#to_bytes_implementations),*].concat();
                let packet_length = following_bytes.len() as u16 + 4;

                let mut final_bytes = packet_length.to_bytes()?;
                final_bytes.extend(following_bytes);

                Ok(final_bytes)
            }
        }
        false => quote! {
            Ok([#(#to_bytes_implementations),*].concat())
        },
    };

    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #new_implementation
        }

        impl #impl_generics ragnarok_packets::Packet for #name #type_generics #where_clause {
            const IS_PING: bool = #is_ping;
            const HEADER: ragnarok_packets::PacketHeader = ragnarok_packets::PacketHeader(#signature);

            fn payload_from_bytes<Meta>(byte_stream: &mut ragnarok_bytes::ByteStream<Meta>) -> ragnarok_bytes::ConversionResult<Self> {
                let base_offset = byte_stream.get_offset();
                #insert_packet_length
                #(#from_bytes_implementations)*
                let packet = #instanciate;

                Ok(packet)
            }

            fn payload_to_bytes(&self) -> ragnarok_bytes::ConversionResult<Vec<u8>> {
                #final_to_bytes
            }

            #[cfg(feature = "packet-to-prototype-element")]
            fn to_prototype_element<App: korangar_interface::application::Application>(
                &self,
            ) -> Box<dyn korangar_interface::elements::PrototypeElement<App> + Send> {
                Box::new(self.clone())
            }
        }
    }
    .into()
}
