use korangar_interface::element::Element;
use korangar_interface::window::{CustomWindow, Window};
use korangar_networking::{InventoryItem, InventoryItemDetails};
use ragnarok_packets::EquipPosition;
use rust_state::{Path, Selector};

use crate::ItemSource;
use crate::interface::windows::WindowClass;
use crate::loaders::OverflowBehavior;
use crate::state::ClientState;
use crate::state::theme::InterfaceThemeType;
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

impl<P> Selector<ClientState, InventoryItem<ResourceMetadata>, false> for EquipmentPath<P>
where
    P: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a InventoryItem<ResourceMetadata>> {
        self.follow(state)
    }
}

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

pub struct EquipmentWindow<A> {
    items_path: A,
}

impl<A> EquipmentWindow<A> {
    pub fn new(items_path: A) -> Self {
        Self { items_path }
    }
}

impl<A> CustomWindow<ClientState> for EquipmentWindow<A>
where
    A: Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
{
    fn window_class() -> Option<WindowClass> {
        Some(WindowClass::Equipment)
    }

    fn to_window<'a>(self) -> impl Window<ClientState> + 'a {
        use korangar_interface::prelude::*;

        fn equip_box(
            items_path: impl Path<ClientState, Vec<InventoryItem<ResourceMetadata>>>,
            equip_position: EquipPosition,
        ) -> impl Element<ClientState> {
            use korangar_components::item_box;
            use korangar_interface::prelude::*;

            let equipment_path = EquipmentPath {
                equip_position,
                path: items_path,
            };

            let display_name = match equip_position {
                _ if equip_position.contains(EquipPosition::HEAD_LOWER) => "Head lower",
                _ if equip_position.contains(EquipPosition::HEAD_MIDDLE) => "Head middle",
                _ if equip_position.contains(EquipPosition::HEAD_TOP) => "Head top",
                _ if equip_position.contains(EquipPosition::RIGHT_HAND) => "Right hand",
                _ if equip_position.contains(EquipPosition::LEFT_HAND) => "Left hand",
                _ if equip_position.contains(EquipPosition::ARMOR) => "Armor",
                _ if equip_position.contains(EquipPosition::SHOES) => "Shoes",
                _ if equip_position.contains(EquipPosition::GARMENT) => "Garment",
                _ if equip_position.contains(EquipPosition::LEFT_ACCESSORY) => "Left accessory",
                _ if equip_position.contains(EquipPosition::RIGTH_ACCESSORY) => "Right accessory",
                _ if equip_position.contains(EquipPosition::COSTUME_HEAD_TOP) => "Costume head top",
                _ if equip_position.contains(EquipPosition::COSTUME_HEAD_MIDDLE) => "Costume head middle",
                _ if equip_position.contains(EquipPosition::COSTUME_HEAD_LOWER) => "Costume head lower",
                _ if equip_position.contains(EquipPosition::COSTUME_GARMENT) => "Costume garment",
                _ if equip_position.contains(EquipPosition::AMMO) => "Ammo",
                _ if equip_position.contains(EquipPosition::SHADOW_ARMOR) => "Shadow ammo",
                _ if equip_position.contains(EquipPosition::SHADOW_WEAPON) => "Shadow weapon",
                _ if equip_position.contains(EquipPosition::SHADOW_SHIELD) => "Shadow shield",
                _ if equip_position.contains(EquipPosition::SHADOW_SHOES) => "Shadow shoes",
                _ if equip_position.contains(EquipPosition::SHADOW_RIGHT_ACCESSORY) => "Shadow right accessory",
                _ if equip_position.contains(EquipPosition::SHADOW_LEFT_ACCESSORY) => "Shadow left accessory",
                _ if equip_position.contains(EquipPosition::LEFT_RIGHT_ACCESSORY) => "Accessory",
                _ if equip_position.contains(EquipPosition::LEFT_RIGHT_HAND) => "Two hand weapon",
                _ if equip_position.contains(EquipPosition::SHADOW_LEFT_RIGHT_ACCESSORY) => "Shadow accessory",
                _ => panic!("no display name for equip position"),
            };

            split! {
                gaps: theme().window().gaps(),
                children: (
                    item_box! {
                        item_path: equipment_path,
                        source: ItemSource::Equipment { position: equip_position },
                    },
                    text! {
                        text: display_name,
                        // Get this height from the skill box theme.
                        height: 40.0,
                        overflow_behavior: OverflowBehavior::Shrink,
                    }
                ),
            }
        }

        window! {
            title: "Equipment",
            class: Self::window_class(),
            theme: InterfaceThemeType::Game,
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
