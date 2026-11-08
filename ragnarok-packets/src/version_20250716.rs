//! Live Turoran MAIN 20250716 (Renewal), with three forge options.
use ragnarok_bytes::{ByteConvertable, FixedByteSize};

use crate::*;
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EquippableItemInformation {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8,
    pub option_data: [ItemOptions; 5], // fix count
    pub forge_data: [ItemOptions; 3],
    pub refinement_level: u8,
    pub enchantment_level: u8,
    pub flags: EquippableItemFlags,
}
impl From<EquippableItemInformation> for crate::EquippableItemInformation {
    fn from(p: EquippableItemInformation) -> Self {
        Self {
            index: p.index,
            item_id: p.item_id,
            item_type: p.item_type,
            equip_position: p.equip_position,
            equipped_position: p.equipped_position,
            slot: p.slot,
            hire_expiration_date: p.hire_expiration_date,
            bind_on_equip_type: p.bind_on_equip_type,
            w_item_sprite_number: p.w_item_sprite_number,
            option_count: p.option_count,
            option_data: p.option_data,
            refinement_level: p.refinement_level,
            enchantment_level: p.enchantment_level,
            flags: p.flags,
        }
    }
}
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B39)]
#[variable_length]
pub struct EquippableItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation>,
}
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B41)]
pub struct ItemPickupPacket {
    pub index: InventoryIndex,
    pub quantity: u16,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub is_broken: u8,
    pub cards: [u32; 4],
    pub equip_position: EquipPosition,
    pub item_type: u8,
    pub result: ItemPickupResult,
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub option_data: [ItemOptions; 5], // fix count
    pub forge_data: [ItemOptions; 3],
    pub favorite: u8,
    pub look: u16,
    pub refinement_level: u8,
    pub enchantment_level: u8,
}
impl From<ItemPickupPacket> for crate::ItemPickupPacket {
    fn from(p: ItemPickupPacket) -> Self {
        Self {
            index: p.index,
            quantity: p.quantity,
            item_id: p.item_id,
            is_identified: p.is_identified,
            is_broken: p.is_broken,
            cards: p.cards,
            equip_position: p.equip_position,
            item_type: p.item_type,
            result: p.result,
            hire_expiration_date: p.hire_expiration_date,
            bind_on_equip_type: p.bind_on_equip_type,
            option_data: p.option_data,
            favorite: p.favorite,
            look: p.look,
            refinement_level: p.refinement_level,
            enchantment_level: p.enchantment_level,
        }
    }
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0BE9)]
pub struct RequestEmotionPacket {
    #[new_default]
    pub group: u16,
    pub emotion: u16,
}
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0BEA)]
pub struct DisplayExpandedEmotionPacket {
    pub entity_id: EntityId,
    pub group: u16,
    pub emotion: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0111)]
pub struct AddSkillPacket {
    pub skill_information: SkillInformation,
}

#[cfg(test)]
mod tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;
    #[test]
    fn equipment_and_pickup_preserve_forge_refinement_and_grade() {
        assert_eq!(EquippableItemInformation::size_in_bytes(), 83);
        let mut bytes = vec![0x39, 0x0B, 88, 0, 0];
        let mut item = [0u8; 83];
        item[0] = 2;
        item[65..67].copy_from_slice(&123u16.to_le_bytes());
        item[80] = 9;
        item[81] = 4;
        bytes.extend_from_slice(&item);
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = EquippableItemListPacket::packet_from_bytes(&mut reader).unwrap();
        assert!(reader.is_empty());
        assert_eq!(packet.item_information[0].forge_data[0].index, 123);
        assert_eq!(packet.item_information[0].refinement_level, 9);
        assert_eq!(packet.item_information[0].enchantment_level, 4);
        let mut bytes = [0u8; 85];
        bytes[..2].copy_from_slice(&0x0B41u16.to_le_bytes());
        bytes[2] = 2;
        bytes[65..67].copy_from_slice(&456u16.to_le_bytes());
        bytes[83] = 11;
        bytes[84] = 3;
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = ItemPickupPacket::packet_from_bytes(&mut reader).unwrap();
        assert!(reader.is_empty());
        assert_eq!(packet.forge_data[0].index, 456);
        assert_eq!(packet.refinement_level, 11);
        assert_eq!(packet.enchantment_level, 3);
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), bytes);
    }
    #[test]
    fn modern_emotes_keep_six_byte_request_and_ten_byte_response() {
        let mut writer = ByteWriter::new();
        RequestEmotionPacket::new(300).packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), [0xE9, 0x0B, 0, 0, 0x2C, 1]);
        let mut reader = ByteReader::without_metadata(&[0xEA, 0x0B, 123, 0, 0, 0, 0, 0, 0x2C, 1]);
        let packet = DisplayExpandedEmotionPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(packet.emotion, 300);
        assert!(reader.is_empty());
    }
    #[test]
    fn live_character_and_map_handshakes_use_modern_widths() {
        assert_eq!(crate::CreateCharacterSuccessPacket::HEADER, PacketHeader(0x0B6F));
        assert_eq!(CharacterInformation::size_in_bytes(), 175);
        let mut writer = ByteWriter::new();
        crate::MapServerLoginPacket::new(AccountId(1), CharacterId(2), 3, ClientTick(4), Sex::Male)
            .packet_to_bytes(&mut writer)
            .unwrap();
        assert_eq!(writer.as_slice().len(), 23);
        assert_eq!(&writer.as_slice()[19..], &[0u8; 4]);
    }
}
