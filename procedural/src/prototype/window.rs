use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataStruct, Generics, Ident};

use super::helper::prototype_element_helper;

pub fn derive_prototype_window_struct(
    data_struct: DataStruct,
    generics: Generics,
    attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (initializers, window_title, window_class) = prototype_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let (window_class_option, window_class_ref_option) = window_class
        .map(|window_class| (quote!(#window_class.to_string().into()), quote!(#window_class.into())))
        .unwrap_or((quote!(None), quote!(None)));

    quote! {
        impl #impl_generics crate::interface::PrototypeWindow for #name #type_generics #where_clause {

            fn window_class(&self) -> Option<&str> {
                #window_class_ref_option
            }

            fn to_window(&self, window_cache: &crate::interface::WindowCache, interface_settings: &crate::interface::InterfaceSettings, avalible_space: crate::interface::Size) -> crate::interface::Window {
                let scroll_view = crate::interface::ScrollView::new(vec![#(#initializers),*], constraint!(100%, ?));
                let elements: Vec<crate::interface::ElementCell> = vec![std::rc::Rc::new(std::cell::RefCell::new(scroll_view))];

                crate::interface::WindowBuilder::default()
                    .with_title(#window_title.to_string())
                    .with_class_option(#window_class_option)
                    .with_elements(elements)
                    .closable()
                    .build(window_cache, interface_settings, avalible_space)
            }
        }
    }.into()
}
