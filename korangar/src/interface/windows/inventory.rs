use korangar_components::item_box;
use korangar_interface::window::{CustomWindow, WindowTrait};
use korangar_networking::InventoryItem;
use rust_state::{Path, VecIndexExt};

use crate::ItemSource;
use crate::interface::windows::WindowClass;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
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

    fn to_window<'a>(self) -> impl WindowTrait<ClientState> + 'a {
        use korangar_interface::prelude::*;

        // TODO: Probably this should be more dynamic
        const INVENTORY_ROWS: usize = 4;
        const INVENTORY_COLUMNS: usize = 10;

        window! {
            title: "Inventory",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
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
