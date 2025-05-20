use std::collections::HashMap;

use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Path, Token};

pub enum ParameterValue {
    MustSet,
    CanOverride(Expr),
    Fixed(Expr),
}

pub fn component_macro_inner(
    token_stream: proc_macro2::TokenStream,
    element_type: Path,
    property_values: &HashMap<&'static str, ParameterValue>,
) -> proc_macro2::TokenStream {
    struct PropertyValueInput {
        entries: HashMap<String, Expr>,
    }

    impl Parse for PropertyValueInput {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut entries = HashMap::new();

            while !input.is_empty() {
                let property_name: Ident = input.parse()?;
                input.parse::<Token![:]>()?;

                let value: Expr = input.parse()?;
                entries.insert(property_name.to_string(), value);

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }

            Ok(PropertyValueInput { entries })
        }
    }

    // TODO: Don't unwrap.
    let property_value_pairs: PropertyValueInput = syn::parse2(token_stream).unwrap();
    let mut entries = property_value_pairs.entries;

    // Fill in missing default values
    for (property, value) in property_values {
        if let ParameterValue::Fixed(..) = value {
            if entries.contains_key(*property) {
                // TODO: Collect errors and return multiple.
                panic!("Property {} can not be overwritten", property);
            }
        }

        entries.entry((*property).to_owned()).or_insert_with(|| {
            match value {
                // TODO: Collect errors and return multiple.
                ParameterValue::MustSet => panic!("Property {} needs to be set", property),
                ParameterValue::CanOverride(expr) => expr.clone(),
                ParameterValue::Fixed(expr) => expr.clone(),
            }
        });
    }

    let properties = entries.keys().map(|field_name| syn::Ident::new(field_name, Span::call_site()));
    let values = entries.values();

    quote! {
        {
            use korangar_interface::prelude::*;

            #element_type {
                #(#properties : #values),*
            }
        }
    }
}

#[macro_export]
macro_rules! create_component_macro {
    ($path:path, { $( $property:ident: $value:tt ,)* }) => {
        fn macro_impl(token_stream: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
            let parameters = std::collections::HashMap::from([
                $(
                    (stringify!($property), create_component_macro!(_dont_use, $value))
                ),*
            ]);

            $crate::component_macro_inner(token_stream, syn::parse_quote!($path), &parameters)
        }
    };
    (_dont_use, !) => {
        $crate::ParameterValue::MustSet
    };
    (_dont_use, { const $value:expr }) => {
        $crate::ParameterValue::Fixed(syn::parse_quote!($value))
    };
    (_dont_use, { $value:expr }) => {
        $crate::ParameterValue::CanOverride(syn::parse_quote!($value))
    };
}
