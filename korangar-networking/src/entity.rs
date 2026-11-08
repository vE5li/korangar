use ragnarok_packets::*;

#[derive(Debug)]
pub struct EntityData {
    pub entity_id: EntityId,
    pub movement_speed: u16,
    pub job_id: JobId,
    pub head: u16,
    pub position: WorldPosition,
    pub destination: Option<WorldPosition>,
    pub health_points: i64,
    pub maximum_health_points: i64,
    pub head_direction: usize,
    pub sex: Sex,
}

impl EntityData {
    pub fn from_character(account_id: AccountId, character_information: &CharacterInformation, position: WorldPosition) -> Self {
        Self {
            entity_id: EntityId(account_id.0),
            movement_speed: character_information.movement_speed as u16,
            job_id: character_information.job_id,
            head: character_information.head as u16,
            position,
            destination: None,
            health_points: character_information.health_points,
            maximum_health_points: character_information.maximum_health_points,
            head_direction: 0, // TODO: get correct rotation
            sex: character_information.sex,
        }
    }
}

impl From<EntityAppearPacket> for EntityData {
    fn from(packet: EntityAppearPacket) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            position: packet.position,
            destination: None,
            health_points: i64::from(packet.health_points),
            maximum_health_points: i64::from(packet.maximum_health_points),
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
}

impl From<EntityAppear2Packet> for EntityData {
    fn from(packet: EntityAppear2Packet) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            position: packet.position,
            destination: None,
            health_points: i64::from(packet.health_points),
            maximum_health_points: i64::from(packet.maximum_health_points),
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
}

impl From<MovingEntityAppearPacket> for EntityData {
    fn from(packet: MovingEntityAppearPacket) -> Self {
        let (origin, destination) = packet.position.to_origin_destination();

        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job_id: packet.job_id,
            head: packet.head,
            position: origin,
            destination: Some(destination),
            health_points: i64::from(packet.health_points),
            maximum_health_points: i64::from(packet.maximum_health_points),
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
}

#[cfg(test)]
mod parity_tests {
    use ragnarok_bytes::{ByteReader, FromBytes};

    use super::*;
    #[test]
    fn character_hp_is_not_narrowed_to_signed_32_bits() {
        let bytes = vec![0; 512];
        let mut character = CharacterInformation::from_bytes(&mut ByteReader::without_metadata(&bytes)).unwrap();
        character.health_points = 2_147_483_648;
        character.maximum_health_points = 3_000_000_000;
        let entity = EntityData::from_character(AccountId(1), &character, WorldPosition {
            x: 1,
            y: 1,
            direction: Direction::North,
        });
        assert_eq!(entity.health_points, character.health_points);
        assert_eq!(entity.maximum_health_points, character.maximum_health_points);
    }
}
