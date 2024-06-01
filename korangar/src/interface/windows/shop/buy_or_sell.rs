use derive_new::new;
use korangar_interface::elements::{ButtonBuilder, ElementWrap};
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_interface::{dimension_bound, size_bound};
use ragnarok_packets::{BuyOrSellOption, ShopId};

use crate::input::UserEvent;
use crate::interface::application::InterfaceSettings;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;

#[derive(new)]
pub struct BuyOrSellWindow {
    shop_id: ShopId,
}

impl BuyOrSellWindow {
    pub const WINDOW_CLASS: &'static str = "buy_or_sell";
}

impl PrototypeWindow<InterfaceSettings> for BuyOrSellWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![
            ButtonBuilder::new()
                .with_text("Buy")
                .with_event(UserEvent::BuyOrSell {
                    shop_id: self.shop_id,
                    buy_or_sell: BuyOrSellOption::Buy,
                })
                .with_width_bound(dimension_bound!(50%))
                .build()
                .wrap(),
            ButtonBuilder::new()
                .with_text("Sell")
                .with_event(UserEvent::BuyOrSell {
                    shop_id: self.shop_id,
                    buy_or_sell: BuyOrSellOption::Sell,
                })
                .with_width_bound(dimension_bound!(!))
                .build()
                .wrap(),
        ];

        WindowBuilder::new()
            .with_title("Buy or sell".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 60%))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
