use korangar_interface::window::{CustomWindow, PrototypeWindow, Window, WindowTrait};
use korangar_networking::InventoryItem;
use rust_state::Path;

use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};
use crate::world::ResourceMetadata;

pub struct EquipmentWindow<P> {
    items_path: P,
}

impl<P> EquipmentWindow<P> {
    pub fn new(items_path: P) -> Self {
        Self { items_path }
    }
}

impl<P> CustomWindow<ClientState> for EquipmentWindow<P>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Equipment)
    }

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        window! {
            title: "Equipment",
            class: Self::window_class(),
            theme: ClientThemeType::Game,
            closable: true,
            elements: ()
        }
    }
}
