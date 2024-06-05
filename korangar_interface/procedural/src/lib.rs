#![feature(extract_if)]

mod bound;
mod element;
mod helper;
mod utils;
mod window;

use bound::{DimensionBound, SizeBound};
use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{parse, Data, DeriveInput};

use self::element::*;
use self::window::*;

#[proc_macro]
pub fn dimension_bound(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<DimensionBound>(token_stream).unwrap().stream.into()
}

#[proc_macro]
pub fn size_bound(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    parse::<SizeBound>(token_stream).unwrap().stream.into()
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

#[proc_macro_derive(LinkBackDefault)]
pub fn derive_link_back_default(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ElLinkBack<App> for #ident #type_generics
            #where_clause Self: DeriveInputGetState<App>,
        {
            fn link_back(&mut self, weak_self: WeakElementCell<App>, weak_parent: Option<WeakElementCell<App>>) {
                <<Self as DeriveInputGetState<App>>::Type as StateType<App>>::link_back(self.get_state_mut(), weak_self, weak_parent)
            }
        }
    }
    .into()
}

#[proc_macro_derive(FucusableDefault)]
pub fn derive_focusable_default(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ElFocusable<App> for #ident #type_generics
            #where_clause Self: DeriveInputGetState<App> + DeriveInputFocusable<App>,
        {
            fn is_focusable(&self) -> bool {
                <<Self as DeriveInputGetState<App>>::Type as StateType<App>>::is_focusable(self.get_state(), self.is_self_focusable())
            }
        }
    }
    .into()
}

#[proc_macro_derive(FucusNextDefault)]
pub fn derive_focus_next_default(token_stream: InterfaceTokenStream) -> InterfaceTokenStream {
    let DeriveInput { ident, generics, .. } = parse(token_stream).expect("failed to parse token stream");

    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ElFocusNext<App> for #ident #type_generics
            #where_clause Self: DeriveInputGetState<App> + DeriveInputFocusable<App>,
        {
            fn focus_next(&self, self_cell: ElementCell<App>, caller_cell: Option<ElementCell<App>>, focus: Focus) -> Option<ElementCell<App>> {
                <<Self as DeriveInputGetState<App>>::Type as StateType<App>>::focus_next(
                    self.get_state(),
                    self_cell,
                    caller_cell,
                    focus,
                    self.is_self_focusable(),
                )
            }
        }
    }.into()
}
