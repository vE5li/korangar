use derive_new::new;
use korangar_interface::element::{ElementWrap, ScrollView};
use korangar_interface::size_bound;
use korangar_interface::state::PlainTrackedState;
use korangar_interface::window::{PrototypeWindow, Window, WindowBuilder};
use korangar_networking::ShopItem;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::BuyCartContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::world::ResourceMetadata;

#[derive(new)]
pub struct BuyCartWindow {
    cart: PlainTrackedState<Vec<ShopItem<(ResourceMetadata, u32)>>>,
}

impl BuyCartWindow {
    pub const WINDOW_CLASS: &'static str = "buy_cart";
}

impl PrototypeWindow<InterfaceSettings> for BuyCartWindow {
    fn window_class(&self) -> Option<&str> {
        Some(Self::WINDOW_CLASS)
    }

    fn to_window(
        &self,
    ) -> Window<InterfaceSettings> {
        let elements = vec![BuyCartContainer::new(self.cart.clone()).wrap()];
        let elements = vec![ScrollView::new(elements, size_bound!(100%, ? < super)).wrap()];

        WindowBuilder::new()
            .with_title("Cart".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 60%))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
