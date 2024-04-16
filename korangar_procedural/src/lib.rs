#![feature(extend_one)]
#![feature(extract_if)]

mod bound;
mod prototype;
mod toggle;
mod utils;

use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse, Data, DeriveInput, ItemFn, LitStr, Stmt};

use self::bound::*;
use self::prototype::*;
use self::toggle::*;

#[proc_macro]
pub fn dimension_bound(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<DimensionBound>(token_stream).unwrap().stream.into()
}

#[proc_macro]
pub fn size_bound(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<SizeBound>(token_stream).unwrap().stream.into()
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
        LitStr::new(function_name.to_string().replace('_', " ").as_str(), function_name.span())
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
