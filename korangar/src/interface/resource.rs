use korangar_networking::InventoryItem;
use ragnarok_packets::{EquipPosition, HotbarSlot};

use crate::inventory::Skill;
use crate::world::ResourceMetadata;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemSource {
    Inventory,
    Equipment { position: EquipPosition },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSource {
    SkillTree,
    Hotbar { slot: HotbarSlot },
}
