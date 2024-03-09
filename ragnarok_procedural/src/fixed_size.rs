use proc_macro::TokenStream as InterfaceTokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, DataStruct, Field, Generics, Ident};

use crate::utils::get_unique_attribute;

pub fn derive_fixed_byte_size_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let types: Vec<_> = data_struct.fields.iter().map(|field| field.ty.clone()).collect();

    let fields: Vec<Field> = match data_struct.fields {
        syn::Fields::Named(named_fields) => named_fields.named.into_iter().collect(),
        syn::Fields::Unnamed(unnamed_fields) => unnamed_fields.unnamed.into_iter().collect(),
        syn::Fields::Unit => panic!("unit types are not supported"),
    };

    let sizes = fields.into_iter().zip(types.iter()).map(|(mut field, field_type)| {
        get_unique_attribute(&mut field.attrs, "length_hint")
            .map(|attribute| match attribute.meta {
                syn::Meta::List(list) => list.tokens,
                syn::Meta::Path(_) | syn::Meta::NameValue(_) => panic!("expected token stream in attribute"),
            })
            .map(|length_hint| quote!((#length_hint) as usize))
            .unwrap_or(quote!(<#field_type as crate::loaders::FixedByteSize>::size_in_bytes()))
    });

    quote! {
        impl #impl_generics const crate::loaders::FixedByteSize for #name #type_generics #where_clause {
            fn size_in_bytes() -> usize {
                let mut total = 0;
                #(total += #sizes;)*
                total
            }
        }
    }
    .into()
}

pub fn derive_fixed_byte_size_enum(generics: Generics, mut attributes: Vec<Attribute>, name: Ident) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let numeric_type = get_unique_attribute(&mut attributes, "numeric_type")
        .map(|attribute| attribute.parse_args().unwrap())
        .unwrap_or_else(|| Ident::new("u8", Span::call_site()));

    quote! {
        impl #impl_generics const crate::loaders::FixedByteSize for #name #type_generics #where_clause {
            fn size_in_bytes() -> usize {
                <#numeric_type as crate::loaders::FixedByteSize>::size_in_bytes()
            }
        }
    }
    .into()
}
