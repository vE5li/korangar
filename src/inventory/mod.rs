use vulkano::sync::GpuFuture;

use crate::graphics::Texture;
use crate::interface::TrackedState;
use crate::loaders::{GameFileLoader, ScriptLoader, TextureLoader};
use crate::network::{EquipPosition, ItemOptions};

enum ItemDetails {
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
}

#[derive(Clone, Debug)]
pub struct Item {
    pub index: u16,
    pub item_id: usize,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    //pub item_type: u8,
    //pub wear_state: u32,
    //pub slot: [u32; 4], // card ?
    //pub hire_expiration_date: i32,
    pub texture: Texture,
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
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        script_loader: &ScriptLoader,
        item_data: Vec<(usize, usize, EquipPosition, EquipPosition)>,
    ) {

        let items = item_data
            .into_iter()
            .map(|item_data| {

                let resource_name = script_loader.get_item_resource_from_id(item_data.1);
                let full_path = format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\{}.bmp", resource_name);
                let texture = texture_loader.get(&full_path, game_file_loader, texture_future).unwrap();
                Item {
                    index: item_data.0 as u16,
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
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        script_loader: &ScriptLoader,
        item_index: usize,
        item_data: usize,
        equip_position: EquipPosition,
        equipped_position: EquipPosition,
    ) {

        self.items.with_mut(|items, changed| {

            // set changed ahead of time since we might exit early
            changed();

            if let Some(_stack) = items.iter_mut().find(|item| item.item_id == item_data) {
                //stack.amount += item_data.amount;
                return;
            }

            let resource_name = script_loader.get_item_resource_from_id(item_data);
            let full_path = format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\item\\{}.bmp", resource_name);
            let texture = texture_loader.get(&full_path, game_file_loader, texture_future).unwrap();
            let item = Item {
                index: item_index as u16,
                item_id: item_data,
                equip_position,
                equipped_position,
                texture,
            };

            items.push(item);
        });
    }

    pub fn update_equipped_position(&mut self, index: u16, equipped_position: EquipPosition) {

        self.items.with_mut(|items, changed| {

            items.iter_mut().find(|item| item.index == index).unwrap().equipped_position = equipped_position;
            changed();
        });
    }

    pub fn get_item_state(&self) -> TrackedState<Vec<Item>> {
        self.items.clone()
    }
}
