//! Hercules 20190530 RE wire types.
//!
//! Turoran profile: Renewal 20190530 plus wide character HP/SP and three forge
//! options. Do not register 20220406 response handlers for this version by
//! aliasing them. Wire reference: Turor/hercules-prere-plus at
//! 3846d1b4d3c1d8bf5e3a739f43c2206cc27bd0ed,
//! src/map/packets_shuffle_re.h (20190530: sex at offset 18), cross-checked
//! with roBrowserLegacy 3239980f4a69a3146d1c96d44dcce7104ebdc274 CZ.ENTER2
//! builder.

use ragnarok_bytes::{ByteConvertable, FixedByteSize};
use ragnarok_macros::{ClientPacket, MapServer, Packet};

use crate::*;

/// The 19-byte CZ_ENTER2 request for the 20190530 RE profile.
///
/// The opcode is shared with newer clients, but their extra four-byte field
/// must not be sent here. Select the type from the configured packet version;
/// the opcode alone cannot distinguish the layouts. Session fields are secret:
/// do not log serialized handshakes (or this type's Debug representation).
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0436)]
pub struct MapServerLoginPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub login_id1: u32,
    pub client_tick: ClientTick,
    pub sex: Sex,
}

#[cfg(test)]
mod tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;
    use crate::PacketExt;

    // Synthetic values only; never use captured account/session data here.
    const GOLDEN: [u8; 19] = [
        0x36, 0x04, // opcode
        0x04, 0x03, 0x02, 0x01, // account ID
        0x14, 0x13, 0x12, 0x11, // character ID
        0x24, 0x23, 0x22, 0x21, // login ID 1
        0x34, 0x33, 0x32, 0x31, // client tick
        0x01, // male; offset 18, no extra 2022 field
    ];

    fn synthetic_packet(sex: Sex) -> MapServerLoginPacket {
        MapServerLoginPacket {
            account_id: AccountId(0x0102_0304),
            character_id: CharacterId(0x1112_1314),
            login_id1: 0x2122_2324,
            client_tick: ClientTick(0x3132_3334),
            sex,
        }
    }

    #[test]
    fn map_login_matches_20190530_wire_contract() {
        let mut writer = ByteWriter::new();
        let size = synthetic_packet(Sex::Male).packet_to_bytes(&mut writer).unwrap();
        assert_eq!(size, 19);
        assert_eq!(writer.as_slice(), GOLDEN);
    }

    #[test]
    fn female_sex_is_at_offset_18() {
        let mut writer = ByteWriter::new();
        synthetic_packet(Sex::Female).packet_to_bytes(&mut writer).unwrap();
        let mut expected = GOLDEN;
        expected[18] = 0;
        assert_eq!(writer.as_slice(), expected);
    }

    #[test]
    fn map_login_reads_the_2019_fields_without_padding() {
        let mut reader = ByteReader::without_metadata(&GOLDEN);
        let packet = MapServerLoginPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(packet.account_id, AccountId(0x0102_0304));
        assert_eq!(packet.character_id, CharacterId(0x1112_1314));
        assert_eq!(packet.login_id1, 0x2122_2324);
        assert_eq!(packet.client_tick.0, 0x3132_3334);
        assert!(matches!(packet.sex, Sex::Male));
    }

    #[test]
    fn every_truncated_handshake_is_rejected() {
        for length in 0..GOLDEN.len() {
            let mut reader = ByteReader::without_metadata(&GOLDEN[..length]);
            assert!(MapServerLoginPacket::packet_from_bytes(&mut reader).is_err(), "length {length}");
        }
    }

    #[test]
    fn mismatched_opcode_is_rejected() {
        let mut bytes = GOLDEN;
        bytes[0] ^= 1;
        let mut reader = ByteReader::without_metadata(&bytes);
        assert!(MapServerLoginPacket::packet_from_bytes(&mut reader).is_err());
    }

    #[test]
    fn existing_20220406_handshake_is_unchanged() {
        let packet = crate::MapServerLoginPacket {
            account_id: AccountId(1),
            character_id: CharacterId(2),
            login_id1: 3,
            client_tick: ClientTick(4),
            sex: Sex::Male,
            unknown: [0; 4],
        };
        let mut writer = ByteWriter::new();
        assert_eq!(packet.packet_to_bytes(&mut writer).unwrap(), 23);
        assert_eq!(writer.as_slice().len(), 23);
    }
}

// Shared definitions are reused only where the effective Turoran wire layout is
// identical. These are the divergent character and map/inventory messages.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x099D)]
#[variable_length]
pub struct RequestCharacterListSuccessPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation>,
}
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x006D)]
pub struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x08D5)]
pub struct SwitchCharacterSlotResponsePacket {
    #[new_default]
    pub unknown: u16,
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x035F)]
pub struct RequestPlayerMovePacket {
    pub position: WorldPosition,
}
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0366)]
pub struct UseSkillOnGroundPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_position: TilePosition,
}
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x00F3)]
#[variable_length]
pub struct GlobalMessagePacket {
    #[length_remaining_off_by_one]
    pub message: String,
}
#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct EquippableItemInformation {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    pub refinement_level: u8,
    pub slot: [u32; 4],
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8,
    pub option_data: [ItemOptions; 5],
    pub forge_data: [ItemOptions; 3],
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
            enchantment_level: 0,
            flags: p.flags,
        }
    }
}
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0B0A)]
#[variable_length]
pub struct EquippableItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation>,
}
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
#[header(0x0A37)]
pub struct ItemPickupPacket {
    pub index: InventoryIndex,
    pub quantity: u16,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub is_broken: u8,
    pub refinement_level: u8,
    pub cards: [u32; 4],
    pub equip_position: EquipPosition,
    pub item_type: u8,
    pub result: ItemPickupResult,
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub option_data: [ItemOptions; 5],
    pub forge_data: [ItemOptions; 3],
    pub favorite: u8,
    pub look: u16,
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
            enchantment_level: 0,
        }
    }
}

#[cfg(test)]
mod codec_tests {
    use ragnarok_bytes::{ByteReader, ByteWriter};

    use super::*;
    #[test]
    fn wide_character_response_headers_and_offsets() {
        assert_eq!(CharacterInformation::size_in_bytes(), 175);
        let mut bytes = vec![0u8; 177];
        bytes[..2].copy_from_slice(&0x006Du16.to_le_bytes());
        bytes[52..60].copy_from_slice(&(1i64 << 40).to_le_bytes());
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = CreateCharacterSuccessPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(packet.character_information.health_points, 1i64 << 40);
        assert!(reader.is_empty());
        let mut bytes = vec![0x9D, 0x09, 179, 0];
        bytes.extend_from_slice(&[0u8; 175]);
        let mut reader = ByteReader::without_metadata(&bytes);
        assert_eq!(
            RequestCharacterListSuccessPacket::packet_from_bytes(&mut reader)
                .unwrap()
                .character_information
                .len(),
            1
        );
        assert!(reader.is_empty());
    }
    #[test]
    fn equipment_retains_three_forge_options_before_flags() {
        assert_eq!(EquippableItemInformation::size_in_bytes(), 82);
        let mut bytes = vec![0x0A, 0x0B, 87, 0, 0];
        let mut item = [0u8; 82];
        item[0] = 2;
        item[15] = 9;
        item[66..68].copy_from_slice(&123u16.to_le_bytes());
        item[68..70].copy_from_slice(&456u16.to_le_bytes());
        item[70] = 7;
        bytes.extend_from_slice(&item);
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = EquippableItemListPacket::packet_from_bytes(&mut reader).unwrap();
        assert!(reader.is_empty());
        assert_eq!(packet.item_information[0].refinement_level, 9);
        assert_eq!(packet.item_information[0].forge_data[0].index, 123);
        assert_eq!(packet.item_information[0].forge_data[0].value, 456);
        assert_eq!(packet.item_information[0].forge_data[0].parameter, 7);
    }
    #[test]
    fn pickup_is_84_bytes_with_pre_card_refinement() {
        let mut bytes = [0u8; 84];
        bytes[..2].copy_from_slice(&0x0A37u16.to_le_bytes());
        bytes[2] = 2;
        bytes[12] = 11;
        bytes[66..68].copy_from_slice(&789u16.to_le_bytes());
        let mut reader = ByteReader::without_metadata(&bytes);
        let packet = ItemPickupPacket::packet_from_bytes(&mut reader).unwrap();
        assert_eq!(packet.refinement_level, 11);
        assert_eq!(packet.forge_data[0].index, 789);
        assert!(reader.is_empty());
        let mut writer = ByteWriter::new();
        packet.packet_to_bytes(&mut writer).unwrap();
        assert_eq!(writer.as_slice(), bytes);
    }
    #[test]
    fn movement_ground_skill_and_chat_use_2019_headers() {
        let position = WorldPosition {
            x: 150,
            y: 160,
            direction: Direction::North,
        };
        let mut writer = ByteWriter::new();
        RequestPlayerMovePacket::new(position).packet_to_bytes(&mut writer).unwrap();
        assert_eq!(&writer.as_slice()[..2], &[0x5F, 0x03]);
        assert_eq!(writer.as_slice().len(), 5);
        writer.clear();
        UseSkillOnGroundPacket::new(SkillLevel(1), SkillId(28), TilePosition { x: 150, y: 160 })
            .packet_to_bytes(&mut writer)
            .unwrap();
        assert_eq!(&writer.as_slice()[..2], &[0x66, 0x03]);
        assert_eq!(writer.as_slice().len(), 10);
        writer.clear();
        GlobalMessagePacket::new("Tester : hello".into())
            .packet_to_bytes(&mut writer)
            .unwrap();
        assert_eq!(&writer.as_slice()[..2], &[0xF3, 0x00]);
        assert_eq!(writer.as_slice().last(), Some(&0));
    }
}
