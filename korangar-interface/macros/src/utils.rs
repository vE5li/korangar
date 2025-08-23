use syn::Attribute;

pub fn get_unique_attribute(attributes: &mut Vec<Attribute>, name: &str) -> Option<Attribute> {
    let mut matching_attributes = attributes.extract_if(.., |attribute| attribute.path().segments[0].ident == name);
    let return_attribute = matching_attributes.next();

    if matching_attributes.next().is_some() {
        panic!("attribute {} may only be specified once per field", name);
    }

    return_attribute
}
