use derive_new::new;
use korangar_interface::elements::{ElementWrap, ScrollView};
use korangar_interface::size_bound;
use korangar_interface::state::PlainTrackedState;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_networking::ShopItem;
use rust_state::Context;

use crate::interface::elements::BuyCartContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::loaders::ResourceMetadata;
use crate::GameState;

#[derive(Default)]
pub struct BuyCartWindow;

impl BuyCartWindow {
    pub const WINDOW_CLASS: &'static str = "buy_cart";
}

impl PrototypeWindow<GameState> for BuyCartWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(&self, window_cache: &WindowCache, application: &Context<GameState>, available_space: ScreenSize) -> Window<GameState> {
        let elements = vec![BuyCartContainer::new().wrap()];
        let elements = vec![ScrollView::new(elements, size_bound!(100%, ? < super)).wrap()];

        WindowBuilder::new()
            .with_title("Cart".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 60%))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
