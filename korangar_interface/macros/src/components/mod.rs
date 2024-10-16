// pub mod button;

use std::collections::HashMap;

use proc_macro2::Span;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, Path, Token, parse_macro_input, parse_quote};

fn generic_macro_stuff(
    token_stream: proc_macro::TokenStream,
    element_type: Path,
    default_parameters: &HashMap<&'static str, Option<Expr>>,
) -> proc_macro::TokenStream {
    struct KeyValueInput {
        entries: HashMap<String, Expr>,
    }

    impl Parse for KeyValueInput {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut entries = HashMap::new();

            while !input.is_empty() {
                let key: Ident = input.parse()?;
                input.parse::<Token![:]>()?;

                let value: Expr = input.parse()?;
                entries.insert(key.to_string(), value);

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }

            Ok(KeyValueInput { entries })
        }
    }

    let key_value_pairs = parse_macro_input!(token_stream as KeyValueInput);
    let mut entries = key_value_pairs.entries;

    // Fill in missing default values
    for (key, default) in default_parameters {
        entries.entry((*key).to_owned()).or_insert_with(|| {
            let Some(default) = default else {
                panic!("Key {} needs to be set", key);
            };

            default.clone()
        });
    }

    let keys = entries.keys().map(|field_name| syn::Ident::new(field_name, Span::call_site()));
    let values = entries.values();

    quote! {
        {
            use korangar_interface::prelude::*;

            #element_type {
                #(#keys : #values),*
            }
        }
    }
    .into()
}

// TODO: Maybe move this out of components
pub fn window(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([
        ("title_marker", Some(parse_quote!(std::marker::PhantomData))),
        ("title", None),
        (
            "title_color",
            Some(parse_quote!(korangar_interface::theme::theme().window().title_color())),
        ),
        (
            "hovered_title_color",
            Some(parse_quote!(korangar_interface::theme::theme().window().hovered_title_color())),
        ),
        (
            "background_color",
            Some(parse_quote!(korangar_interface::theme::theme().window().background_color())),
        ),
        (
            "title_height",
            Some(parse_quote!(korangar_interface::theme::theme().window().title_height())),
        ),
        (
            "font_size",
            Some(parse_quote!(korangar_interface::theme::theme().window().font_size())),
        ),
        ("gaps", Some(parse_quote!(korangar_interface::theme::theme().window().gaps()))),
        (
            "border",
            Some(parse_quote!(korangar_interface::theme::theme().window().border())),
        ),
        (
            "corner_radius",
            Some(parse_quote!(korangar_interface::theme::theme().window().corner_radius())),
        ),
        ("theme", None),
        ("window_id", None),
        ("elements", None),
    ]);

    generic_macro_stuff(token_stream, parse_quote!(korangar_interface::window::Window), &parameters)
}

pub fn text(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([
        ("text_marker", Some(parse_quote!(std::marker::PhantomData))),
        ("text", None),
        ("color", Some(parse_quote!(korangar_interface::theme::theme().text().color()))),
        ("height", Some(parse_quote!(korangar_interface::theme::theme().text().height()))),
        (
            "font_size",
            Some(parse_quote!(korangar_interface::theme::theme().text().font_size())),
        ),
        (
            "horizontal_alignment",
            Some(parse_quote!(korangar_interface::theme::theme().text().horizontal_alignment())),
        ),
        (
            "vertical_alignment",
            Some(parse_quote!(korangar_interface::theme::theme().text().vertical_alignment())),
        ),
    ]);

    generic_macro_stuff(
        token_stream,
        parse_quote!(korangar_interface::components::text::Text),
        &parameters,
    )
}

pub fn button(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([
        ("text_marker", Some(parse_quote!(std::marker::PhantomData))),
        ("text", None),
        ("event", None),
        ("disabled", Some(parse_quote!(false))),
        (
            "foreground_color",
            Some(parse_quote!(korangar_interface::theme::theme().button().foreground_color())),
        ),
        (
            "background_color",
            Some(parse_quote!(korangar_interface::theme::theme().button().background_color())),
        ),
        (
            "hovered_foreground_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().button().hovered_foreground_color()
            )),
        ),
        (
            "hovered_background_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().button().hovered_background_color()
            )),
        ),
        (
            "height",
            Some(parse_quote!(korangar_interface::theme::theme().button().height())),
        ),
        (
            "corner_radius",
            Some(parse_quote!(korangar_interface::theme::theme().button().corner_radius())),
        ),
        (
            "font_size",
            Some(parse_quote!(korangar_interface::theme::theme().button().font_size())),
        ),
        (
            "text_alignment",
            Some(parse_quote!(korangar_interface::theme::theme().button().text_alignment())),
        ),
    ]);

    generic_macro_stuff(
        token_stream,
        parse_quote!(korangar_interface::components::button::Button),
        &parameters,
    )
}

pub fn state_button(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([
        ("text_marker", Some(parse_quote!(std::marker::PhantomData))),
        ("text", None),
        ("state", None),
        ("event", None),
        ("disabled", Some(parse_quote!(false))),
        (
            "foreground_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().state_button().foreground_color()
            )),
        ),
        (
            "background_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().state_button().background_color()
            )),
        ),
        (
            "hovered_foreground_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().state_button().hovered_foreground_color()
            )),
        ),
        (
            "hovered_background_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().state_button().hovered_background_color()
            )),
        ),
        (
            "checkbox_color",
            Some(parse_quote!(korangar_interface::theme::theme().state_button().checkbox_color())),
        ),
        (
            "height",
            Some(parse_quote!(korangar_interface::theme::theme().state_button().height())),
        ),
        (
            "corner_radius",
            Some(parse_quote!(korangar_interface::theme::theme().state_button().corner_radius())),
        ),
        (
            "font_size",
            Some(parse_quote!(korangar_interface::theme::theme().state_button().font_size())),
        ),
        (
            "text_alignment",
            Some(parse_quote!(korangar_interface::theme::theme().state_button().text_alignment())),
        ),
    ]);

    generic_macro_stuff(
        token_stream,
        parse_quote!(korangar_interface::components::state_button::StateButton),
        &parameters,
    )
}

pub fn collapsable(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([
        ("text_marker", Some(parse_quote!(std::marker::PhantomData))),
        ("text", None),
        (
            "foreground_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().collapsable().foreground_color()
            )),
        ),
        (
            "background_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().collapsable().background_color()
            )),
        ),
        (
            "hovered_foreground_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().collapsable().hovered_foreground_color()
            )),
        ),
        (
            "gaps",
            Some(parse_quote!(korangar_interface::theme::theme().collapsable().gaps())),
        ),
        (
            "border",
            Some(parse_quote!(korangar_interface::theme::theme().collapsable().border())),
        ),
        (
            "corner_radius",
            Some(parse_quote!(korangar_interface::theme::theme().collapsable().corner_radius())),
        ),
        (
            "title_height",
            Some(parse_quote!(korangar_interface::theme::theme().collapsable().title_height())),
        ),
        (
            "font_size",
            Some(parse_quote!(korangar_interface::theme::theme().collapsable().font_size())),
        ),
        (
            "text_alignment",
            Some(parse_quote!(korangar_interface::theme::theme().collapsable().text_alignment())),
        ),
        ("children", None),
    ]);

    generic_macro_stuff(
        token_stream,
        parse_quote!(korangar_interface::components::collapsable::Collapsable),
        &parameters,
    )
}

pub fn scroll_view(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([("children", None), ("height_bound", None)]);

    generic_macro_stuff(
        token_stream,
        parse_quote!(korangar_interface::components::scroll_view::ScrollView),
        &parameters,
    )
}

pub fn text_box(token_stream: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parameters = HashMap::from([
        ("text_marker", Some(parse_quote!(std::marker::PhantomData))),
        ("text", None),
        ("state", None),
        (
            "input_handler",
            Some(parse_quote!(korangar_interface::components::text_box::DefaultHandler)),
        ),
        ("disabled", Some(parse_quote!(false))),
        (
            "foreground_color",
            Some(parse_quote!(korangar_interface::theme::theme().text_box().foreground_color())),
        ),
        (
            "background_color",
            Some(parse_quote!(korangar_interface::theme::theme().text_box().background_color())),
        ),
        (
            "hovered_foreground_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().text_box().hovered_foreground_color()
            )),
        ),
        (
            "hovered_background_color",
            Some(parse_quote!(
                korangar_interface::theme::theme().text_box().hovered_background_color()
            )),
        ),
        (
            "height",
            Some(parse_quote!(korangar_interface::theme::theme().text_box().height())),
        ),
        (
            "corner_radius",
            Some(parse_quote!(korangar_interface::theme::theme().text_box().corner_radius())),
        ),
        (
            "font_size",
            Some(parse_quote!(korangar_interface::theme::theme().text_box().font_size())),
        ),
        (
            "text_alignment",
            Some(parse_quote!(korangar_interface::theme::theme().text_box().text_alignment())),
        ),
    ]);

    generic_macro_stuff(
        token_stream,
        parse_quote!(korangar_interface::components::text_box::TextBox),
        &parameters,
    )
}
