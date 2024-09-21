mod hotbar;
mod skills;

use std::cell::Ref;

use korangar_interface::state::{PlainRemote, PlainTrackedState, TrackedState, TrackedStateExt, ValueState};
use korangar_networking::{InventoryItem, InventoryItemDetails, NoMetadata};
use ragnarok_packets::{EquipPosition, InventoryIndex};

pub use self::hotbar::Hotbar;
pub use self::skills::{Skill, SkillTree};
use crate::loaders::{ResourceMetadata, ScriptLoader, TextureLoader};

#[derive(Default)]
pub struct Inventory {
    items: PlainTrackedState<Vec<InventoryItem<ResourceMetadata>>>,
}

impl Inventory {
    pub fn fill(&mut self, texture_loader: &mut TextureLoader, script_loader: &ScriptLoader, items: Vec<InventoryItem<NoMetadata>>) {
        let items = items
            .into_iter()
            .map(|item| script_loader.load_inventory_item_metadata(texture_loader, item))
            .collect();

        self.items.set(items);
    }

    pub fn add_item(&mut self, texture_loader: &mut TextureLoader, script_loader: &ScriptLoader, item: InventoryItem<NoMetadata>) {
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
                let item = script_loader.load_inventory_item_metadata(texture_loader, item);

                items.push(item);
            }

            ValueState::Mutated(())
        });
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
