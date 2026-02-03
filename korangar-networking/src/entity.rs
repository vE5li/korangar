use ragnarok_packets::*;

#[derive(Debug)]
pub struct EntityData {
    pub entity_id: EntityId,
    pub movement_speed: u16,
    pub job: u16,
    pub head: u16,
    pub position: WorldPosition,
    pub destination: Option<WorldPosition>,
    pub health_points: i32,
    pub maximum_health_points: i32,
    pub head_direction: usize,
    pub sex: Sex,
}

impl EntityData {
    pub fn from_character(account_id: AccountId, character_information: &CharacterInformation, position: WorldPosition) -> Self {
        Self {
            entity_id: EntityId(account_id.0),
            movement_speed: character_information.movement_speed as u16,
            job: character_information.job as u16,
            head: character_information.head as u16,
            position,
            destination: None,
            health_points: character_information.health_points as i32,
            maximum_health_points: character_information.maximum_health_points as i32,
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
            job: packet.job,
            head: packet.head,
            position: packet.position,
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
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
            job: packet.job,
            head: packet.head,
            position: packet.position,
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
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
            job: packet.job,
            head: packet.head,
            position: origin,
            destination: Some(destination),
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
}
