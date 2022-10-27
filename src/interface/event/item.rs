use crate::inventory::Item;
use crate::network::EquipPosition;

#[derive(Clone, Copy, Debug)]
pub enum ItemSource {
    Inventory,
    Equipment { position: EquipPosition },
}

#[derive(Debug, Clone)]
pub struct ItemMove {
    pub source: ItemSource,
    pub destination: ItemSource,
    pub item: Item,
}
