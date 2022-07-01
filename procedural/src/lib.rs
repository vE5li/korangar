#![feature(let_else)]

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
extern crate quote;

use syn::{ Ident, Data, Fields, LitFloat };
use proc_macro2::TokenStream;
use quote::quote;

struct ButtonArguments {
    name: syn::Lit,
    event: syn::Ident,
}

impl syn::parse::Parse for ButtonArguments {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let name = input.parse().unwrap();
        let _punct: proc_macro2::Punct = input.parse().unwrap();
        let event = input.parse().unwrap();
        Ok(ButtonArguments { name, event })
    }
}

fn parse_common(item: proc_macro::TokenStream) -> (Ident, Vec<TokenStream>, String, Option<String>) {

    let ast: syn::DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

    let mut window_title = ast.ident.to_string();
    let mut window_class = None;
    let mut initializers = vec![];

    let Data::Struct(data_struct) = ast.data else {
        panic!("only structs may be derived");
    };

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    'fields: for field in named_fields.named {

        let field_name = field.ident.unwrap();
        let mut display_name = str::replace(&field_name.to_string(), "_", " ");

        for attribute in field.attrs {
            let attribute_name = attribute.path.segments[0].ident.to_string();

            if &attribute_name == "hidden_element" {
                continue 'fields;
            }

            if &attribute_name == "name" {
                let arguments: syn::Lit = attribute.parse_args().unwrap();
                let syn::Lit::Str(new_name_string) = arguments else {
                    panic!("name must be a literal string");
                };
                display_name = new_name_string.value();
            }

            if &attribute_name == "window_title" {
                let arguments: syn::Lit = attribute.parse_args().unwrap();
                let syn::Lit::Str(new_window_title) = arguments else {
                    panic!("window title must be a literal string");
                };
                window_title = new_window_title.value();
            }

            if &attribute_name == "window_class" {
                let arguments: syn::Lit = attribute.parse_args().unwrap();
                let syn::Lit::Str(new_window_class) = arguments else {
                    panic!("window class must be a literal string");
                };
                window_class = Some(new_window_class.value());
            }

            if &attribute_name == "event_button" {
                let arguments: ButtonArguments = attribute.parse_args().unwrap();
                let name = arguments.name;
                let event = arguments.event;

                let syn::Lit::Str(name) = name else {
                    panic!("expected string literal");
                };

                initializers.push(quote!(
                    std::rc::Rc::new(std::cell::RefCell::new(crate::interface::elements::Button::new(#name, crate::input::UserEvent::#event, false)))
                ));
            }
        }

        initializers.push(quote!(crate::interface::traits::PrototypeElement::to_element(&self.#field_name, #display_name.to_string())));
    }

    (name, initializers, window_title, window_class)
}

#[proc_macro_derive(PrototypeElement, attributes(name, hidden_element, event_button))]
pub fn derive_prototype_element(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let (name, initializers, _window_title, _window_class) = parse_common(item);

    let expanded = quote! {
        impl crate::interface::traits::PrototypeElement for #name {
            fn to_element(&self, display: String) -> crate::interface::types::ElementCell {
                let elements: Vec<crate::interface::types::ElementCell> = vec![#(#initializers),*];
                std::rc::Rc::new(std::cell::RefCell::new(crate::interface::elements::Expandable::new(display, elements, false)))
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

//#[proc_macro_derive(PrototypeMutableElement, attributes(name, hidden_element, event_button))]
//pub fn derive_mutable_prototype_element(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
//
//    let (name, initializers, _window_title, _window_class) = parse_common(item, true);
//
//    let expanded = quote! {
//        impl crate::interface::traits::PrototypeMutableElement for #name {
//            fn to_mutable_element(&self, display: String, change_event: Option<crate::interface::types::ChangeEvent>) -> crate::interface::types::ElementCell {
//                let elements: Vec<crate::interface::types::ElementCell> = vec![#(#initializers),*];
//                std::rc::Rc::new(std::cell::RefCell::new(crate::interface::elements::Expandable::new(display, elements, false)))
//            }
//        }
//    };
//
//    proc_macro::TokenStream::from(expanded)
//}

#[proc_macro_derive(PrototypeWindow, attributes(name, hidden_element, event_button, window_title, window_class))]
pub fn derive_prototype_window(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let (name, initializers, window_title, window_class) = parse_common(item);

    let (window_class_option, window_class_ref_option) = window_class
        .map(|window_class| (quote!(#window_class.to_string().into()), quote!(#window_class.into())))
        .unwrap_or_else(|| (quote!(None), quote!(None)));

    let expanded = quote! {
        impl crate::interface::traits::PrototypeWindow for #name {

            fn window_class(&self) -> Option<&str> {
                #window_class_ref_option
            }

            fn to_window(&self, window_cache: &crate::interface::types::WindowCache, interface_settings: &crate::interface::types::InterfaceSettings, avalible_space: crate::interface::types::Size) -> std::boxed::Box<dyn crate::interface::traits::Window + 'static> {
                let elements: Vec<crate::interface::types::ElementCell> = vec![#(#initializers),*];
                let size_constraint = constraint!(200.0 > 300.0 < 400.0, 100.0 > ? < 80.0%);
                std::boxed::Box::new(crate::interface::windows::FramedWindow::new(window_cache, interface_settings, avalible_space, #window_title.to_string(), #window_class_option, elements, size_constraint))
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

//#[proc_macro_derive(PrototypeMutableWindow, attributes(name, hidden_element, event_button, window_title, window_class))]
//pub fn derive_prototype_mutable_window(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
//
//    let (name, initializers, window_title, window_class) = parse_common(item, true);
//
//    let window_class_option = match window_class {
//        Some(window_class) => quote!(#window_class.to_string().into()),
//        None => quote!(None),
//    };
//
//    let expanded = quote! {
//        impl crate::interface::traits::PrototypeWindow for #name {
//            fn to_window(&self, window_cache: &crate::interface::types::WindowCache, interface_settings: &crate::interface::types::InterfaceSettings, avalible_space: crate::interface::types::Size) -> std::boxed::Box<dyn crate::interface::traits::Window + 'static> {
//                let elements: Vec<crate::interface::types::ElementCell> = vec![#(#initializers),*];
//                let size_constraint = constraint!(200.0 > 300.0 < 400.0, 100.0 > ? < 80.0%);
//                std::boxed::Box::new(crate::interface::windows::FramedWindow::new(window_cache, interface_settings, avalible_space, #window_title.to_string(), #window_class_option, elements, size_constraint))
//            }
//        }
//    };
//
//    proc_macro::TokenStream::from(expanded)
//}

#[proc_macro_derive(ByteConvertable, attributes(version_equals_above, length_hint))]
pub fn derive_byte_convertable(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let ast: syn::DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

    let Data::Struct(data_struct) = ast.data else {
        panic!("only structs may be derived");
    };

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let mut from_bytes_initializers = vec![];
    let mut to_bytes_initializers = vec![];

    for field in named_fields.named {

        let field_name = field.ident.unwrap();
        let mut length_hint = None;

        for attribute in field.attrs {
            let attribute_name = attribute.path.segments[0].ident.to_string();

            if &attribute_name == "length_hint" {
                assert!(length_hint.is_none(), "length hint may only be given once");
                let length: syn::LitInt = attribute.parse_args().unwrap();
                length_hint = length.into();
            }
        }

        let length_hint_option = match length_hint {
            Some(length) => quote!(#length.into()),
            None => quote!(None),
        };

        from_bytes_initializers.push(quote!(#field_name: crate::traits::ByteConvertable::from_bytes(byte_stream, #length_hint_option)));
        to_bytes_initializers.push(quote!(crate::traits::ByteConvertable::to_bytes(&self.#field_name, #length_hint_option).as_slice()));
    }

    let expanded = quote! {
        impl crate::traits::ByteConvertable for #name {

            fn from_bytes(byte_stream: &mut crate::types::ByteStream, length_hint: Option<usize>) -> Self {
                assert!(length_hint.is_none(), "structs may not have a length hint");
                Self { #(#from_bytes_initializers),* }
            }

            fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
                assert!(length_hint.is_none(), "structs may not have a length hint");
                [#(#to_bytes_initializers),*].concat()
            } 
        }
    };

    proc_macro::TokenStream::from(expanded)
}

#[derive(Debug)]
struct Dimension {
    pub stream: proc_macro2::TokenStream,
}

impl syn::parse::Parse for Dimension {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {

        let lookahead = input.lookahead1();

        let expanded = if lookahead.peek(syn::Token![!]) {
            let _punct: proc_macro2::Punct = input.parse().unwrap();
            quote!(crate::interface::types::Dimension::Remaining)
        } else if lookahead.peek(syn::Token![?]) {
            let _punct: proc_macro2::Punct = input.parse().unwrap();
            quote!(crate::interface::types::Dimension::Flexible)
        } else {
            let value: LitFloat = input.parse().expect("constraint expanded number");

            let lookahead = input.lookahead1();
            if lookahead.peek(syn::Token![%]) {
                let _punct: proc_macro2::Punct = input.parse().unwrap();
                quote!(crate::interface::types::Dimension::Relative(#value))
            } else {
                quote!(crate::interface::types::Dimension::Absolute(#value))
            }
        };

        Ok(Dimension { stream: proc_macro2::TokenStream::from(expanded) })
    }
}

struct SizeConstraint {
    pub stream: proc_macro2::TokenStream,
}

impl syn::parse::Parse for SizeConstraint {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {

        let first_dimension: Dimension = input.parse().unwrap();

        let (width, minimum_width) = if input.lookahead1().peek(syn::Token![>]) {
            let _punct: proc_macro2::Punct = input.parse().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();

            let minimum_width = first_dimension.stream;
            let width = second_dimension.stream;

            let minimum_width = quote!(Some(#minimum_width));
            (width, minimum_width)
        } else {
            let minimum_width = quote!(None);
            let width = first_dimension.stream;
            (width, minimum_width)
        };

        let maximum_width = if input.lookahead1().peek(syn::Token![<]) {
            let _punct: proc_macro2::Punct = input.parse().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();
            let maximum_width = second_dimension.stream;
            quote!(Some(#maximum_width))
        } else {
            quote!(None)
        };

        assert!(input.lookahead1().peek(syn::Token![,]), "constraint expected comma after first dimension");
        let _punct: proc_macro2::Punct = input.parse().unwrap();

        let first_dimension: Dimension = input.parse().unwrap();

        let (height, minimum_height) = if input.lookahead1().peek(syn::Token![>]) {
            let _punct: proc_macro2::Punct = input.parse().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();

            let minimum_height = first_dimension.stream;
            let height = second_dimension.stream;

            let minimum_height = quote!(Some(#minimum_height));
            (height, minimum_height)
        } else {
            let minimum_height = quote!(None);
            let height = first_dimension.stream;
            (height, minimum_height)
        };

        let maximum_height = if input.lookahead1().peek(syn::Token![<]) {
            let _punct: proc_macro2::Punct = input.parse().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();
            let maximum_height = second_dimension.stream;
            quote!(Some(#maximum_height))
        } else {
            quote!(None)
        };

        let expanded = quote! {
            crate::interface::types::SizeConstraint {
                width: #width,
                minimum_width: #minimum_width,
                maximum_width: #maximum_width,
                height: #height,
                minimum_height: #minimum_height,
                maximum_height: #maximum_height,
            }
        };

        Ok(SizeConstraint { stream: proc_macro2::TokenStream::from(expanded) })
    }
}

#[proc_macro]
pub fn constraint(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let size_constraint: SizeConstraint = syn::parse(item).unwrap();
    size_constraint.stream.into()
}








struct PacketSignature {
    first: syn::LitInt,
    second: syn::LitInt,
}

impl syn::parse::Parse for PacketSignature {

    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let first = input.parse().unwrap();
        let _punct: proc_macro2::Punct = input.parse().unwrap();
        let second = input.parse().unwrap();
        Ok(PacketSignature { first, second })
    }
}

#[proc_macro_derive(Packet, attributes(header, length_hint))]
pub fn derive_packet(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let ast: syn::DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

    let mut from_bytes_initializers = vec![];
    let mut to_bytes_initializers = vec![];
    let mut has_fields = false;

    let Data::Struct(data_struct) = ast.data else {
        panic!("only structs may be derived");
    };

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let mut packet_signature = None;
    for attribute in ast.attrs {

        if attribute.path.segments[0].ident.to_string().as_str() != "header" {
            continue;
        }

        assert!(packet_signature.is_none(), "packet signature may only be specified once");
        packet_signature = attribute.parse_args::<PacketSignature>().unwrap().into();
    }

    let packet_signature = packet_signature.expect("packet needs to specify a signature");
    let (first, second) = (packet_signature.first, packet_signature.second);

    for field in named_fields.named {
        let field_name = field.ident.unwrap();
        let mut length_hint = None;

        for attribute in field.attrs {
            let attribute_name = attribute.path.segments[0].ident.to_string();

            if &attribute_name == "length_hint" {
                assert!(length_hint.is_none(), "length hint may only be given once");
                let length: syn::LitInt = attribute.parse_args().unwrap();
                length_hint = length.into();
            }
        }

        let length_hint_option = match length_hint {
            Some(length) => quote!(#length.into()),
            None => quote!(None),
        };

        from_bytes_initializers.push(quote!(#field_name: crate::traits::ByteConvertable::from_bytes(byte_stream, #length_hint_option)));
        to_bytes_initializers.push(quote!(crate::traits::ByteConvertable::to_bytes(&self.#field_name, #length_hint_option).as_slice()));

        has_fields = true;
    }

    let to_bytes = match has_fields {
        true => quote!([&[#first, #second], #(#to_bytes_initializers),*].concat()),
        false => quote!(vec![#first, #second]),
    };

    let expanded = quote! {
        impl crate::network::Packet for #name {

            fn header() -> [u8; 2] {
                [#first, #second]
            }
    
            fn to_bytes(&self) -> Vec<u8> {
                #to_bytes
            }
        }

        impl #name {

            fn try_from_bytes(byte_stream: &mut crate::types::ByteStream) -> Result<Self, String> {
                let result = match byte_stream.match_signature(Self::header()) {
                    true => Ok(Self { #(#from_bytes_initializers),* }),
                    false => Err(format!("invalid signature 0x{:02x} 0x{:02x}", byte_stream.peek(0), byte_stream.peek(1))),
                };

                #[cfg(feature = "debug_network")]
                if let Ok(packet) = &result {
                    print_debug!("{}incoming packet{}: {:?}", YELLOW, NONE, packet);
                }
 
                result
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}
