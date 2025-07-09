use korangar_interface::prelude::create_component_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn character_slot_preview(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(crate::interface::components::character_slot_preview::CharacterSlotPreview, {
        character_information: !,
        switch_request: !,
        background_color: { crate::graphics::Color::monochrome_u8(80) },
        click_handler: !,
        overlay_handler: !,
        slot: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn item_box(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(crate::interface::components::item_box::ItemBox, {
        item_path: !,
        source: !,
        amount_display: { const crate::interface::components::item_box::AmountDisplay::default() },
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn skill_box(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(crate::interface::components::skill_box::SkillBox, {
        skill_path: !,
        source: !,
        level_display: { const crate::interface::components::skill_box::LevelDisplay::default() },
    });

    macro_impl(token_stream.into()).into()
}
