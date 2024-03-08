mod hotbar;
mod skills;

use std::sync::Arc;

use vulkano::image::view::ImageView;

pub use self::hotbar::Hotbar;
pub use self::skills::{Skill, SkillTree};
use crate::interface::{Remote, TrackedState, ValueState};
use crate::loaders::{GameFileLoader, ScriptLoader, TextureLoader};
use crate::network::{EquipPosition, ItemId, ItemIndex};

/*enum ItemDetails {
    Regular {
        amount: u16,
        fags: u8, // bit 1 - is_identified; bit 2 - place_in_etc_tab;
    },

    Equippable {
        location: u32,
        bind_on_equip_type: u16,
        w_item_sprite_number: u16,
        option_count: u8,
        option_data: [ItemOptions; 5], // fix count
        refinement_level: u8,
        enchantment_level: u8,
        fags: u8, // bit 1 - is_identified; bit 2 - is_damaged; bit 3 - place_in_etc_tab
    },
}*/

#[derive(Clone, Debug)]
pub struct Item {
    pub index: ItemIndex,
    pub item_id: ItemId,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    //pub item_type: u8,
    //pub wear_state: u32,
    //pub slot: [u32; 4], // card ?
    //pub hire_expiration_date: i32,
    pub texture: Arc<ImageView>,
}

#[derive(Default)]
pub struct Inventory {
    items: TrackedState<Vec<Item>>,
}

impl Inventory {
    pub fn fill(
        &mut self,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        script_loader: &ScriptLoader,
        item_data: Vec<(ItemIndex, ItemId, EquipPosition, EquipPosition)>,
    ) {
        let items = item_data
            .into_iter()
            .map(|item_data| {
                let resource_name = script_loader.get_item_resource_from_id(item_data.1);
                let full_path = format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\{resource_name}.bmp");
                let texture = texture_loader.get(&full_path, game_file_loader).unwrap();
                Item {
                    index: item_data.0,
                    item_id: item_data.1,
                    equip_position: item_data.2,
                    equipped_position: item_data.3,
                    texture,
                }
            })
            .collect();

        self.items.set(items);
    }

    pub fn add_item(
        &mut self,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        script_loader: &ScriptLoader,
        item_index: ItemIndex,
        item_id: ItemId,
        equip_position: EquipPosition,
        equipped_position: EquipPosition,
    ) {
        self.items.with_mut(|items| {
            if let Some(_stack) = items.iter_mut().find(|item| item.item_id == item_id) {
                //stack.amount += item_data.amount;
                return ValueState::Mutated(());
            }

            let resource_name = script_loader.get_item_resource_from_id(item_id);
            let full_path = format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\{resource_name}.bmp");
            let texture = texture_loader.get(&full_path, game_file_loader).unwrap();
            let item = Item {
                index: item_index,
                item_id,
                equip_position,
                equipped_position,
                texture,
            };

            items.push(item);

            ValueState::Mutated(())
        });
    }

    pub fn update_equipped_position(&mut self, index: ItemIndex, equipped_position: EquipPosition) {
        self.items.with_mut(|items| {
            items.iter_mut().find(|item| item.index == index).unwrap().equipped_position = equipped_position;
            ValueState::Mutated(())
        });
    }

    pub fn get_items(&self) -> Remote<Vec<Item>> {
        self.items.new_remote()
    }
}
