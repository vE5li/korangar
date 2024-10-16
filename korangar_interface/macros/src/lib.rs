#![feature(extract_if)]
#![feature(box_into_inner)]

mod components;
mod element;
mod helper;
mod utils;
mod window;

use proc_macro::TokenStream as InterfaceTokenStream;
use syn::{Data, DeriveInput, parse};

use self::element::*;
use self::window::*;

#[proc_macro]
pub fn window(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::window(token_stream)
}

#[proc_macro]
pub fn text(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::text(token_stream)
}

#[proc_macro]
pub fn button(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::button(token_stream)
}

#[proc_macro]
pub fn state_button(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::state_button(token_stream)
}

#[proc_macro]
pub fn collapsable(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::collapsable(token_stream)
}

#[proc_macro]
pub fn scroll_view(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::scroll_view(token_stream)
}

#[proc_macro]
pub fn text_box(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    components::text_box(token_stream)
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
