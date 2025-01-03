use derive_new::new;
use korangar_interface::elements::{ElementWrap, ScrollView};
use korangar_interface::size_bound;
use korangar_interface::state::PlainTrackedState;
use korangar_interface::windows::{PrototypeWindow, Window, WindowBuilder};
use korangar_networking::SellItem;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::SellCartContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::world::ResourceMetadata;

#[derive(new)]
pub struct SellCartWindow {
    cart: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>>,
}

impl SellCartWindow {
    pub const WINDOW_CLASS: &'static str = "sell_cart";
}

impl PrototypeWindow<InterfaceSettings> for SellCartWindow {
    fn window_class(&self) -> Option<&str> {
        Self::WINDOW_CLASS.into()
    }

    fn to_window(
        &self,
        window_cache: &WindowCache,
        application: &InterfaceSettings,
        available_space: ScreenSize,
    ) -> Window<InterfaceSettings> {
        let elements = vec![SellCartContainer::new(self.cart.clone()).wrap()];
        let elements = vec![ScrollView::new(elements, size_bound!(100%, ? < super)).wrap()];

        WindowBuilder::new()
            .with_title("Cart".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 60%))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
