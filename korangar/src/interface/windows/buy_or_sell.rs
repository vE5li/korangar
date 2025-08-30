use korangar_interface::window::{CustomWindow, Window};
use ragnarok_packets::{BuyOrSellOption, ShopId};

use super::WindowClass;
use crate::input::InputEvent;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;

pub struct BuyOrSellWindow {
    shop_id: ShopId,
}

impl BuyOrSellWindow {
    pub fn new(shop_id: ShopId) -> Self {
        Self { shop_id }
    }
}

impl CustomWindow<ClientState> for BuyOrSellWindow {
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::BuyOrSell)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Buy or sell",
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            elements: (
                button! {
                    text: "Buy",
                    event: InputEvent::BuyOrSell {
                        shop_id: self.shop_id,
                        buy_or_sell: BuyOrSellOption::Buy,
                    },
                },
                button! {
                    text: "Sell",
                    event: InputEvent::BuyOrSell {
                        shop_id: self.shop_id,
                        buy_or_sell: BuyOrSellOption::Sell,
                    },
                },
            ),
        }
    }
}
