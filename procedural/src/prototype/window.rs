use proc_macro::TokenStream as InterfaceTokenStream;
use syn::{ Ident, DataStruct, Generics, Attribute };
use quote::quote;

use super::helper::prototype_element_helper;

pub fn derive_prototype_window_struct(data_struct: DataStruct, generics: Generics, attributes: Vec<Attribute>, name: Ident) -> InterfaceTokenStream {

    let (initializers, window_title, window_class) = prototype_element_helper(data_struct, attributes);
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let (window_class_option, window_class_ref_option) = window_class
        .map(|window_class| (quote!(#window_class.to_string().into()), quote!(#window_class.into())))
        .unwrap_or((quote!(None), quote!(None)));

    quote! {
        impl #impl_generics crate::interface::traits::PrototypeWindow for #name #type_generics #where_clause {

            fn window_class(&self) -> Option<&str> {
                #window_class_ref_option
            }

            fn to_window(&self, window_cache: &crate::interface::types::WindowCache, interface_settings: &crate::interface::types::InterfaceSettings, avalible_space: crate::interface::types::Size) -> std::boxed::Box<dyn crate::interface::traits::Window + 'static> {
                let elements: Vec<crate::interface::types::ElementCell> = vec![#(#initializers),*];
                let size_constraint = constraint!(200 > 300 < 400, 100 > ? < 80%);
                std::boxed::Box::new(crate::interface::windows::FramedWindow::new(window_cache, interface_settings, avalible_space, #window_title.to_string(), #window_class_option, elements, size_constraint))
            }
        }
    }.into()
}
