use proc_macro2::Punct;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Error, LitInt};

#[derive(Clone)]
pub struct PacketSignature {
    pub signature: u16,
}

impl Parse for PacketSignature {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let signature: LitInt = input.parse().expect("packet header must be u16");
        Ok(PacketSignature {
            signature: signature.base10_parse::<u16>()?,
        })
    }
}

#[derive(Clone)]
pub struct Version {
    pub major: LitInt,
    pub minor: LitInt,
}

impl Parse for Version {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let major = input.parse().expect("version must be two bytes long");
        input.parse::<Punct>().expect("version must be separated by commas");
        let minor = input.parse().expect("version must be two bytes long");
        Ok(Version { major, minor })
    }
}

#[derive(Clone)]
pub struct VersionAndBuildVersion {
    pub version: Version,
    pub build_version: LitInt,
}

impl Parse for VersionAndBuildVersion {
    fn parse(input: ParseStream) -> Result<Self, Error> {
        let version = input.parse().expect("version format is not correct");
        input.parse::<Punct>().expect("version build must be separated by commas");
        let build_version = input.parse().expect("build must be two bytes long");
        Ok(VersionAndBuildVersion { version, build_version })
    }
}

pub fn get_unique_attribute(attributes: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
    let mut matching_attributes = attributes.extract_if(.., |attribute| attribute.path().segments[0].ident == name);
    let return_attribute = matching_attributes.next();

    if matching_attributes.next().is_some() {
        panic!("attribute {} may only be specified once per field", name);
    }

    return_attribute
}
