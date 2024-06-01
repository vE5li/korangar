use korangar_networking::InventoryItem;
use ragnarok_packets::{EquipPosition, HotbarSlot};

use crate::inventory::Skill;
use crate::loaders::ResourceMetadata;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemSource {
    Inventory,
    Equipment { position: EquipPosition },
}

#[derive(Debug, Clone)]
pub struct ItemMove {
    pub source: ItemSource,
    pub destination: ItemSource,
    pub item: InventoryItem<ResourceMetadata>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSource {
    SkillTree,
    Hotbar { slot: HotbarSlot },
}

#[derive(Clone, Debug)]
pub struct SkillMove {
    pub source: SkillSource,
    pub destination: SkillSource,
    pub skill: Skill,
}

#[derive(Clone, Debug)]
pub enum PartialMove {
    Item {
        source: ItemSource,
        item: InventoryItem<ResourceMetadata>,
    },
    Skill {
        source: SkillSource,
        skill: Skill,
    },
}

#[derive(Clone, Debug)]
pub enum Move {
    Item {
        source: ItemSource,
        destination: ItemSource,
        item: InventoryItem<ResourceMetadata>,
    },
    Skill {
        source: SkillSource,
        destination: SkillSource,
        skill: Skill,
    },
}
