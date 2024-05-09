use ragnarok_packets::{EquipPosition, ItemId, ItemIndex};

#[derive(Debug, Clone)]
pub struct InventoryItem {
    pub index: ItemIndex,
    pub id: ItemId,
    pub is_identified: bool,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
}
