use std::collections::HashMap;

use proc_macro2::{Delimiter, TokenStream};
use quote::{format_ident, quote};
use syn::{DataStruct, Field};

use crate::utils::{Version, VersionAndBuildVersion, get_unique_attribute};

pub fn byte_convertable_helper(
    data_struct: DataStruct,
    versions: Vec<(&str, u16)>,
) -> (
    Vec<TokenStream>,
    Vec<TokenStream>,
    Vec<TokenStream>,
    Vec<TokenStream>,
    Delimiter,
) {
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

        // Check if this field should be skipped for the serialization.
        // This is the case for some packet which hold their headers as field, to handle
        // packet id "shuffling". We use #[hidden_element] which is already used
        // by the UI library to hide fields.
        let skip_payload_serialization = get_unique_attribute(&mut field.attrs, "hidden_element").is_some();

        let is_version = get_unique_attribute(&mut field.attrs, "version").is_some();
        let is_build_version = get_unique_attribute(&mut field.attrs, "build_version").is_some();

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
                quote!(ragnarok_bytes::FromBytesExt::from_n_bytes(byte_reader, #length as usize))
            }
            None if length_remaining => quote!(ragnarok_bytes::FromBytesExt::from_n_bytes(
                byte_reader,
                (__packet_length as usize).saturating_sub(2 + (byte_reader.get_offset() - base_offset))
            )),
            None if length_remaining_off_by_one => quote!(ragnarok_bytes::FromBytesExt::from_n_bytes(
                byte_reader,
                (__packet_length as usize).saturating_sub(1 + (byte_reader.get_offset() - base_offset))
            )),
            None => quote!(ragnarok_bytes::FromBytes::from_bytes(byte_reader)),
        };

        let to_length = match length {
            Some(length) if syn::parse::<syn::Ident>(length.clone().into()).is_ok() => {
                quote!(ragnarok_bytes::ToBytesExt::to_n_bytes(&self.#field_identifier, writer, self.#length as usize))
            }
            Some(length) => quote!(ragnarok_bytes::ToBytesExt::to_n_bytes(&self.#field_identifier, writer, #length as usize)),
            None => quote!(ragnarok_bytes::ToBytes::to_bytes(&self.#field_identifier, writer)),
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

        let version_below = get_unique_attribute(&mut field.attrs, "version_below")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: Version| (version.major, version.minor))
            .map(|(major, minor)| {
                quote! {
                    byte_reader
                        .get_metadata::<Self, dyn ragnarok_formats::version::VersionMetadata>()?
                        .get_version()
                        .ok_or(ragnarok_bytes::ConversionError::from_message("version not set"))?
                        .below(#major, #minor)
                }
            });

        let version_equals_or_above = get_unique_attribute(&mut field.attrs, "version_equals_or_above")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: Version| (version.major, version.minor))
            .map(|(major, minor)| {
                quote! {
                    byte_reader
                        .get_metadata::<Self, dyn ragnarok_formats::version::VersionMetadata>()?
                        .get_version()
                        .ok_or(ragnarok_bytes::ConversionError::from_message("version not set"))?
                        .equals_or_above(#major, #minor)
                }
            });

        let version_and_build_version_equals_or_above = get_unique_attribute(&mut field.attrs, "version_and_build_version_equals_or_above")
            .map(|attribute| attribute.parse_args().expect("failed to parse version"))
            .map(|version: VersionAndBuildVersion| (version.major, version.minor, version.build))
            .map(|(major, minor, build)| {
                quote! {
                    {
                        let internal_version = byte_reader
                            .get_metadata::<Self, dyn ragnarok_formats::version::VersionMetadata>()?
                            .get_version()
                            .ok_or(ragnarok_bytes::ConversionError::from_message("version not set"))?;

                        // HACK: Since the build version is added conditionally, we can't know if
                        // it will be set or not. If we don't, we just ignore that part of the check.
                        let build_version_condition = byte_reader
                            .get_metadata::<Self, dyn ragnarok_formats::version::BuildVersionMetadata>()?
                            .get_build_version()
                            .map(|build_version| build_version.equals_or_above(#build))
                            .unwrap_or(true);

                        internal_version.equals_or_above_with_extra_condition(#major, #minor, build_version_condition)
                    }
                }
            });

        assert!(
            [&version_below, &version_equals_or_above, &version_and_build_version_equals_or_above]
                .iter()
                .filter(|function| function.is_some())
                .count()
                <= 1,
            "version restriction may only be specified once"
        );

        let version_function = version_below
            .or(version_equals_or_above)
            .or(version_and_build_version_equals_or_above);
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
                    let remaining_bytes = __packet_length - ((byte_reader.get_offset() - base_offset) as u16) - 2;
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
                    let #field_variable = match #function {
                        true => Some(#from_implementation),
                        false => None,
                    };
                }
            }
            None => quote!(let #field_variable = #from_implementation;),
        };
        from_bytes_implementations.push(from_implementation);

        // base to byte implementation
        // Skip field we don't want to serialize
        if !skip_payload_serialization {
            let to_implementation = match version_restricted {
                true => quote!(panic!("version restricted fields can't be serialized at the moment");),
                false => quote!(ragnarok_bytes::ConversionResultExt::trace::<Self>(#to_length)?;),
            };
            to_bytes_implementations.push(to_implementation);
        }

        if is_version {
            from_bytes_implementations.push(quote!(
                byte_reader.get_metadata_mut::<Self, dyn ragnarok_formats::version::VersionMetadata>()?.set_version(
                    ragnarok_formats::version::InternalVersion::from(#field_variable)
                );
            ));
        } else if is_build_version {
            from_bytes_implementations.push(quote!(
                // HACK: The build version of the map format is version restricted, meaning
                // that it is wrapped in an `Option`. We could generate code that works for version
                // restricted and non-version restricted build versions but I don't think that will
                // ever be required. So for now I'm assuming that the build number is an `Option`
                // in the generated code.
                if let Some(build_version) = #field_variable {
                    byte_reader.get_metadata_mut::<Self, dyn ragnarok_formats::version::BuildVersionMetadata>()?.set_build_version(
                        build_version
                    );
                }
            ));
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

        match &field.ident {
            Some(field_identifier) => {
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
                } else {
                    match new_value {
                        Some(new_value) => {
                            new_implementations.push(quote! {
                                #field_identifier: #new_value
                            });
                        }
                        _ => {
                            new_arguments.push(quote! {
                                #field_variable: #field_type
                            });
                            new_implementations.push(quote! {
                                #field_identifier: #field_variable
                            });
                        }
                    }
                }
            }
            _ =>
            {
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
                } else {
                    match new_value {
                        Some(new_value) => {
                            new_implementations.push(quote! {
                                #new_value
                            });
                        }
                        _ => {
                            new_arguments.push(quote! {
                                #field_variable: #field_type
                            });
                            new_implementations.push(quote! {
                                #field_variable
                            });
                        }
                    }
                }
            }
        }
    }

    // Generate constructor for each version
    let new_implementations_vec: Vec<TokenStream> = versions
        .into_iter()
        .map(|(version, header_value)| {
            let constructor_name = if version == "default" {
                format_ident!("new")
            } else {
                format_ident!("new_{}", version)
            };

            // Filter out header field from arguments
            let filtered_args: Vec<_> = new_arguments
                .iter()
                .filter(|arg| {
                    let arg_str = arg.to_string();
                    !arg_str.starts_with("header")
                })
                .collect();

            // Filter out header field from implementations and replace it with the
            // version-specific header
            let filtered_impls: Vec<_> = new_implementations
                .iter()
                .map(|impl_tokens| {
                    let impl_str = impl_tokens.to_string();
                    if impl_str.starts_with("header") {
                        // Replace with version-specific header (format as hex)
                        let header_hex = syn::LitInt::new(&format!("{:#x}", header_value), proc_macro2::Span::call_site());
                        quote!(header: ragnarok_packets::PacketHeader(#header_hex))
                    } else {
                        impl_tokens.clone()
                    }
                })
                .collect();

            let constructor_inner = match delimiter {
                Delimiter::Brace => quote!(Self { #(#filtered_impls),* }),
                Delimiter::Parenthesis => quote!(Self(#(#filtered_impls),*)),
                _ => unreachable!(),
            };

            quote! {
                /// Automatically derived `new` function that will fill any fields annotated with any of
                /// the `new` attributes.
                ///
                /// NOTE: The filled fields will *NOT* be updated, so keep that in mind when making
                /// changes to the struct afterwards.
                #[allow(clippy::too_many_arguments)]
                pub fn #constructor_name(#(#filtered_args),*) -> Self {
                    #constructor_inner
                }
            }
        })
        .collect();

    (
        new_implementations_vec,
        from_bytes_implementations,
        implemented_fields,
        to_bytes_implementations,
        delimiter,
    )
}
