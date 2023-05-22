use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{DataStruct, Generics, Ident};

pub fn derive_fixed_byte_size_struct(data_struct: DataStruct, generics: Generics, name: Ident) -> InterfaceTokenStream {
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let types = data_struct.fields.iter().map(|field| field.ty.clone());

    quote! {
        impl #impl_generics const crate::loaders::FixedByteSize for #name #type_generics #where_clause {

            fn size_in_bytes() -> usize {
                let mut total = 0;
                #(total += <#types as crate::loaders::FixedByteSize>::size_in_bytes();)*
                total
            }
        }
    }
    .into()
}
