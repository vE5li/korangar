#![feature(extend_one)]
#![feature(extract_if)]

mod byte;
mod constraint;
mod prototype;
mod toggle;
mod utils;

use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse, Data, DeriveInput, ItemFn, LitStr, Stmt};

use self::byte::*;
use self::constraint::*;
use self::prototype::*;
use self::toggle::*;

#[proc_macro]
pub fn dimension(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<DimensionConstraint>(token_stream).unwrap().stream.into()
}

#[proc_macro]
pub fn constraint(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<SizeConstraint>(token_stream).unwrap().stream.into()
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
    }
    .into()
}

#[proc_macro_attribute]
pub fn profile(name: InterfaceTokenStream, function: InterfaceTokenStream) -> InterfaceTokenStream {
    let mut function: ItemFn = parse(function).expect("failed to parse token stream");
    let name: LitStr = parse(name).unwrap_or_else(|_| {
        let function_name = &function.sig.ident;
        LitStr::new(function_name.to_string().replace("_", " ").as_str(), function_name.span())
    });

    let code = quote! {
        #[cfg(feature = "debug")]
        let _measurement = crate::debug::start_measurement(#name);
    }
    .into();

    let statement: Stmt = parse(code).expect("failed to parse token stream");
    function.block.stmts.insert(0, statement);

    quote! { #function }.into()
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

#[proc_macro_derive(FixedByteSize)]
pub fn derive_fixed_byte_size(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, data, .. } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_fixed_byte_size_struct(data_struct, generics, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(Named)]
pub fn derive_named(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics crate::loaders::Named for #ident #type_generics #where_clause {
            const NAME: &'static str = stringify!(#ident);
        }
    }
    .into()
}

#[proc_macro_derive(
    ByteConvertable,
    attributes(
        packet_length,
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
        packet_length,
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
        packet_length,
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

#[proc_macro_derive(PrototypeElement, attributes(name, hidden_element))]
pub fn derive_prototype_element(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_prototype_element_struct(data_struct, generics, attrs, ident),
        Data::Enum(data_enum) => derive_prototype_element_enum(data_enum, generics, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(PrototypeWindow, attributes(name, hidden_element, window_title, window_class))]
pub fn derive_prototype_window(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_prototype_window_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}
