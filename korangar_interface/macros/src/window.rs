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
    let (initializers, _initializers_mut, _is_unnamed, window_title, window_class) =
        prototype_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let window_class_option = window_class.map(|window_class| quote!(Some(#window_class))).unwrap_or(quote!(None));

    // TODO: Instead get this from the proc macro.
    let impl_for = match std::env::var("CARGO_PKG_NAME").unwrap() == "korangar" {
        true => Some(quote!(crate::state::ClientState)),
        false => None,
    };

    if let Some(impl_for) = impl_for {
        return quote! {
            impl #impl_generics korangar_interface::window::PrototypeWindow<#impl_for> for #name #type_generics #where_clause {
                fn window_class() -> Option<<#impl_for as korangar_interface::application::Appli>::WindowClass> {
                    #window_class_option
                }

                fn to_window<'a>(self_path: impl rust_state::Path<#impl_for, Self>,
                ) -> impl korangar_interface::window::WindowTrait<#impl_for> + 'a {
                    use korangar_interface::prelude::*;

                    window! {
                        title: #window_title.to_string(),
                        theme: <#impl_for as korangar_interface::application::Appli>::ThemeType::default(),
                        closable: true,
                        elements: (scroll_view! { children: (#(#initializers,)*), height_bound: HeightBound::WithMax, }, )
                    }
                }
            }
        }
        .into();
    }

    quote! {
        impl<App: korangar_interface::application::Appli> #impl_generics korangar_interface::window::PrototypeWindow<App> for #name #type_generics #where_clause {
            fn window_class() -> Option<App::WindowClass> {
                #window_class_option
            }

            fn to_window<'a>(self_path: impl rust_state::Path<App, Self>) -> impl korangar_interface::window::WindowTrait<App> + 'a {
                use korangar_interface::prelude::*;

                window! {
                    title: #window_title.to_string(),
                    theme: App::ThemeType::default(),
                    closable: true,
                    elements: (scroll_view! { children: (#(#initializers,)*), height_bound: HeightBound::WithMax, }, )
                }
            }
        }
    }.into()
}
