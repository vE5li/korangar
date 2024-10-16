use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataStruct, Generics, Ident};

use super::helper::byte_convertable_helper;
use crate::utils::{PacketSignature, get_unique_attribute};

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
        let __packet_length = ragnarok_bytes::ConversionResultExt::trace::<Self>(u16::from_bytes(byte_reader))?;
    });

    let final_to_bytes = match is_variable_length {
        _ if to_bytes_implementations.is_empty() => quote! {
            Ok(0)
        },
        true => {
            quote! {
                let start_position = byte_writer.len();
                let dummy_packet_length = 0u16;

                let written = byte_writer.write_counted(|writer| {
                    dummy_packet_length.to_bytes(writer)?;
                    #(#to_bytes_implementations)*
                    Ok(())
                })?;

                // We add 2 for the header bytes
                let packet_length = (written + 2) as u16;
                byte_writer.overwrite_at(start_position, packet_length.to_le_bytes())?;

                Ok(written)
            }
        }
        false => quote! {
            byte_writer.write_counted(|writer| {
                #(#to_bytes_implementations)*
                Ok(())
            })
        },
    };

    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #new_implementation
        }

        impl #impl_generics ragnarok_packets::Packet for #name #type_generics #where_clause {
            const IS_PING: bool = #is_ping;
            const HEADER: ragnarok_packets::PacketHeader = ragnarok_packets::PacketHeader(#signature);

            fn payload_from_bytes<Meta>(byte_reader: &mut ragnarok_bytes::ByteReader<Meta>) -> ragnarok_bytes::ConversionResult<Self> {
                let base_offset = byte_reader.get_offset();
                #insert_packet_length
                #(#from_bytes_implementations)*
                let packet = #instanciate;

                Ok(packet)
            }

            fn payload_to_bytes(&self, byte_writer: &mut ragnarok_bytes::ByteWriter) -> ragnarok_bytes::ConversionResult<usize> {
                #final_to_bytes
            }

            #[cfg(feature = "packet-to-state-element")]
            fn to_element<App: korangar_interface::application::Application>(
                self_path: impl rust_state::Path<App, Self>,
                name: String,
            ) -> Box<dyn korangar_interface::element::Element<App, LayoutInfo = ()>> {
                korangar_interface::element::ErasedElement::new(<Self as korangar_interface::element::StateElement<App>>::to_element(self_path, name))
            }
        }
    }
    .into()
}
