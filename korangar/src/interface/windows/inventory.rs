use korangar_components::item_box;
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::InventoryItem;
use rust_state::{Path, VecIndexExt};

use crate::ItemSource;
use crate::interface::windows::WindowClass;
use crate::state::localization::LocalizationPathExt;
use crate::state::theme::InterfaceThemeType;
use crate::state::{ClientState, ClientStatePathExt, client_state};
use crate::world::ResourceMetadata;

pub struct InventoryWindow<P> {
    items_path: P,
}

impl<P> InventoryWindow<P> {
    pub fn new(items_path: P) -> Self {
        Self { items_path }
    }
}

impl<P> CustomWindow<ClientState> for InventoryWindow<P>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Inventory)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // TODO: Probably this should be more dynamic
        const INVENTORY_ROWS: usize = 4;
        const INVENTORY_COLUMNS: usize = 10;

        window! {
            title: client_state().localization().inventory_window_title(),
            class: Self::window_class(),
            theme: InterfaceThemeType::InGame,
            closable: true,
            elements: std::array::from_fn::<_, INVENTORY_ROWS, _>(|row| {
                split! {
                    gaps: theme().window().gaps(),
                    children: std::array::from_fn::<_, INVENTORY_COLUMNS, _>(|column| {
                        let path = self.items_path.index(row * INVENTORY_COLUMNS + column);

                        item_box! {
                            item_path: path,
                            source: ItemSource::Inventory,
                        }
                    }),
                }
            }),
        }
    }
}
