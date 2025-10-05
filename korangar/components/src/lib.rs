use korangar_interface::prelude::create_component_macro;
use proc_macro::TokenStream;

#[proc_macro]
pub fn item_box(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(crate::interface::components::item_box::ItemBox, {
        item_path: !,
        source: !,
    });

    macro_impl(token_stream.into()).into()
}

#[proc_macro]
pub fn skill_box(token_stream: TokenStream) -> TokenStream {
    create_component_macro!(crate::interface::components::skill_box::SkillBox, {
        learnable_skill_path: !,
        learned_skill_path: !,
        source: !,
    });

    macro_impl(token_stream.into()).into()
}
