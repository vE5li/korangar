use proc_macro2::{ TokenStream, TokenTree, Group };
use syn::FieldsNamed;
use quote::quote;

use crate::utils::*;

fn remove_self_from_stream(token_stream: TokenStream) -> TokenStream {

    let mut new_stream = TokenStream::new();
    let mut iterator = token_stream.into_iter();

    while let Some(token) = iterator.next() {

        if let TokenTree::Group(group) = &token {
            let delimiter = group.delimiter();
            let new_group_stream = remove_self_from_stream(group.stream());
            let new_group = Group::new(delimiter, new_group_stream);
            new_stream.extend_one(TokenTree::Group(new_group));
            continue;
        }

        if let TokenTree::Ident(ident) = &token {
            if &ident.to_string() == "self" {
                // remove the '.' after self
                iterator.next().expect("expected a token after self");
                continue;
            }
        }

        new_stream.extend_one(token);
    }

    new_stream
}

pub fn byte_convertable_helper(named_fields: FieldsNamed) -> (Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>) {

    let mut from_bytes_implementations = vec![];
    let mut implemented_fields = vec![];
    let mut to_bytes_implementations = vec![];

    for mut field in named_fields.named {

        let field_name = field.ident.unwrap();

        let is_version = get_unique_attribute(&mut field.attrs, "version").is_some();

        let length_hint = get_unique_attribute(&mut field.attrs, "length_hint")
            .map(|attribute| attribute.tokens.into())
            .map(|length_hint: TokenStream| quote!(((#length_hint) as usize).into()))
            .unwrap_or(quote!(None));

        let from_length_hint = remove_self_from_stream(length_hint.clone());

        let repeating: Option<TokenStream> = get_unique_attribute(&mut field.attrs, "repeating")
            .map(|attribute| remove_self_from_stream(attribute.tokens).into());

        let version_smaller = get_unique_attribute(&mut field.attrs, "version_smaller")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: PacketSignature| (version.first, version.second))
            .map(|(major, minor)| quote!(smaller(#major, #minor)));

        let version_equals_or_above = get_unique_attribute(&mut field.attrs, "version_equals_or_above")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: PacketSignature| (version.first, version.second))
            .map(|(major, minor)| quote!(equals_or_above(#major, #minor)));

        assert!(version_smaller.is_none() || version_equals_or_above.is_none(), "version restriction may only be specified once");
        let version_function = version_smaller.or(version_equals_or_above);
        let version_restricted = version_function.is_some();
        let is_repeating = repeating.is_some();

        // base from bytes implementation
        let from_implementation = quote!(crate::traits::ByteConvertable::from_bytes(byte_stream, #from_length_hint));

        // wrap base implementation in a loop if the element can appear multiple times
        let from_implementation = match repeating {
            Some(repeat_count) => quote!((0..(#repeat_count)).map(|_| #from_implementation).collect()),
            None => from_implementation,
        };

        // wrap the potentially looped implementation in an option if it has a version restriction
        let from_implementation = match version_function {

            Some(function) => {
                quote!{
                    let #field_name = match byte_stream.get_version().#function {
                        true => Some(#from_implementation),
                        false => None,
                    };
                }
            }

            None => quote!(let #field_name = #from_implementation;),
        };

        // base to byte implementation
        let to_implementation = match is_repeating || version_restricted {
            true => quote!(panic!("implement for to_bytes aswell")),
            false => quote!(crate::traits::ByteConvertable::to_bytes(&self.#field_name, #length_hint).as_slice()),
        };

        implemented_fields.push(quote!(#field_name));
        from_bytes_implementations.push(from_implementation);
        to_bytes_implementations.push(to_implementation);

        if is_version {
            from_bytes_implementations.push(quote!(byte_stream.set_version(#field_name);));
        }
    }

    (from_bytes_implementations, implemented_fields, to_bytes_implementations)
}
