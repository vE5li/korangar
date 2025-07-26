use interface_component_macros::create_component_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn window(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::window::Window, {
        title_marker: { const std::marker::PhantomData },
        title: !,
        title_color: { korangar_interface::theme::theme().window().title_color() },
        hovered_title_color: { korangar_interface::theme::theme().window().hovered_title_color() },
        background_color: { korangar_interface::theme::theme().window().background_color() },
        title_height: { korangar_interface::theme::theme().window().title_height() },
        title_gap: { korangar_interface::theme::theme().window().title_gap() },
        font_size: { korangar_interface::theme::theme().window().font_size() },
        gaps: { korangar_interface::theme::theme().window().gaps() },
        border: { korangar_interface::theme::theme().window().border() },
        corner_radius: { korangar_interface::theme::theme().window().corner_radius() },
        closable: { false },
        close_button_size: { korangar_interface::theme::theme().window().close_button_size() },
        close_button_corner_radius: { korangar_interface::theme::theme().window().close_button_corner_radius() },
        minimum_width: { korangar_interface::theme::theme().window().minimum_width() },
        maximum_width: { korangar_interface::theme::theme().window().maximum_width() },
        minimum_height: { korangar_interface::theme::theme().window().minimum_height() },
        maximum_height: { korangar_interface::theme::theme().window().maximum_height() },
        class: { None },
        theme: !,
        elements: !,
        layout_info: { const None },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn text(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::text::Text, {
        text_marker: { const std::marker::PhantomData },
        text: !,
        color: { korangar_interface::theme::theme().text().color() },
        height: { korangar_interface::theme::theme().text().height() },
        font_size: { korangar_interface::theme::theme().text().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().text().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().text().vertical_alignment() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn button(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::button::Button, {
        text_marker: { const std::marker::PhantomData },
        text: !,
        tooltip: { "" },
        event: !,
        disabled: { false },
        foreground_color: { korangar_interface::theme::theme().button().foreground_color() },
        background_color: { korangar_interface::theme::theme().button().background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().button().hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().button().hovered_background_color() },
        height: { korangar_interface::theme::theme().button().height() },
        corner_radius: { korangar_interface::theme::theme().button().corner_radius() },
        font_size: { korangar_interface::theme::theme().button().font_size() },
        text_alignment: { korangar_interface::theme::theme().button().text_alignment() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn state_button(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::state_button::StateButton, {
        text_marker: { const std::marker::PhantomData },
        text: !,
        state: !,
        event: !,
        disabled: { false },
        foreground_color: { korangar_interface::theme::theme().state_button().foreground_color() },
        background_color: { korangar_interface::theme::theme().state_button().background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().state_button().hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().state_button().hovered_background_color() },
        checkbox_color: { korangar_interface::theme::theme().state_button().checkbox_color() },
        height: { korangar_interface::theme::theme().state_button().height() },
        corner_radius: { korangar_interface::theme::theme().state_button().corner_radius() },
        font_size: { korangar_interface::theme::theme().state_button().font_size() },
        text_alignment: { korangar_interface::theme::theme().state_button().text_alignment() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn drop_down(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::drop_down::DropDown, {
        options: !,
        selected: !,
        // TODO: Don't use the button theme.
        foreground_color: { korangar_interface::theme::theme().drop_down().button_foreground_color() },
        background_color: { korangar_interface::theme::theme().drop_down().button_background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().drop_down().button_hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().drop_down().button_hovered_background_color() },
        height: { korangar_interface::theme::theme().drop_down().button_height() },
        corner_radius: { korangar_interface::theme::theme().drop_down().button_corner_radius() },
        font_size: { korangar_interface::theme::theme().drop_down().button_font_size() },
        text_alignment: { korangar_interface::theme::theme().drop_down().button_text_alignment() },
        click_handler: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn collapsable(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::collapsable::Collapsable, {
        text_marker: { const std::marker::PhantomData },
        text: !,
        foreground_color: { korangar_interface::theme::theme().collapsable().foreground_color() },
        background_color: { korangar_interface::theme::theme().collapsable().background_color() },
        secondary_background_color: { korangar_interface::theme::theme().collapsable().secondary_background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().collapsable().hovered_foreground_color() },
        icon_color: { korangar_interface::theme::theme().collapsable().icon_color() },
        icon_size: { korangar_interface::theme::theme().collapsable().icon_size() },
        gaps: { korangar_interface::theme::theme().collapsable().gaps() },
        border: { korangar_interface::theme::theme().collapsable().border() },
        corner_radius: { korangar_interface::theme::theme().collapsable().corner_radius() },
        title_height: { korangar_interface::theme::theme().collapsable().title_height() },
        font_size: { korangar_interface::theme::theme().collapsable().font_size() },
        text_alignment: { korangar_interface::theme::theme().collapsable().text_alignment() },
        initially_expanded: { false },
        extra_elements: { () },
        children: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn fragment(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::fragment::Fragment, {
        gaps: { 0.0 },
        border: { 0.0 },
        children: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn split(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::split::Split, {
        children: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn scroll_view(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::scroll_view::ScrollView, {
        children: !,
        height_bound: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn text_box(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::text_box::TextBox, {
        text_marker: { const std::marker::PhantomData },
        text: !,
        state: !,
        input_handler: !,
        hidable: { false },
        foreground_color: { korangar_interface::theme::theme().text_box().foreground_color() },
        background_color: { korangar_interface::theme::theme().text_box().background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().text_box().hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().text_box().hovered_background_color() },
        focused_foreground_color: { korangar_interface::theme::theme().text_box().focused_foreground_color() },
        focused_background_color: { korangar_interface::theme::theme().text_box().focused_background_color() },
        hide_icon_color: { korangar_interface::theme::theme().text_box().hide_icon_color() },
        hovered_hide_icon_color: { korangar_interface::theme::theme().text_box().hovered_hide_icon_color() },
        height: { korangar_interface::theme::theme().text_box().height() },
        corner_radius: { korangar_interface::theme::theme().text_box().corner_radius() },
        font_size: { korangar_interface::theme::theme().text_box().font_size() },
        text_alignment: { korangar_interface::theme::theme().text_box().text_alignment() },
    });

    macro_impl(token_stream.into()).into()
}
