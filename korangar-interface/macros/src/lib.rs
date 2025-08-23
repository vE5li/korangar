#![feature(box_into_inner)]

mod element;
mod helper;
mod utils;
mod window;

use proc_macro::TokenStream as InterfaceTokenStream;
use syn::{Data, DeriveInput, parse};

use self::element::*;
use self::window::*;

#[proc_macro_derive(StateElement, attributes(name, hidden_element))]
pub fn derive_state_element(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_state_element_struct(data_struct, generics, attrs, ident),
        Data::Enum(data_enum) => derive_state_element_enum(data_enum, generics, ident),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

#[proc_macro_derive(StateWindow, attributes(name, hidden_element, window_title, window_class))]
pub fn derive_state_window(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput {
        ident,
        attrs,
        data,
        generics,
        ..
    } = parse(token_stream).expect("failed to parse token stream");

    match data {
        Data::Struct(data_struct) => derive_state_window_struct(data_struct, generics, attrs, ident),
        Data::Enum(..) => panic!("enum types may not be derived"),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}
