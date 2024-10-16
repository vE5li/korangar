use std::collections::HashMap;

use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Path, Token};

pub enum ParameterValue {
    MustSet,
    CanOverride(Expr),
}

pub fn component_macro_inner(
    token_stream: proc_macro2::TokenStream,
    element_type: Path,
    property_values: &Vec<(&'static str, ParameterValue)>,
) -> proc_macro2::TokenStream {
    #[derive(Clone)]
    enum ExprOrIdent {
        Expr(Expr),
        Ident(Ident),
    }

    impl quote::ToTokens for ExprOrIdent {
        fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
            match self {
                ExprOrIdent::Expr(expr) => expr.to_tokens(tokens),
                ExprOrIdent::Ident(ident) => ident.to_tokens(tokens),
            }
        }
    }

    struct PropertyValueInput {
        entries: HashMap<String, ExprOrIdent>,
    }

    impl Parse for PropertyValueInput {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut entries = HashMap::new();

            while !input.is_empty() {
                let property_name: Ident = input.parse()?;
                if input.parse::<Token![:]>().is_err() {
                    entries.insert(property_name.to_string(), ExprOrIdent::Ident(property_name));

                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    } else {
                        break;
                    }

                    continue;
                }

                let value: Expr = input.parse()?;
                entries.insert(property_name.to_string(), ExprOrIdent::Expr(value));

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
    let entries = property_value_pairs.entries;

    let values = property_values.iter().map(|(property, value)| {
        entries.get(*property).cloned().unwrap_or_else(|| match value {
            ParameterValue::MustSet => panic!("Property {} needs to be set", property),
            ParameterValue::CanOverride(expr) => ExprOrIdent::Expr(expr.clone()),
        })
    });

    quote! {
        {
            use korangar_interface::prelude::*;

            #element_type::component_new(#(#values),*)
        }
    }
}

#[macro_export]
macro_rules! create_component_macro {
    ($path:path, { $( $property:ident: $value:tt ,)* }) => {
        fn macro_impl(token_stream: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
            let parameters = Vec::from([
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
    (_dont_use, { $value:expr }) => {
        $crate::ParameterValue::CanOverride(syn::parse_quote!($value))
    };
}
