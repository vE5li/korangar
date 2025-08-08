use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::SellItem;
use rust_state::Path;

use super::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

pub struct SellWindow<A, B> {
    items_path: A,
    cart_path: B,
}

impl<A, B> SellWindow<A, B> {
    pub fn new(items_path: A, cart_path: B) -> Self {
        Self { items_path, cart_path }
    }
}

impl<A, B> CustomWindow<ClientState> for SellWindow<A, B>
where
    A: Path<ClientState, Vec<SellItem<(ResourceMetadata, u16)>>>,
    B: Path<ClientState, Vec<SellItem<(ResourceMetadata, u16)>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Sell)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Sell",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            elements: (
            ),
        }
    }
}
