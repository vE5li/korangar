use derive_new::new;
use korangar_interface::element::{ElementWrap, ScrollView};
use korangar_interface::size_bound;
use korangar_interface::state::{PlainRemote, PlainTrackedState};
use korangar_interface::window::{StateWindow, Window, WindowBuilder};
use korangar_networking::ShopItem;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::BuyContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::world::ResourceMetadata;

#[derive(new)]
pub struct BuyWindow {
    items: PlainRemote<Vec<ShopItem<ResourceMetadata>>>,
    cart: PlainTrackedState<Vec<ShopItem<(ResourceMetadata, u32)>>>,
}

impl BuyWindow {
    pub const WINDOW_CLASS: &'static str = "shop";
}

impl StateWindow<InterfaceSettings> for BuyWindow {
    fn window_class(&self) -> Option<&str> {
        Some(Self::WINDOW_CLASS)
    }

    fn to_window(
        &self,
    ) -> Window<InterfaceSettings> {
        let elements = vec![BuyContainer::new(self.items.clone(), self.cart.clone()).wrap()];
        let elements = vec![ScrollView::new(elements, size_bound!(100%, ? < super)).wrap()];

        WindowBuilder::new()
            .with_title("Buy".to_string())
            .with_class(Self::WINDOW_CLASS.to_string())
            .with_size_bound(size_bound!(300 > 400 < 500, ? < 60%))
            .with_elements(elements)
            .build(window_cache, application, available_space)
    }
}
