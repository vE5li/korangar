#![feature(extend_one)]
#![feature(extract_if)]

mod convertable;
mod fixed_size;
mod helper;
mod packet;
mod utils;

use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{parse, Data, DeriveInput};

use self::convertable::*;
use self::fixed_size::{derive_fixed_byte_size_enum, derive_fixed_byte_size_struct};
use self::packet::*;

#[proc_macro_derive(FixedByteSize)]
pub fn derive_fixed_byte_size(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_fixed_byte_size_struct(data_struct, generics, ident),
        Data::Enum(..) => derive_fixed_byte_size_enum(generics, attrs, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(
    ByteConvertable,
    attributes(
        length_hint,
        repeating,
        repeating_remaining,
        numeric_type,
        numeric_value,
        version,
        version_smaller,
        version_equals_or_above
    )
)]
pub fn derive_byte_convertable(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_byte_convertable_struct(data_struct, generics, ident),
        Data::Enum(data_enum) => derive_byte_convertable_enum(data_enum, generics, attrs, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(
    FromBytes,
    attributes(
        length_hint,
        repeating,
        repeating_remaining,
        numeric_type,
        numeric_value,
        version,
        version_smaller,
        version_equals_or_above
    )
)]
pub fn derive_from_bytes(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_from_bytes_struct(data_struct, generics, ident),
        Data::Enum(data_enum) => derive_from_bytes_enum(data_enum, generics, attrs, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(
    ToBytes,
    attributes(
        length_hint,
        repeating,
        repeating_remaining,
        numeric_type,
        numeric_value,
        version,
        version_smaller,
        version_equals_or_above
    )
)]
pub fn derive_to_bytes(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_to_bytes_struct(data_struct, generics, ident),
        Data::Enum(data_enum) => derive_to_bytes_enum(data_enum, generics, attrs, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(
    IncomingPacket,
    attributes(packet_length, header, ping, length_hint, repeating, repeating_remaining)
)]
pub fn derive_incoming_packet(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_incoming_packet_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(
    OutgoingPacket,
    attributes(packet_length, header, ping, length_hint, repeating, repeating_remaining)
)]
pub fn derive_packet(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        attrs,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_outgoing_packet_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(LoginServer)]
pub fn derive_login_server_packet(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ragnarok_packets::LoginServerPacket for #ident #type_generics #where_clause {}
    }
    .into()
}

#[proc_macro_derive(CharacterServer)]
pub fn derive_character_server_packet(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ragnarok_packets::CharacterServerPacket for #ident #type_generics #where_clause {}
    }
    .into()
}

#[proc_macro_derive(MapServer)]
pub fn derive_map_server_packet(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ragnarok_packets::MapServerPacket for #ident #type_generics #where_clause {}
    }
    .into()
}
