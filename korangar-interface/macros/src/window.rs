use proc_macro::TokenStream as InterfaceTokenStream;
use quote::quote;
use syn::{Attribute, DataStruct, Generics, Ident};

use super::helper::state_element_helper;

pub fn derive_state_window_struct(
    data_struct: DataStruct,
    generics: Generics,
    attributes: Vec<Attribute>,
    name: Ident,
) -> InterfaceTokenStream {
    let (initializers, initializers_mut, _is_unnamed, window_title, window_class) =
        state_element_helper(data_struct, attributes, name.to_string());
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let window_class_option = window_class.map(|window_class| quote!(Some(#window_class))).unwrap_or(quote!(None));

    // TODO: Instead get this from the proc macro.
    let impl_for = match std::env::var("CARGO_PKG_NAME").unwrap() == "korangar" {
        true => Some(quote!(crate::state::ClientState)),
        false => None,
    };

    if let Some(impl_for) = impl_for {
        return quote! {
            impl #impl_generics korangar_interface::window::StateWindow<#impl_for> for #name #type_generics #where_clause {
                fn window_class() -> Option<<#impl_for as korangar_interface::application::Application>::WindowClass> {
                    #window_class_option
                }

                fn to_window<'a>(self_path: impl rust_state::Path<#impl_for, Self>,
                ) -> impl korangar_interface::window::Window<#impl_for> + 'a {
                    use korangar_interface::prelude::*;

                    window! {
                        title: #window_title.to_string(),
                        class: #window_class_option,
                        minimum_height: 150.0,
                        theme: <#impl_for as korangar_interface::application::Application>::ThemeType::default(),
                        closable: true,
                        resizable: true,
                        elements: scroll_view! { children: (#(#initializers,)*), }
                    }
                }

                fn to_window_mut<'a>(self_path: impl rust_state::Path<#impl_for, Self>,
                ) -> impl korangar_interface::window::Window<#impl_for> + 'a {
                    use korangar_interface::prelude::*;

                    window! {
                        title: #window_title.to_string(),
                        class: #window_class_option,
                        minimum_height: 150.0,
                        theme: <#impl_for as korangar_interface::application::Application>::ThemeType::default(),
                        closable: true,
                        resizable: true,
                        elements: scroll_view! { children: (#(#initializers_mut,)*), }
                    }
                }
            }
        }
        .into();
    }

    quote! {
        impl<App: korangar_interface::application::Application> #impl_generics korangar_interface::window::StateWindow<App> for #name #type_generics #where_clause {
            fn window_class() -> Option<App::WindowClass> {
                #window_class_option
            }

            fn to_window<'a>(self_path: impl rust_state::Path<App, Self>) -> impl korangar_interface::window::Window<App> + 'a {
                use korangar_interface::prelude::*;

                window! {
                    title: #window_title.to_string(),
                    class: #window_class_option,
                    minimum_height: 150.0,
                    theme: App::ThemeType::default(),
                    closable: true,
                    resizable: true,
                    elements: scroll_view! { children: (#(#initializers,)*), }
                }
            }

            fn to_window_mut<'a>(self_path: impl rust_state::Path<App, Self>) -> impl korangar_interface::window::Window<App> + 'a {
                use korangar_interface::prelude::*;

                window! {
                    title: #window_title.to_string(),
                    class: #window_class_option,
                    minimum_height: 150.0,
                    theme: App::ThemeType::default(),
                    closable: true,
                    resizable: true,
                    elements: scroll_view! { children: (#(#initializers_mut,)*), }
                }
            }
        }
    }.into()
}
