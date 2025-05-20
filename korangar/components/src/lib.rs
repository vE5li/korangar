use korangar_interface::prelude::create_component_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn character_slot_preview(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(crate::interface::components::character_slot_preview::CharacterSlotPreview, {
        path: !,
        background_color: { crate::graphics::Color::monochrome_u8(80) },
        click_handler: !,
        slot: !,
    });

    macro_impl(token_stream.into()).into()
}
