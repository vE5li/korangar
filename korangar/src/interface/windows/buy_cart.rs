use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::ShopItem;
use rust_state::Path;

use super::WindowClass;
use crate::input::InputEvent;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
use crate::world::ResourceMetadata;

pub struct BuyCartWindow<A> {
    cart_path: A,
}

impl<A> BuyCartWindow<A> {
    pub fn new(cart_path: A) -> Self {
        Self { cart_path }
    }
}

impl<A> CustomWindow<ClientState> for BuyCartWindow<A>
where
    A: Path<ClientState, Vec<ShopItem<(ResourceMetadata, u32)>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BuyCart)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Cart",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
            resizable: true,
            elements: (
                split! {
                    gaps: theme().window().gaps(),
                    children: (
                        button! {
                            text: "Buy",
                            event: InputEvent::BuyItems { items: Vec::new() },
                        },
                        button! {
                            text: "Cancel",
                            event: InputEvent::CloseShop,
                        },
                    ),
                },
            ),
        }
    }
}
