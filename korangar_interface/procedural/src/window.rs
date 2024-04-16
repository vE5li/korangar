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
    let (initializers, _is_unnamed, window_title, window_class) = prototype_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let (window_class_option, window_class_ref_option) = window_class
        .map(|window_class| (quote!(#window_class.to_string().into()), quote!(#window_class.into())))
        .unwrap_or((quote!(None), quote!(None)));

    if std::env::var("CARGO_PKG_NAME").unwrap() == "korangar" {
        return quote! {
            impl #impl_generics korangar_interface::windows::PrototypeWindow<crate::interface::application::InterfaceSettings> for #name #type_generics #where_clause {

                fn window_class(&self) -> Option<&str> {
                    #window_class_ref_option
                }

                fn to_window(&self,
                    window_cache: &crate::interface::windows::WindowCache,
                    application: &crate::interface::application::InterfaceSettings,
                    available_space: crate::interface::layout::ScreenSize
                ) -> korangar_interface::windows::Window<crate::interface::application::InterfaceSettings> {
                    use crate::interface::application::InterfaceSettings;
                    use korangar_interface::elements::ElementCell;
                    use korangar_interface::elements::ScrollView;
                    use korangar_interface::windows::WindowBuilder;
                    use korangar_interface::size_bound;
                    use std::cell::RefCell;
                    use std::rc::Rc;

                    let scroll_view = ScrollView::new(vec![#(#initializers),*], size_bound!(100%, super > ? < super));
                    let elements: Vec<ElementCell<InterfaceSettings>> = vec![Rc::new(RefCell::new(scroll_view))];

                    WindowBuilder::new()
                        .with_title(#window_title.to_string())
                        .with_class_option(#window_class_option)
                        .with_size_bound(size_bound!(200 > 300 < 400, 0 > ? < 80%))
                        .with_elements(elements)
                        .closable()
                        .build(window_cache, application, available_space)
                }
            }
        }.into();
    }

    quote! {
        impl<App: korangar_interface::application::Application> #impl_generics korangar_interface::windows::PrototypeWindow<App> for #name #type_generics #where_clause {

            fn window_class(&self) -> Option<&str> {
                #window_class_ref_option
            }

            fn to_window(&self, window_cache: &App::Cache, application: &App, available_space: App::Size) -> korangar_interface::windows::Window<App> {
                let scroll_view = korangar_interface::elements::ScrollView::new(vec![#(#initializers),*], korangar_interface::size_bound!(100%, super > ? < super));
                let elements: Vec<korangar_interface::elements::ElementCell<App>> = vec![std::rc::Rc::new(std::cell::RefCell::new(scroll_view))];

                korangar_interface::windows::WindowBuilder::new()
                    .with_title(#window_title.to_string())
                    .with_class_option(#window_class_option)
                    .with_size_bound(korangar_interface::size_bound!(200 > 300 < 400, 0 > ? < 80%))
                    .with_elements(elements)
                    .closable()
                    .build(window_cache, application, available_space)
            }
        }
    }.into()
}
