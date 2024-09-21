use std::collections::HashMap;

use proc_macro2::{Delimiter, TokenStream};
use quote::{format_ident, quote};
use syn::{DataStruct, Field};

use crate::utils::{get_unique_attribute, Version};

pub fn byte_convertable_helper(data_struct: DataStruct) -> (TokenStream, Vec<TokenStream>, Vec<TokenStream>, Vec<TokenStream>, Delimiter) {
    let mut from_bytes_implementations = vec![];
    let mut implemented_fields = vec![];
    let mut to_bytes_implementations = vec![];
    let mut deriveable_map: HashMap<syn::Ident, (syn::Ident, bool)> = HashMap::new();

    let (mut fields, delimiter): (Vec<Field>, _) = match data_struct.fields {
        syn::Fields::Named(named_fields) => (named_fields.named.into_iter().collect(), Delimiter::Brace),
        syn::Fields::Unnamed(unnamed_fields) => (unnamed_fields.unnamed.into_iter().collect(), Delimiter::Parenthesis),
        syn::Fields::Unit => panic!("unit types are not supported"),
    };

    for (counter, field) in fields.iter_mut().enumerate() {
        let counter_ident = format_ident!("_{}", counter);
        let counter_index = syn::Index::from(counter);
        let field_variable = field.ident.clone().unwrap_or(counter_ident);
        let field_identifier = field.ident.as_ref().map(|ident| quote!(#ident)).unwrap_or(quote!(#counter_index));
        let field_type = field.ty.clone();

        let is_version = get_unique_attribute(&mut field.attrs, "version").is_some();

        let length = get_unique_attribute(&mut field.attrs, "length").map(|attribute| match attribute.meta {
            syn::Meta::List(list) => list.tokens,
            syn::Meta::Path(_) | syn::Meta::NameValue(_) => panic!("expected token stream in attribute"),
        });
        let length_remaining = get_unique_attribute(&mut field.attrs, "length_remaining").is_some();
        let length_remaining_off_by_one = get_unique_attribute(&mut field.attrs, "length_remaining_off_by_one").is_some();

        if (length.is_some() as usize) + (length_remaining as usize) + (length_remaining_off_by_one as usize) > 1 {
            panic!("only one of `length`, `length_remaining`, or `length_remaining_off_by_one` can be used for one field at a time");
        }

        let from_length = match length.clone() {
            Some(length) => {
                quote!(ragnarok_bytes::FromBytesExt::from_n_bytes(byte_stream, #length as usize))
            }
            None if length_remaining => quote!(ragnarok_bytes::FromBytesExt::from_n_bytes(
                byte_stream,
                (__packet_length as usize).saturating_sub(2 + (byte_stream.get_offset() - base_offset))
            )),
            None if length_remaining_off_by_one => quote!(ragnarok_bytes::FromBytesExt::from_n_bytes(
                byte_stream,
                (__packet_length as usize).saturating_sub(1 + (byte_stream.get_offset() - base_offset))
            )),
            None => quote!(ragnarok_bytes::FromBytes::from_bytes(byte_stream)),
        };

        let to_length = match length {
            Some(length) if syn::parse::<syn::Ident>(length.clone().into()).is_ok() => {
                quote!(ragnarok_bytes::ToBytesExt::to_n_bytes(&self.#field_identifier, self.#length as usize))
            }
            Some(length) => quote!(ragnarok_bytes::ToBytesExt::to_n_bytes(&self.#field_identifier, #length as usize)),
            None => quote!(ragnarok_bytes::ToBytes::to_bytes(&self.#field_identifier)),
        };

        let mut repeating: Option<(syn::Ident, bool)> = None;

        if let Some(identifier) = get_unique_attribute(&mut field.attrs, "repeating").map(|attribute| match attribute.meta {
            syn::Meta::List(list) => syn::parse(list.tokens.into()).expect("repeating takes a single identifier"),
            syn::Meta::Path(_) | syn::Meta::NameValue(_) => panic!("expected token stream in attribute"),
        }) {
            repeating = Some((identifier, false));
        }

        if let Some(identifier) = get_unique_attribute(&mut field.attrs, "repeating_option").map(|attribute| match attribute.meta {
            syn::Meta::List(list) => syn::parse(list.tokens.into()).expect("repeating takes a single identifier"),
            syn::Meta::Path(_) | syn::Meta::NameValue(_) => panic!("expected token stream in attribute"),
        }) {
            repeating = Some((identifier, true));
        }

        let repeating_remaining = get_unique_attribute(&mut field.attrs, "repeating_remaining").is_some();
        let repeating_expr = get_unique_attribute(&mut field.attrs, "repeating_expr").map(|attribute| match attribute.meta {
            syn::Meta::List(list) => list.tokens,
            syn::Meta::Path(_) | syn::Meta::NameValue(_) => panic!("expected token stream in attribute"),
        });

        let version_smaller = get_unique_attribute(&mut field.attrs, "version_smaller")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: Version| (version.major, version.minor))
            .map(|(major, minor)| quote!(smaller(#major, #minor)));

        let version_equals_or_above = get_unique_attribute(&mut field.attrs, "version_equals_or_above")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: Version| (version.major, version.minor))
            .map(|(major, minor)| quote!(equals_or_above(#major, #minor)));

        assert!(
            version_smaller.is_none() || version_equals_or_above.is_none(),
            "version restriction may only be specified once"
        );
        let version_function = version_smaller.or(version_equals_or_above);
        let version_restricted = version_function.is_some();

        // base from bytes implementation
        let from_implementation = quote!(ragnarok_bytes::ConversionResultExt::trace::<Self>(#from_length)?);

        // wrap base implementation in a loop if the element can appear multiple times
        let from_implementation = match repeating {
            Some((repeat_count, is_option)) => {
                deriveable_map.insert(repeat_count.clone(), (field_variable.clone(), is_option));

                let repeat_count_inner = match is_option {
                    true => quote!(#repeat_count.unwrap_or_default()),
                    false => quote!(#repeat_count),
                };

                quote!({
                    let repeat_count = #repeat_count_inner;
                    // TODO: Add check to make sure this allocation is not too big.
                    let mut vector = Vec::with_capacity(repeat_count as usize);

                    for _ in 0..repeat_count {
                        vector.push(#from_implementation);
                    }

                    vector
                })
            }
            None if repeating_remaining => {
                quote!({
                    let remaining_bytes = __packet_length - ((byte_stream.get_offset() - base_offset) as u16) - 2;
                    let struct_size = <#field_type as ragnarok_bytes::FixedByteSizeCollection>::size_in_bytes() as u16;

                    if remaining_bytes % struct_size != 0 {
                        return Err(ragnarok_bytes::ConversionError::from_message("type doesn't perfectly divide remaining data"));
                    }

                    let repeat_count = (remaining_bytes / struct_size) as usize;
                    // TODO: Add check to make sure this allocation is not too big.
                    let mut vector = Vec::with_capacity(repeat_count);

                    for _ in 0..repeat_count {
                        vector.push(#from_implementation);
                    }

                    vector
                })
            }
            None if repeating_expr.is_some() => {
                let repeating_expr = repeating_expr.unwrap();

                quote!({
                    let repeat_count = (#repeating_expr) as usize;

                    // TODO: Add check to make sure this allocation is not too big.
                    let mut vector = Vec::with_capacity(repeat_count);

                    for _ in 0..repeat_count {
                        vector.push(#from_implementation);
                    }

                    vector
                })
            }
            None => from_implementation,
        };

        implemented_fields.push(quote!(#field_variable));

        // wrap the potentially looped implementation in an option if it has a version
        // restriction
        let from_implementation = match version_function {
            Some(function) => {
                quote! {
                    let #field_variable = match byte_stream
                            .get_metadata::<Self, Option<ragnarok_formats::version::InternalVersion>>()?
                            .ok_or(ragnarok_bytes::ConversionError::from_message("version not set"))?
                            .#function {
                        true => Some(#from_implementation),
                        false => None,
                    };
                }
            }
            None => quote!(let #field_variable = #from_implementation;),
        };
        from_bytes_implementations.push(from_implementation);

        // base to byte implementation
        let to_implementation = match version_restricted {
            true => quote!({
                panic!("version restricted fields can't be serialized at the moment");
                [0u8].as_slice()
            }),
            false => {
                quote!(ragnarok_bytes::ConversionResultExt::trace::<Self>(#to_length)?.as_slice())
            }
        };
        to_bytes_implementations.push(to_implementation);

        if is_version {
            from_bytes_implementations.push(
                quote!(*byte_stream.get_metadata_mut::<Self, Option<ragnarok_formats::version::InternalVersion>>()? = Some(ragnarok_formats::version::InternalVersion::from(#field_variable));),
            );
        }
    }

    // Implement `new` function.

    let mut new_arguments = vec![];
    let mut new_implementations = vec![];

    for (counter, mut field) in fields.into_iter().enumerate() {
        let counter_ident = format_ident!("_{}", counter);
        let field_variable = field.ident.clone().unwrap_or(counter_ident);

        let is_new_derive = get_unique_attribute(&mut field.attrs, "new_derive").is_some();
        let is_new_default = get_unique_attribute(&mut field.attrs, "new_default").is_some();
        let new_value = get_unique_attribute(&mut field.attrs, "new_value").map(|attribute| match attribute.meta {
            syn::Meta::List(list) => list.tokens,
            syn::Meta::Path(_) | syn::Meta::NameValue(_) => panic!("expected token stream in attribute"),
        });

        if (is_new_derive as usize) + (is_new_default as usize) + (new_value.is_some() as usize) > 1 {
            panic!("only one of `new_derive`, `new_default`, or `new_value` can be used for one field at a time");
        }

        let field_type = field.ty;

        if let Some(field_identifier) = &field.ident {
            if is_new_derive {
                let (collection, is_option) = deriveable_map.get(&field_variable).expect("can't derive field without repeat");

                match is_option {
                    true => {
                        new_implementations.push(quote! {
                            #field_identifier: Some(#collection.len() as _)
                        });
                    }
                    false => {
                        new_implementations.push(quote! {
                            #field_identifier: #collection.len() as _
                        });
                    }
                }
            } else if is_new_default {
                new_implementations.push(quote! {
                    #field_identifier: Default::default()
                });
            } else if let Some(new_value) = new_value {
                new_implementations.push(quote! {
                    #field_identifier: #new_value
                });
            } else {
                new_arguments.push(quote! {
                    #field_variable: #field_type
                });
                new_implementations.push(quote! {
                    #field_identifier: #field_variable
                });
            }
        } else {
            #[allow(clippy::collapsible_else_if)]
            if is_new_derive {
                let (collection, is_option) = deriveable_map.get(&field_variable).expect("can't derive field without repeat");

                match is_option {
                    true => {
                        new_implementations.push(quote! {
                            Some(#collection.len() as _)
                        });
                    }
                    false => {
                        new_implementations.push(quote! {
                            #collection.len() as _
                        });
                    }
                }
            } else if is_new_default {
                new_implementations.push(quote! {
                    Default::default()
                });
            } else if let Some(new_value) = new_value {
                new_implementations.push(quote! {
                    #new_value
                });
            } else {
                new_arguments.push(quote! {
                    #field_variable: #field_type
                });
                new_implementations.push(quote! {
                    #field_variable
                });
            }
        }
    }

    let new_implementation_inner = match delimiter {
        Delimiter::Brace => quote!(Self { #(#new_implementations),* }),
        Delimiter::Parenthesis => quote!(Self(#(#new_implementations),*)),
        _ => unreachable!(),
    };

    let new_implementation = quote! {
        /// Automatically derived `new` function that will fill any fields annotated with any of
        /// the `new` attributes.
        ///
        /// NOTE: The filled fields will *NOT* be updated, so keep that in mind when making
        /// changes to the struct afterwards.
        #[allow(too_many_arguments)]
        pub fn new(#(#new_arguments),*) -> Self {
            #new_implementation_inner
        }
    };

    (
        new_implementation,
        from_bytes_implementations,
        implemented_fields,
        to_bytes_implementations,
        delimiter,
    )
}
