use proc_macro2::{Punct, TokenStream};
use quote::quote;
use syn::parse::ParseStream;
use syn::Lit;

#[derive(Debug)]
struct Dimension {
    pub stream: TokenStream,
}

impl syn::parse::Parse for Dimension {
    fn parse(input: ParseStream) -> Result<Self, syn::Error> {
        let lookahead = input.lookahead1();

        let expanded = if lookahead.peek(syn::Token![!]) {
            // remove the ','
            input.parse::<Punct>()?;

            quote!(korangar_interface::layout::Dimension::Remaining)
        } else if lookahead.peek(syn::Token![?]) {
            // remove the ','
            input.parse::<Punct>()?;

            quote!(korangar_interface::layout::Dimension::Flexible)
        } else if lookahead.peek(syn::Token![super]) {
            // remove the 'super'
            input.parse::<syn::Token![super]>()?;

            quote!(korangar_interface::layout::Dimension::Super)
        } else {
            let literal: Lit = input.parse()?;

            match literal {
                Lit::Float(..) | Lit::Int(..) => {}
                _ => panic!("literal must be a float or an integer"),
            }

            let lookahead = input.lookahead1();

            if lookahead.peek(syn::Token![%]) {
                input.parse::<Punct>()?;
                quote!(korangar_interface::layout::Dimension::Relative(#literal as f32))
            } else {
                quote!(korangar_interface::layout::Dimension::Absolute(#literal as f32))
            }
        };

        Ok(Dimension { stream: expanded })
    }
}

pub struct SizeBound {
    pub stream: TokenStream,
}

impl syn::parse::Parse for SizeBound {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let first_dimension: Dimension = input.parse().unwrap();

        let (width, minimum_width) = if input.lookahead1().peek(syn::Token![>]) {
            input.parse::<Punct>().unwrap();
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
            input.parse::<Punct>().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();
            let maximum_width = second_dimension.stream;
            quote!(Some(#maximum_width))
        } else {
            quote!(None)
        };

        assert!(
            input.lookahead1().peek(syn::Token![,]),
            "bound expected comma after first dimension"
        );
        input.parse::<Punct>().unwrap();

        let first_dimension: Dimension = input.parse().unwrap();

        let (height, minimum_height) = if input.lookahead1().peek(syn::Token![>]) {
            input.parse::<Punct>().unwrap();
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
            input.parse::<Punct>().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();
            let maximum_height = second_dimension.stream;
            quote!(Some(#maximum_height))
        } else {
            quote!(None)
        };

        let expanded = quote! {
            korangar_interface::layout::SizeBound {
                width: #width,
                minimum_width: #minimum_width,
                maximum_width: #maximum_width,
                height: #height,
                minimum_height: #minimum_height,
                maximum_height: #maximum_height,
            }
        };

        Ok(SizeBound { stream: expanded })
    }
}

pub struct DimensionBound {
    pub stream: TokenStream,
}

impl syn::parse::Parse for DimensionBound {
    fn parse(input: syn::parse::ParseStream) -> Result<Self, syn::Error> {
        let first_dimension: Dimension = input.parse().unwrap();

        let (size, minimum_size) = if input.lookahead1().peek(syn::Token![>]) {
            input.parse::<Punct>().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();

            let minimum_size = first_dimension.stream;
            let size = second_dimension.stream;

            let minimum_size = quote!(Some(#minimum_size));
            (size, minimum_size)
        } else {
            let minimum_size = quote!(None);
            let size = first_dimension.stream;
            (size, minimum_size)
        };

        let maximum_size = if input.lookahead1().peek(syn::Token![<]) {
            input.parse::<Punct>().unwrap();
            let second_dimension: Dimension = input.parse().unwrap();
            let maximum_size = second_dimension.stream;
            quote!(Some(#maximum_size))
        } else {
            quote!(None)
        };

        let expanded = quote! {
            korangar_interface::layout::DimensionBound {
                size: #size,
                minimum_size: #minimum_size,
                maximum_size: #maximum_size,
            }
        };

        Ok(DimensionBound { stream: expanded })
    }
}
