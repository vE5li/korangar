#![feature(let_else)]
#![feature(extend_one)]
#![feature(drain_filter)]

mod utils;
mod constraint;
mod toggle;
mod byte;
mod prototype;

use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::TokenStream;
use syn::{ Data, DeriveInput, parse };
use quote::quote;

use self::constraint::*;
use self::toggle::*;
use self::byte::*;
use self::prototype::*;

#[proc_macro]
pub fn constraint(item: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<SizeConstraint>(item).unwrap().stream.into()
}

#[proc_macro_attribute]
pub fn debug_condition(condition: InterfaceTokenStream, conditional: InterfaceTokenStream) -> InterfaceTokenStream {

    let condition = TokenStream::from(condition);
    let conditional = TokenStream::from(conditional);

    quote! {

        #[cfg(feature = "debug")]
        let execute = #condition;
        #[cfg(not(feature = "debug"))]
        let execute = true;

        if execute {
            #conditional
        }
    }.into()
}

#[proc_macro_derive(toggle, attributes(toggle))]
pub fn derive_toggle(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {

    let DeriveInput { ident, generics, data, .. } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_toggle_struct(data_struct, generics, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(ByteConvertable, attributes(length_hint, repeating, numeric_type, numeric_value, version, version_smaller, version_equals_or_above))]
pub fn derive_byte_convertable(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {

    let DeriveInput { ident, attrs, data, generics, .. } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_byte_convertable_struct(data_struct, generics, ident),
        Data::Enum(data_enum) => derive_byte_convertable_enum(data_enum, generics, attrs, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

/// Derive the Packet trait. A packet header must be specified and all fields must implement ByteConvertable.
#[proc_macro_derive(Packet, attributes(header, length_hint, repeating))]
pub fn derive_packet(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {

    let DeriveInput { ident, generics, data, attrs, .. } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_packet_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(PrototypeElement, attributes(name, hidden_element, event_button))]
pub fn derive_prototype_element(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {

    let DeriveInput { ident, attrs, data, generics, .. } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_prototype_element_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(PrototypeWindow, attributes(name, hidden_element, event_button, window_title, window_class))]
pub fn derive_prototype_window(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {

    let DeriveInput { ident, attrs, data, generics, .. } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_prototype_window_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}
