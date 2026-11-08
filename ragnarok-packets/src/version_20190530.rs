//! Hercules 20190530 RE wire types.
//!
//! This is the first codec in a staged port, NOT a complete selectable backend.
//! Do not register 20220406 response handlers for this version by aliasing
//! them. Wire reference: Turor/hercules-prere-plus at
//! 3846d1b4d3c1d8bf5e3a739f43c2206cc27bd0ed,
//! src/map/packets_shuffle_re.h (20190530: sex at offset 18), cross-checked
//! with roBrowserLegacy 3239980f4a69a3146d1c96d44dcce7104ebdc274 CZ.ENTER2
//! builder.

use ragnarok_macros::{ClientPacket, MapServer, Packet};

use crate::{AccountId, CharacterId, ClientTick, Sex};

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
