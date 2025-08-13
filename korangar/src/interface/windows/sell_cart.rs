use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::SellItem;
use rust_state::Path;

use super::WindowClass;
use crate::input::InputEvent;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

pub struct SellCartWindow<A> {
    items_path: A,
}

impl<A> SellCartWindow<A> {
    pub fn new(items_path: A) -> Self {
        Self { items_path }
    }
}

impl<A> CustomWindow<ClientState> for SellCartWindow<A>
where
    A: Path<ClientState, Vec<SellItem<(ResourceMetadata, u16)>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::SellCart)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Cart",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            resizable: true,
            elements: (
                button! {
                    text: "Cancel",
                    event: InputEvent::CloseShop,
                },
            ),
        }
    }
}
