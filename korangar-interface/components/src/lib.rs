use interface_component_macros::create_component_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn window(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::window::WindowInternal, {
        title: !,
        title_color: { korangar_interface::theme::theme().window().title_color() },
        hovered_title_color: { korangar_interface::theme::theme().window().hovered_title_color() },
        background_color: { korangar_interface::theme::theme().window().background_color() },
        title_height: { korangar_interface::theme::theme().window().title_height() },
        title_gap: { korangar_interface::theme::theme().window().title_gap() },
        font_size: { korangar_interface::theme::theme().window().font_size() },
        gaps: { korangar_interface::theme::theme().window().gaps() },
        border: { korangar_interface::theme::theme().window().border() },
        corner_diameter: { korangar_interface::theme::theme().window().corner_diameter() },
        closable: { false },
        resizable: { false },
        close_button_size: { korangar_interface::theme::theme().window().close_button_size() },
        close_button_corner_diameter: { korangar_interface::theme::theme().window().close_button_corner_diameter() },
        minimum_width: { korangar_interface::theme::theme().window().minimum_width() },
        maximum_width: { korangar_interface::theme::theme().window().maximum_width() },
        minimum_height: { korangar_interface::theme::theme().window().minimum_height() },
        maximum_height: { korangar_interface::theme::theme().window().maximum_height() },
        theme: !,
        class: { None },
        elements: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn text(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::text::Text, {
        text: !,
        color: { korangar_interface::theme::theme().text().color() },
        height: { korangar_interface::theme::theme().text().height() },
        font_size: { korangar_interface::theme::theme().text().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().text().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().text().vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().text().overflow_behavior() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn button(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::button::Button, {
        text: !,
        tooltip: { "" },
        event: !,
        disabled: { false },
        disabled_tooltip: { "" },
        foreground_color: { korangar_interface::theme::theme().button().foreground_color() },
        background_color: { korangar_interface::theme::theme().button().background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().button().hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().button().hovered_background_color() },
        disabled_foreground_color: { korangar_interface::theme::theme().button().disabled_foreground_color() },
        disabled_background_color: { korangar_interface::theme::theme().button().disabled_background_color() },
        height: { korangar_interface::theme::theme().button().height() },
        corner_diameter: { korangar_interface::theme::theme().button().corner_diameter() },
        font_size: { korangar_interface::theme::theme().button().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().button().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().button().vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().button().overflow_behavior() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn state_button(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::state_button::StateButton, {
        text: !,
        tooltip: { "" },
        state: !,
        event: !,
        disabled: { false },
        disabled_tooltip: { "" },
        foreground_color: { korangar_interface::theme::theme().state_button().foreground_color() },
        background_color: { korangar_interface::theme::theme().state_button().background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().state_button().hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().state_button().hovered_background_color() },
        disabled_foreground_color: { korangar_interface::theme::theme().state_button().disabled_foreground_color() },
        disabled_background_color: { korangar_interface::theme::theme().state_button().disabled_background_color() },
        checkbox_color: { korangar_interface::theme::theme().state_button().checkbox_color() },
        hovered_checkbox_color: { korangar_interface::theme::theme().state_button().hovered_checkbox_color() },
        disabled_checkbox_color: { korangar_interface::theme::theme().state_button().disabled_checkbox_color() },
        height: { korangar_interface::theme::theme().state_button().height() },
        corner_diameter: { korangar_interface::theme::theme().state_button().corner_diameter() },
        font_size: { korangar_interface::theme::theme().state_button().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().state_button().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().state_button().vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().state_button().overflow_behavior() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn drop_down(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::drop_down::DropDown, {
        options: !,
        selected: !,
        foreground_color: { korangar_interface::theme::theme().drop_down().button_foreground_color() },
        background_color: { korangar_interface::theme::theme().drop_down().button_background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().drop_down().button_hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().drop_down().button_hovered_background_color() },
        height: { korangar_interface::theme::theme().drop_down().button_height() },
        corner_diameter: { korangar_interface::theme::theme().drop_down().button_corner_diameter() },
        font_size: { korangar_interface::theme::theme().drop_down().button_font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().drop_down().button_horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().drop_down().button_vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().drop_down().button_overflow_behavior() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn collapsable(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::collapsable::Collapsable, {
        text: !,
        tooltip: { "" },
        foreground_color: { korangar_interface::theme::theme().collapsable().foreground_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().collapsable().hovered_foreground_color() },
        background_color: { korangar_interface::theme::theme().collapsable().background_color() },
        secondary_background_color: { korangar_interface::theme::theme().collapsable().secondary_background_color() },
        icon_color: { korangar_interface::theme::theme().collapsable().icon_color() },
        icon_size: { korangar_interface::theme::theme().collapsable().icon_size() },
        gaps: { korangar_interface::theme::theme().collapsable().gaps() },
        border: { korangar_interface::theme::theme().collapsable().border() },
        corner_diameter: { korangar_interface::theme::theme().collapsable().corner_diameter() },
        title_height: { korangar_interface::theme::theme().collapsable().title_height() },
        font_size: { korangar_interface::theme::theme().collapsable().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().collapsable().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().collapsable().vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().collapsable().overflow_behavior() },
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
pub fn field(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::field::Field, {
        text: !,
        tooltip: { "" },
        foreground_color: { korangar_interface::theme::theme().field().foreground_color() },
        background_color: { korangar_interface::theme::theme().field().background_color() },
        height: { korangar_interface::theme::theme().field().height() },
        corner_diameter: { korangar_interface::theme::theme().field().corner_diameter() },
        font_size: { korangar_interface::theme::theme().field().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().field().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().field().vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().field().overflow_behavior() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn split(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::split::Split, {
        gaps: { 0.0 },
        children: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn scroll_view(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::scroll_view::ScrollView, {
        children: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn text_box(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(korangar_interface::components::text_box::TextBox, {
        ghost_text: !,
        state: !,
        input_handler: !,
        hidable: { false },
        foreground_color: { korangar_interface::theme::theme().text_box().foreground_color() },
        background_color: { korangar_interface::theme::theme().text_box().background_color() },
        hovered_foreground_color: { korangar_interface::theme::theme().text_box().hovered_foreground_color() },
        hovered_background_color: { korangar_interface::theme::theme().text_box().hovered_background_color() },
        focused_foreground_color: { korangar_interface::theme::theme().text_box().focused_foreground_color() },
        focused_background_color: { korangar_interface::theme::theme().text_box().focused_background_color() },
        ghost_foreground_color: { korangar_interface::theme::theme().text_box().ghost_foreground_color() },
        hide_icon_color: { korangar_interface::theme::theme().text_box().hide_icon_color() },
        hovered_hide_icon_color: { korangar_interface::theme::theme().text_box().hovered_hide_icon_color() },
        height: { korangar_interface::theme::theme().text_box().height() },
        corner_diameter: { korangar_interface::theme::theme().text_box().corner_diameter() },
        font_size: { korangar_interface::theme::theme().text_box().font_size() },
        horizontal_alignment: { korangar_interface::theme::theme().text_box().horizontal_alignment() },
        vertical_alignment: { korangar_interface::theme::theme().text_box().vertical_alignment() },
        overflow_behavior: { korangar_interface::theme::theme().text_box().overflow_behavior() },
        focus_id: !,
    });

    macro_impl(token_stream.into()).into()
}
