mod hotbar;
mod skills;

use std::cell::Ref;
use std::sync::Arc;

use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState, TrackedStateExt, ValueState};
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::{EquipPosition, InventoryIndex, ItemId};

pub use self::hotbar::Hotbar;
pub use self::skills::{Skill, SkillTree};
use crate::graphics::Texture;
use crate::loaders::AsyncLoader;
use crate::world::{Library, ResourceMetadata};

#[derive(Default)]
pub struct Inventory {
    items: PlainTrackedState<Vec<InventoryItem<ResourceMetadata>>>,
}

impl Inventory {
    pub fn fill(&mut self, async_loader: &AsyncLoader, library: &Library, items: Vec<InventoryItem<NoMetadata>>) {
        let items = items
            .into_iter()
            .map(|item| library.load_inventory_item_metadata(async_loader, item))
            .collect();

        self.items.set(items);
    }

    pub fn add_item(&mut self, async_loader: &AsyncLoader, library: &Library, item: InventoryItem<NoMetadata>) {
        self.items.with_mut(|items| {
            if let Some(found_item) = items.iter_mut().find(|inventory_item| inventory_item.index == item.index) {
                let InventoryItemDetails::Regular { amount, .. } = &mut found_item.details else {
                    panic!();
                };

                let InventoryItemDetails::Regular { amount: added_amount, .. } = item.details else {
                    panic!();
                };

                *amount += added_amount;
            } else {
                let item = library.load_inventory_item_metadata(async_loader, item);

                items.push(item);
            }

            ValueState::Mutated(())
        });
    }

    pub fn update_item_sprite(&mut self, item_id: ItemId, texture: Arc<Texture>) {
        self.items.with_mut(|items| {
            items.iter_mut().filter(|item| item.item_id == item_id).for_each(|item| {
                item.metadata.texture = Some(texture.clone());
            });

            ValueState::Mutated(())
        })
    }

    pub fn remove_item(&mut self, index: InventoryIndex, remove_amount: u16) {
        self.items.with_mut(|items| {
            let position = items.iter().position(|item| item.index == index).expect("item not in inventory");

            if let InventoryItemDetails::Regular { amount, .. } = &mut items[position].details {
                if *amount > remove_amount {
                    *amount -= remove_amount;
                    return ValueState::Mutated(());
                }
            }

            items.remove(position);

            ValueState::Mutated(())
        });
    }

    pub fn update_equipped_position(&mut self, index: InventoryIndex, new_equipped_position: EquipPosition) {
        self.items.mutate(|items| {
            let item = items.iter_mut().find(|item| item.index == index).unwrap();
            let InventoryItemDetails::Equippable { equipped_position, .. } = &mut item.details else {
                panic!();
            };

            *equipped_position = new_equipped_position;
        });
    }

    pub fn get_items(&self) -> Ref<'_, Vec<InventoryItem<ResourceMetadata>>> {
        self.items.get()
    }

    pub fn item_remote(&self) -> PlainRemote<Vec<InventoryItem<ResourceMetadata>>> {
        self.items.new_remote()
    }
}
