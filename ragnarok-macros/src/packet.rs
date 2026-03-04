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

    let mut header_attributes = Vec::new();
    attributes.retain(|attr| {
        if attr.path().segments[0].ident == "header" {
            header_attributes.push(attr.clone());
            false
        } else {
            true
        }
    });

    if header_attributes.is_empty() {
        panic!("packet needs to specify at least one header");
    }

    let packet_signatures: Vec<PacketSignature> = header_attributes
        .into_iter()
        .map(|attr| attr.parse_args::<PacketSignature>().expect("failed to parse packet header"))
        .collect();

    let is_ping = get_unique_attribute(&mut attributes, "ping").is_some();
    let is_variable_length = get_unique_attribute(&mut attributes, "variable_length").is_some();

    let default_signature = packet_signatures[0].signature;

    let is_versioned = packet_signatures.len() > 1;

    // If there are multiple headers, all must have versions
    if is_versioned {
        for sig in &packet_signatures {
            if sig.version.is_none() {
                panic!("When multiple #[header(...)] attributes are used, all headers must specify a version");
            }
        }
        let has_header_field = match &data_struct.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .any(|f| f.ident.as_ref().map(|ident| ident == "header").unwrap_or(false)),
            _ => false,
        };

        if !has_header_field {
            panic!(
                "Versioned packets (with multiple #[header(...)] attributes) must have a `header: PacketHeader` field with \
                 #[hidden_element]"
            );
        }
    }

    let versions: Vec<(&str, u16)> = if is_versioned {
        packet_signatures
            .iter()
            .map(|sig| (sig.version.as_ref().unwrap().as_str(), sig.signature))
            .collect()
    } else {
        vec![("default", packet_signatures[0].signature)]
    };

    let (new_implementations, from_bytes_implementations, implemented_fields, to_bytes_implementations, delimiter) =
        byte_convertable_helper(data_struct, versions);

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

    let packet_header_override = if is_versioned {
        quote! {
            fn packet_header(&self) -> ragnarok_packets::PacketHeader {
                self.header
            }
        }
    } else {
        quote! {}
    };

    quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#new_implementations)*
        }

        impl #impl_generics ragnarok_packets::Packet for #name #type_generics #where_clause {
            const IS_PING: bool = #is_ping;
            const HEADER: ragnarok_packets::PacketHeader = ragnarok_packets::PacketHeader(#default_signature);

            #packet_header_override

            fn payload_from_bytes(byte_reader: &mut ragnarok_bytes::ByteReader) -> ragnarok_bytes::ConversionResult<Self> {
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
