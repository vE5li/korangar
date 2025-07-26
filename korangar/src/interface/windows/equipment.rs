use korangar_components::item_box;
use korangar_interface::element::Element;
use korangar_interface::window::{CustomWindow, StateWindow, Window, WindowTrait};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use ragnarok_packets::EquipPosition;
use rust_state::{Path, Selector};

use crate::ItemSource;
use crate::interface::layout::ScreenSize;
use crate::interface::windows::{WindowCache, WindowClass};
use crate::state::{ClientState, ClientThemeType};
use crate::world::ResourceMetadata;

struct EquipmentPath<P> {
    equip_position: EquipPosition,
    path: P,
}

impl<P: Copy> Clone for EquipmentPath<P> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<P: Copy> Copy for EquipmentPath<P> {}

impl<P> Path<ClientState, InventoryItem<ResourceMetadata>, false> for EquipmentPath<P>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        // SAFETY:
        //
        // It is safe to unwrap here since its guaranteed to be `Some` by the bounds.
        self.path.follow(state).unwrap().iter().find(|item| {
            if let InventoryItemDetails::Equippable { equipped_position, .. } = &item.details {
                return equipped_position.contains(self.equip_position);
            }

            false
        })
    }

    fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut InventoryItem<ResourceMetadata>> {
        // SAFETY:
        //
        // It is safe to unwrap here since its guaranteed to be `Some` by the bounds.
        self.path.follow_mut(state).unwrap().iter_mut().find(|item| {
            if let InventoryItemDetails::Equippable { equipped_position, .. } = item.details {
                return equipped_position.contains(self.equip_position);
            }

            false
        })
    }
}

impl<P> Selector<ClientState, InventoryItem<ResourceMetadata>, false> for EquipmentPath<P>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        self.follow(state)
    }
}

pub struct EquipmentWindow<P> {
    items_path: P,
}

impl<P> EquipmentWindow<P> {
    pub fn new(items_path: P) -> Self {
        Self { items_path }
    }
}

fn equip_box(
    items_path: impl Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
    equip_position: EquipPosition,
) -> impl Element<ClientState> {
    item_box! {
        item_path: EquipmentPath {
            equip_position,
            path: items_path,
        },
        source: ItemSource::Equipment { position: equip_position },
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
            elements: [
                equip_box(self.items_path, EquipPosition::HEAD_TOP),
                equip_box(self.items_path, EquipPosition::HEAD_MIDDLE),
                equip_box(self.items_path, EquipPosition::HEAD_LOWER),
                equip_box(self.items_path, EquipPosition::ARMOR),
                equip_box(self.items_path, EquipPosition::GARMENT),
                equip_box(self.items_path, EquipPosition::SHOES),
                equip_box(self.items_path, EquipPosition::LEFT_HAND),
                equip_box(self.items_path, EquipPosition::RIGHT_HAND),
                equip_box(self.items_path, EquipPosition::AMMO),
            ],
        }
    }
}
