use proc_macro2::Punct;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, LitInt};

#[derive(Clone)]
pub struct PacketSignature {
    pub first: LitInt,
    pub second: LitInt,
}

impl Parse for PacketSignature {

    fn parse(input: ParseStream) -> Result<Self, Error> {

        let first = input.parse().expect("packet header must be two bytes long");
        input.parse::<Punct>().expect("packet header must be seperated by commas");
        let second = input.parse().expect("packet header must be two bytes long");
        Ok(PacketSignature { first, second })
    }
}

pub fn get_unique_attribute(attributes: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {

    let mut matching_attributes = attributes.drain_filter(|attribute| attribute.path.segments[0].ident == name);
    let return_attribute = matching_attributes.next();

    if matching_attributes.next().is_some() {
        panic!("attribute {} may only be specified once per field", name);
    }

    return_attribute
}
