#![feature(let_else)]
#![feature(extend_one)]

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
extern crate quote;

use syn::{ Ident, Data, Fields, LitFloat, DeriveInput, DataStruct, DataEnum, Attribute, LitInt, FieldsNamed };
use proc_macro2::{TokenStream, TokenTree};
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

    let ast: DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

    let Data::Struct(data_struct) = ast.data else {
        panic!("only structs may be derived");
    };

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let mut window_title = ast.ident.to_string();
    let mut window_class = None;
    let mut initializers = vec![];

    for attribute in ast.attrs {
        let attribute_name = attribute.path.segments[0].ident.to_string();

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
    }

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

#[proc_macro_derive(ByteConvertable, attributes(length_hint, repeating, base_type, variant_value, version, version_smaller, version_equals_or_above))]
pub fn derive_byte_convertable(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let ast: DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let attributes = ast.attrs;
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

    match ast.data {
        Data::Struct(data_struct) => derive_byte_convertable_struct(data_struct, name),
        Data::Enum(data_enum) => derive_byte_convertable_enum(data_enum, attributes, name),
        Data::Union(..) => panic!("union types may not be derived"),
    }
}

fn derive_byte_convertable_struct(data_struct: DataStruct, name: Ident) -> proc_macro::TokenStream {

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let (from_bytes_initializers, from_fields, to_bytes_initializers, _has_fields) = byte_convertable_helper(named_fields);

    let expanded = quote! {
        impl crate::traits::ByteConvertable for #name {

            fn from_bytes(byte_stream: &mut crate::types::ByteStream, length_hint: Option<usize>) -> Self {
                assert!(length_hint.is_none(), "structs may not have a length hint");
                #(#from_bytes_initializers)*
                Self { #(#from_fields),* }
            }

            fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
                assert!(length_hint.is_none(), "structs may not have a length hint");
                [#(#to_bytes_initializers),*].concat()
            } 
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn derive_byte_convertable_enum(data_enum: DataEnum, attributes: Vec<Attribute>, name: Ident) -> proc_macro::TokenStream {

    let mut base_type = Ident::new("u8", name.span()); // get a correct span
    let mut base_type_set = false;
    //let mut length_hint = None;

    for attribute in attributes {

        let identifier = attribute.path.segments[0].ident.to_string();

        if identifier.as_str() == "base_type" {
            assert!(!base_type_set, "base type may only be set once per enum");
            base_type = attribute.parse_args::<Ident>().unwrap();
            base_type_set = true;
        }

        //if identifier.as_str() == "length_hint" {
        //    assert!(length_hint.is_none(), "length hint may only be set once per enum");
        //    let length = attribute.parse_args::<LitInt>().unwrap();

        //    length_hint = Some(quote!({
        //        let data = byte_stream.slice(#length);
        //        let mut byte_stream = ByteStream::new(&data);
        //    }));
        //}
    }

    //let length_hint = length_hint.unwrap_or_default();

    let mut current_index = 0usize;
    let mut indices = Vec::new();
    let mut values = Vec::new();

    for variant in data_enum.variants.into_iter() {
        let mut index_set = false;

        for attribute in variant.attrs {
            let attribute_name = attribute.path.segments[0].ident.to_string();

            if &attribute_name == "variant_value" {
                assert!(!index_set, "variant value may only be set once per variant");
                current_index = attribute.parse_args::<syn::LitInt>().unwrap().base10_parse().unwrap();
                index_set = true;
            }
        }

        indices.push(current_index);
        values.push(variant.ident);
        current_index += 1;
    }

    let expanded = quote! {

        impl ByteConvertable for #name {

            fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
                assert!(length_hint.is_none(), "length hint may not be given to enums");

                //#length_hint

                match #base_type::from_bytes(byte_stream, None) as usize {
                    #( #indices => Self::#values, )*
                    invalid => panic!("invalid value {}", invalid),
                }
            }

            fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
                assert!(length_hint.is_none(), "length hint may not be given to enums");
                match self {
                    #( #name::#values => (#indices as #base_type).to_bytes(None), )*
                }
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






fn remove_self_from_stream(token_stream: TokenStream) -> TokenStream {
    let mut new_stream = TokenStream::new();
    let mut iterator = token_stream.into_iter();

    while let Some(token) = iterator.next() {

        if let TokenTree::Group(group) = &token {
            let delimiter = group.delimiter();
            let new_group_stream = remove_self_from_stream(group.stream());
            let new_group = proc_macro2::Group::new(delimiter, new_group_stream);
            new_stream.extend_one(TokenTree::Group(new_group));
            continue;
        }

        if let TokenTree::Ident(ident) = &token {
            if ident == &Ident::new("self", token.span()) { // get proper span ?
                let _burn_punktuation = iterator.next().unwrap();
                continue;
            }
        }

        new_stream.extend_one(token);
    }

    new_stream
}

#[derive(Clone)]
enum VersionRestriction {
    Smaller(PacketSignature),
    BiggerOrEquals(PacketSignature),
}

fn byte_convertable_helper(named_fields: FieldsNamed) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>, bool) {

    let mut from_bytes_initializers = vec![];
    let mut from_fields = vec![];
    let mut to_bytes_initializers = vec![];
    let mut has_fields = false;

    for field in named_fields.named {

        let field_name = field.ident.unwrap();
        let mut length_hint = None;
        let mut repeating = None;
        let mut version = false;
        let mut version_restriction = None;

        for attribute in field.attrs {
            let attribute_name = attribute.path.segments[0].ident.to_string();

            if &attribute_name == "length_hint" {
                assert!(length_hint.is_none(), "length hint may only be given once");
                length_hint = attribute.tokens.clone().into();
            }

            if &attribute_name == "repeating" {
                assert!(repeating.is_none(), "repeating may only be given once");
                repeating = remove_self_from_stream(attribute.tokens.clone()).into();
            }

            if &attribute_name == "version" {
                assert!(!version, "version may only be given once");
                version = true;
            }

            if &attribute_name == "version_smaller" {
                assert!(version_restriction.is_none() , "version restriction may only be specified once");
                version_restriction = VersionRestriction::Smaller(attribute.parse_args().unwrap()).into();
            }

            if &attribute_name == "version_equals_or_above" {
                assert!(version_restriction.is_none() , "version restriction may only be specified once");
                version_restriction = VersionRestriction::BiggerOrEquals(attribute.parse_args().unwrap()).into();
            }
        }

        let from_length_hint_option = match length_hint.clone() {
            Some(length_stream) => {
                let cleaned_stream = remove_self_from_stream(length_stream);
                quote!(((#cleaned_stream) as usize).into())
            },
            None => quote!(None),
        };

        let length_hint_option = match length_hint {
            Some(length) => quote!(((#length) as usize).into()),
            None => quote!(None),
        };

        from_fields.push(quote!(#field_name));

        let is_repeating = repeating.is_some();
        let version_restricted = version_restriction.is_some();

        let initializer = match repeating {
            Some(repeat_count) => quote!{
                (0..(#repeat_count))
                    .map(|_| crate::traits::ByteConvertable::from_bytes(byte_stream, #from_length_hint_option))
                    .collect()
            },
            None => quote!(crate::traits::ByteConvertable::from_bytes(byte_stream, #from_length_hint_option)),
        };

        let initializer = match &version_restriction {

            Some(restriction) => {

                let function = match restriction.clone() {

                    VersionRestriction::Smaller(version) => {
                        let (major, minor) = (version.first, version.second);
                        quote!(smaller(#major, #minor))
                    }

                    VersionRestriction::BiggerOrEquals(version) => {
                        let (major, minor) = (version.first, version.second);
                        quote!(equals_or_above(#major, #minor))
                    }
                };

                quote!{
                    let #field_name = match byte_stream.get_version().#function {
                        true => Some(#initializer),
                        false => None,
                    };
                }
            }

            None => quote!(let #field_name = #initializer;),
        };

        from_bytes_initializers.push(initializer);

        if version {
            from_bytes_initializers.push(quote!(byte_stream.set_version(#field_name);));
        }

        let initializer = match is_repeating || version_restricted {
            true => quote!(panic!("implement for to_bytes aswell")),
            false => quote!(crate::traits::ByteConvertable::to_bytes(&self.#field_name, #length_hint_option).as_slice()),
        };
        to_bytes_initializers.push(initializer);

        has_fields = true;
    }

    (from_bytes_initializers, from_fields, to_bytes_initializers, has_fields)
}

#[derive(Clone)]
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

#[proc_macro_derive(Packet, attributes(header, length_hint, repeating))]
pub fn derive_packet(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let ast: DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

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

    let (from_bytes_initializers, from_fields, to_bytes_initializers, has_fields) = byte_convertable_helper(named_fields);

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
                    true => {
                        #(#from_bytes_initializers)*
                        Ok(Self { #(#from_fields),* })
                    },
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

#[proc_macro_derive(toggle, attributes(toggle))]
pub fn derive_toggle(item: proc_macro::TokenStream) -> proc_macro::TokenStream {

    let ast: DeriveInput = syn::parse(item).expect("Couldn't parse item");
    let name = Ident::new(&ast.ident.to_string(), ast.ident.span());

    let Data::Struct(data_struct) = ast.data else {
        panic!("only structs may be derived");
    };

    let Fields::Named(named_fields) = data_struct.fields else {
        panic!("only named fields may be derived");
    };

    let mut function_names = Vec::new();
    let mut fields = Vec::new();

    for field in named_fields.named {
        let field_name = field.ident.unwrap();

        for attribute in field.attrs {
            let attribute_name = attribute.path.segments[0].ident.to_string();

            if &attribute_name == "toggle" {
                function_names.push(Ident::new(&format!("toggle_{}", field_name), field_name.span()));
                fields.push(field_name);
                break;
            }
        }
    }

    let expanded = quote! {

        impl #name {

            #( pub fn #function_names(&mut self) {
                self.#fields = !self.#fields;
            })*
        }
    };

    proc_macro::TokenStream::from(expanded)
}
