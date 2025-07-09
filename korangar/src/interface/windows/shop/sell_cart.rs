use korangar_interface::element::{Element, StateElement};
use korangar_interface::window::{CustomWindow, StateWindow, Window};
use korangar_networking::SellItem;
use rust_state::RustState;

use crate::interface::application::InterfaceSettings;
use crate::interface::elements::SellCartContainer;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::WindowCache;
use crate::state::ClientState;
use crate::world::ResourceMetadata;

#[derive(Default, RustState, StateWindow)]
#[window_class("sell_cart")]
pub struct SellCartState {
    items: Vec<SellItem<(ResourceMetadata, u16)>>,
}

impl StateElement<ClientState> for SellCartState {
    fn to_element(self_path: impl rust_state::Path<ClientState, Self>, name: String) -> impl Element<ClientState> {
        todo!()
    }
}
