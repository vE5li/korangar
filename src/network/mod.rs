mod login;

use std::fmt::Debug;
use std::io::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use cgmath::Vector2;
use chrono::Local;
use derive_new::new;
use procedural::*;

pub use self::login::LoginSettings;
#[cfg(feature = "debug_network")]
use crate::debug::Timer;
use crate::graphics::{Color, ColorBGRA, ColorRGBA};
#[cfg(feature = "debug_network")]
use crate::interface::PacketEntry;
use crate::interface::{CharacterSelectionWindow, ElementCell, PrototypeElement, TrackedState};
use crate::loaders::{ByteConvertable, ByteStream};

#[derive(Clone, Copy, Debug, ByteConvertable, PrototypeElement)]
pub struct ClientTick(pub u32);

// TODO: move to login
#[derive(Clone, Copy, Debug, ByteConvertable, PrototypeElement, PartialEq, Eq, Hash)]
pub struct AccountId(pub u32);

// TODO: move to character
#[derive(Clone, Copy, Debug, ByteConvertable, PrototypeElement, PartialEq, Eq, Hash)]
pub struct CharacterId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, PrototypeElement, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

/// Item index is always actual index + 2.
#[derive(Clone, Copy, Debug, PrototypeElement, PartialEq, Eq, Hash)]
pub struct ItemIndex(u16);

impl ByteConvertable for ItemIndex {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        Self(u16::from_bytes(byte_stream, length_hint) - 2)
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        u16::to_bytes(&(self.0 + 2), length_hint)
    }
}

#[derive(Clone, Copy, Debug, ByteConvertable, PrototypeElement, PartialEq, Eq, Hash)]
pub struct ItemId(pub u32);

/// Base trait that all packets implement.
/// All packets in Ragnarok online consist of a header, two bytes in size,
/// followed by the packet data. If the packet does not have a fixed size,
/// the first two bytes will be the size of the packet in bytes *including* the
/// header. Packets are sent in little endian.
pub trait Packet: PrototypeElement + Clone {
    const PACKET_NAME: &'static str;
    const IS_PING: bool;

    fn header() -> [u8; 2];

    fn to_bytes(&self) -> Vec<u8>;
}

/// An event triggered by the map server.
pub enum NetworkEvent {
    /// Add an entity to the list of entities that the client is aware of
    AddEntity(EntityData),
    /// Remove an entity from the list of entities that the client is aware of
    /// by its id
    RemoveEntity(EntityId),
    /// The player is pathing to a new position
    PlayerMove(Vector2<usize>, Vector2<usize>, ClientTick),
    /// An Entity nearby is pathing to a new position
    EntityMove(EntityId, Vector2<usize>, Vector2<usize>, ClientTick),
    /// Player was moved to a new position on a different map or the current map
    ChangeMap(String, Vector2<usize>),
    /// Update the client side [tick
    /// counter](crate::system::GameTimer::client_tick) to keep server and
    /// client synchronized
    UpdateClientTick(ClientTick),
    /// New chat message for the client
    ChatMessage(ChatMessage),
    /// Update entity details. Mostly received when the client sends
    /// [RequestDetailsPacket] after the player hovered an entity.
    UpdateEntityDetails(EntityId, String),
    UpdateEntityHealth(EntityId, usize, usize),
    DamageEffect(EntityId, usize),
    UpdateStatus(StatusType),
    OpenDialog(String, EntityId),
    AddNextButton,
    AddCloseButton,
    AddChoiceButtons(Vec<String>),
    AddQuestEffect(QuestEffectPacket),
    RemoveQuestEffect(EntityId),
    Inventory(Vec<(ItemIndex, ItemId, EquipPosition, EquipPosition)>),
    AddIventoryItem(ItemIndex, ItemId, EquipPosition, EquipPosition),
    UpdateEquippedPosition {
        index: ItemIndex,
        equipped_position: EquipPosition,
    },
}

pub struct ChatMessage {
    pub text: String,
    pub color: Color,
    offset: usize,
}

impl ChatMessage {
    // TODO: Maybe this shouldn't modify the text directly but rather save the
    // timestamp.
    pub fn new(mut text: String, color: Color) -> Self {
        let prefix = Local::now().format("^66BB44%H:%M:%S^000000: ").to_string();
        let offset = prefix.len();

        text.insert_str(0, &prefix);
        Self { text, color, offset }
    }

    pub fn stamped_text(&self, stamp: bool) -> &str {
        let start = self.offset * !stamp as usize;
        &self.text[start..]
    }
}

#[derive(Copy, Clone, Debug, ByteConvertable, PrototypeElement)]
pub enum Sex {
    Male,
    Female,
    Both,
    Server,
}

/// Sent by the client to the login server.
/// The very first packet sent when logging in, it is sent after the user has
/// entered email and password.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x64, 0x00)]
struct LoginServerLoginPacket {
    /// Unused
    #[new(default)]
    pub version: [u8; 4],
    #[length_hint(24)]
    pub name: String,
    #[length_hint(24)]
    pub password: String,
    /// Unused
    #[new(default)]
    pub client_type: u8,
}

/// Sent by the login server as a response to [LoginServerLoginPacket]
/// succeeding. After receiving this packet, the client will connect to one of
/// the character servers provided by this packet.
#[allow(dead_code)]
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc4, 0x0a)]
struct LoginServerLoginSuccessPacket {
    pub packet_length: u16,
    pub login_id1: u32,
    pub account_id: AccountId,
    pub login_id2: u32,
    /// Deprecated and always 0 on rAthena
    pub ip_address: u32,
    /// Deprecated and always 0 on rAthena
    pub name: [u8; 24],
    /// Always 0 on rAthena
    pub unknown: u16,
    pub sex: Sex,
    pub auth_token: [u8; 17],
    #[repeating((self.packet_length - 64) / 160)]
    pub character_server_information: Vec<CharacterServerInformation>,
}

/// Sent by the character server as a response to [CharacterServerLoginPacket]
/// succeeding. Provides basic information about the number of available
/// character slots.
#[allow(dead_code)]
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x2d, 0x08)]
struct CharacterServerLoginSuccessPacket {
    /// Always 29 on rAthena
    pub unknown: u16,
    pub normal_slot_count: u8,
    pub vip_slot_count: u8,
    pub billing_slot_count: u8,
    pub poducilble_slot_count: u8,
    pub vaild_slot: u8,
    pub unused: [u8; 20],
}

#[allow(dead_code)]
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x6b, 0x00)]
struct Packet6b00 {
    pub unused: u16,
    pub maximum_slot_count: u8,
    pub available_slot_count: u8,
    pub vip_slot_count: u8,
    pub unknown: [u8; 20],
}

#[allow(dead_code)]
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x18, 0x0b)]
struct Packet180b {
    /// Possibly inventory related
    pub unknown: u16,
}

#[derive(Clone, Debug, new, PrototypeElement)]
pub struct WorldPosition {
    pub x: usize,
    pub y: usize,
}

impl WorldPosition {
    pub fn to_vector(&self) -> Vector2<usize> {
        Vector2::new(self.x, self.y)
    }
}

impl ByteConvertable for WorldPosition {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());
        let coordinates = byte_stream.slice(3);

        let x = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        //let direction = ...

        Self {
            x: x as usize,
            y: y as usize,
        }
    }

    fn to_bytes(&self, length_hint: Option<usize>) -> Vec<u8> {
        assert!(length_hint.is_none());
        let mut coordinates = vec![0, 0, 0];

        coordinates[0] = (self.x >> 2) as u8;
        coordinates[1] = ((self.x << 6) as u8) | (((self.y >> 4) & 0x3f) as u8);
        coordinates[2] = (self.y << 4) as u8;

        coordinates
    }
}

#[derive(Clone, Debug, new, PrototypeElement)]
pub struct WorldPosition2 {
    pub x1: usize,
    pub y1: usize,
    pub x2: usize,
    pub y2: usize,
}

impl WorldPosition2 {
    pub fn to_vectors(&self) -> (Vector2<usize>, Vector2<usize>) {
        (Vector2::new(self.x1, self.y1), Vector2::new(self.x2, self.y2))
    }
}

impl ByteConvertable for WorldPosition2 {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());
        let coordinates: Vec<usize> = byte_stream.slice(6).into_iter().map(|byte| byte as usize).collect();

        let x1 = (coordinates[1] >> 6) | (coordinates[0] << 2);
        let y1 = (coordinates[2] >> 4) | ((coordinates[1] & 0b111111) << 4);
        let x2 = (coordinates[3] >> 2) | ((coordinates[2] & 0b1111) << 6);
        let y2 = coordinates[4] | ((coordinates[3] & 0b11) << 8);
        //let direction = ...

        Self { x1, y1, x2, y2 }
    }
}

/// Sent by the map server as a response to [MapServerLoginPacket] succeeding.
#[allow(dead_code)]
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xeb, 0x02)]
struct MapServerLoginSuccessPacket {
    pub client_tick: ClientTick,
    pub position: WorldPosition,
    /// Always [5, 5] on rAthena
    pub ignored: [u8; 2],
    pub font: u16,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub enum LoginFailedReason {
    #[numeric_value(1)]
    ServerClosed,
    #[numeric_value(2)]
    AlreadyLoggedIn,
    #[numeric_value(8)]
    AlreadyOnline,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x81, 0x00)]
struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x40, 0x08)]
struct MapServerUnavailablePacket {
    pub packet_length: u16,
    #[length_hint(self.packet_length - 4)]
    pub unknown: String,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub enum LoginFailedReason2 {
    UnregisteredId,
    IncorrectPassword,
    IdExpired,
    RejectedFromServer,
    BlockedByGMTeam,
    GameOutdated,
    LoginProhibitedUntil,
    ServerFull,
    CompanyAccountLimitReached,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x3e, 0x08)]
struct LoginFailedPacket2 {
    pub reason: LoginFailedReason2,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub enum CharacterSelectionFailedReason {
    RejectedFromServer,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// failing. Provides a reason for the character selection failing.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x6c, 0x00)]
struct CharacterSelectionFailedPacket {
    pub reason: CharacterSelectionFailedReason,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// succeeding. Provides a map server to connect to, along with the ID of our
/// selected character.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc5, 0x0a)]
struct CharacterSelectionSuccessPacket {
    pub character_id: CharacterId,
    #[length_hint(16)]
    pub map_name: String,
    pub map_server_ip: Ipv4Addr,
    pub map_server_port: u16,
    pub unknown: [u8; 128],
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub enum CharacterCreationFailedReason {
    CharacterNameAlreadyUsed,
    NotOldEnough,
    #[numeric_value(3)]
    NotAllowedToUseSlot,
    #[numeric_value(255)]
    CharacterCerationFailed,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// failing. Provides a reason for the character creation failing.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x6e, 0x00)]
struct CharacterCreationFailedPacket {
    pub reason: CharacterCreationFailedReason,
}

/// Sent by the client to the login server every 60 seconds to keep the
/// connection alive.
#[derive(Clone, Debug, Default, Packet, PrototypeElement)]
#[header(0x00, 0x02)]
#[ping]
struct LoginServerKeepalivePacket {
    pub user_id: [u8; 24],
}

impl ByteConvertable for Ipv4Addr {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());
        Ipv4Addr::new(byte_stream.next(), byte_stream.next(), byte_stream.next(), byte_stream.next())
    }
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct CharacterServerInformation {
    pub server_ip: Ipv4Addr,
    pub server_port: u16,
    pub server_name: [u8; 20],
    pub user_count: u16,
    pub server_type: u16, // ServerType
    pub display_new: u16, // bool16 ?
    pub unknown: [u8; 128],
}

/// Sent by the client to the character server after after successfully logging
/// into the login server.
/// Attempts to log into the character server using the provided information.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x65, 0x00)]
struct CharacterServerLoginPacket {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    #[new(default)]
    pub unknown: u16,
    pub sex: Sex,
}

/// Sent by the client to the map server after after successfully selecting a
/// character. Attempts to log into the map server using the provided
/// information.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x36, 0x04)]
struct MapServerLoginPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub login_id1: u32,
    pub client_tick: ClientTick,
    pub sex: Sex,
    #[new(default)]
    pub unknown: [u8; 4],
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x83, 0x02)]
struct Packet8302 {
    pub entity_id: EntityId,
}

/// Sent by the client to the character server when the player tries to create
/// a new character.
/// Attempts to create a new character in an empty slot using the provided
/// information.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x39, 0x0a)]
struct CreateCharacterPacket {
    #[length_hint(24)]
    pub name: String,
    pub slot: u8,
    pub hair_color: u16, // TODO: HairColor
    pub hair_style: u16, // TODO: HairStyle
    pub start_job: u16,  // TODO: Job
    #[new(default)]
    pub unknown: [u8; 2],
    pub sex: Sex,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct CharacterInformation {
    pub character_id: CharacterId,
    pub experience: i64,
    pub money: i32,
    pub job_experience: i64,
    pub jop_level: i32,
    pub body_state: i32,
    pub health_state: i32,
    pub effect_state: i32,
    pub virtue: i32,
    pub honor: i32,
    pub jobpoint: i16,
    pub health_points: i64,
    pub maximum_health_points: i64,
    pub spell_points: i64,
    pub maximum_spell_points: i64,
    pub movement_speed: i16,
    pub job: i16,
    pub head: i16,
    pub body: i16,
    pub weapon: i16,
    pub level: i16,
    pub sp_point: i16,
    pub accessory: i16,
    pub shield: i16,
    pub accessory2: i16,
    pub accessory3: i16,
    pub head_palette: i16,
    pub body_palette: i16,
    #[length_hint(24)]
    pub name: String,
    pub strength: u8,
    pub agility: u8,
    pub vit: u8,
    pub intelligence: u8,
    pub dexterity: u8,
    pub luck: u8,
    pub character_number: u8,
    pub hair_color: u8,
    pub b_is_changed_char: i16,
    #[length_hint(16)]
    pub map_name: String,
    pub deletion_reverse_date: i32,
    pub robe_palette: i32,
    pub character_slot_change_count: i32,
    pub character_name_change_count: i32,
    pub sex: Sex,
}

/// Sent by the character server as a response to [CreateCharacterPacket]
/// succeeding. Provides all character information of the newly created
/// character.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x6f, 0x0b)]
struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}

/// Sent by the client to the character server.
/// Requests a list of every character associated with the account.
#[derive(Clone, Debug, Default, Packet, PrototypeElement)]
#[header(0xa1, 0x09)]
struct RequestCharacterListPacket {}

/// Sent by the character server as a response to [RequestCharacterListPacket]
/// succeeding. Provides the requested list of character information.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x72, 0x0b)]
struct RequestCharacterListSuccessPacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 175)]
    pub character_information: Vec<CharacterInformation>,
}

/// Sent by the client to the map server when the player wants to move.
/// Attempts to path the player towards the provided position.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x81, 0x08)]
struct RequestPlayerMovePacket {
    pub position: WorldPosition,
}

/// Sent by the client to the map server when the player wants to warp.
/// Attempts to warp the player to a specific position on a specific map using
/// the provided information.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x40, 0x01)]
struct RequestWarpToMapPacket {
    #[length_hint(16)]
    pub map_name: String,
    pub x: u16,
    pub y: u16,
}

/// Sent by the map server to the client.
/// Informs the client that an entity is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x86, 0x00)]
struct EntityMovePacket {
    pub entity_id: EntityId,
    pub from_to: WorldPosition2,
    pub timestamp: ClientTick,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x88, 0x00)]
struct EntityStopMovePacket {
    pub entity_id: EntityId,
    pub x: u16,
    pub y: u16,
}

/// Sent by the map server to the client.
/// Informs the client that the player is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x87, 0x00)]
struct PlayerMovePacket {
    pub timestamp: ClientTick,
    pub from_to: WorldPosition2,
}

/// Sent by the client to the character server when the user tries to delete a
/// character.
/// Attempts to delete a character from the user account using the provided
/// information.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0xfb, 0x01)]
struct DeleteCharacterPacket {
    character_id: CharacterId,
    /// This field can be used for email or date of birth, depending on the
    /// configuration of the character server.
    #[length_hint(40)]
    pub email: String,
    /// Ignored by rAthena
    #[new(default)]
    pub unknown: [u8; 10],
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub enum CharacterDeletionFailedReason {
    NotAllowed,
    CharacterNotFound,
    NotEligible,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// failing. Provides a reason for the character deletion failing.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x70, 0x00)]
struct CharacterDeletionFailedPacket {
    pub reason: CharacterDeletionFailedReason,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// succeeding.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x6f, 0x00)]
struct CharacterDeletionSuccessPacket {}

/// Sent by the client to the character server when the user selects a
/// character. Attempts to select the character in the specified slot.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x66, 0x00)]
struct SelectCharacterPacket {
    pub selected_slot: u8,
}

/// Sent by the map server to the client when there is a new chat message from
/// the server. Provides the message to be displayed in the chat window.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x8e, 0x00)]
struct ServerMessagePacket {
    pub packet_length: u16,
    #[length_hint(self.packet_length - 4)]
    pub message: String,
}

/// Sent by the client to the map server when the user hovers over an entity.
/// Attempts to fetch additional information about the entity, such as the
/// display name.
#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x68, 0x03)]
struct RequestDetailsPacket {
    pub entity_id: EntityId,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the player.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x30, 0x0a)]
struct RequestPlayerDetailsSuccessPacket {
    pub character_id: CharacterId,
    #[length_hint(24)]
    pub name: String,
    #[length_hint(24)]
    pub party_name: String,
    #[length_hint(24)]
    pub guild_name: String,
    #[length_hint(24)]
    pub position_name: String,
    pub title_id: u32,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the entity.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xdf, 0x0a)]
struct RequestEntityDetailsSuccessPacket {
    pub entity_id: EntityId,
    pub group_id: u32,
    #[length_hint(24)]
    pub name: String,
    #[length_hint(24)]
    pub title: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xe7, 0x09)]
struct NewMailStatusPacket {
    pub new_available: u8,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct AchievementData {
    pub acheivement_id: u32,
    pub is_completed: u8,
    pub objectives: [u32; 10],
    pub completion_timestamp: u32,
    pub got_rewarded: u8,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x24, 0x0a)]
struct AchievementUpdatePacket {
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    pub acheivement_data: AchievementData,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x23, 0x0a)]
struct AchievementListPacket {
    pub packet_length: u16,
    pub acheivement_count: u32,
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    #[repeating(self.acheivement_count)]
    pub acheivement_data: Vec<AchievementData>,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xde, 0x0a)]
struct CriticalWeightUpdatePacket {
    pub packet_length: u32,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xd7, 0x01)]
struct SpriteChangePacket {
    pub entity_id: EntityId,
    pub sprite_type: u8, // TODO: Is it actually the sprite type?
    pub value: u32,
    pub value2: u32,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x08, 0x0b)]
struct InventoyStartPacket {
    pub packet_length: u16,
    pub inventory_type: u8,
    #[length_hint(self.packet_length - 5)]
    pub inventory_name: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x0b, 0x0b)]
struct InventoyEndPacket {
    pub inventory_type: u8,
    pub flag: u8, // maybe char ?
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
pub struct ItemOptions {
    pub index: u16,
    pub value: u16,
    pub parameter: u8,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct RegularItemInformation {
    pub index: ItemIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub amount: u16,
    pub wear_state: u32,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: i32,
    pub fags: u8, // bit 1 - is_identified; bit 2 - place_in_etc_tab;
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x09, 0x0b)]
struct RegularItemListPacket {
    pub packet_length: u16,
    pub inventory_type: u8,
    #[repeating((self.packet_length - 5) / 34)]
    pub item_information: Vec<RegularItemInformation>,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct EquippableItemInformation {
    pub index: ItemIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub equip_position: EquipPosition,
    pub equipped_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: i32,
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8,
    pub option_data: [ItemOptions; 5], // fix count
    pub refinement_level: u8,
    pub enchantment_level: u8,
    pub fags: u8, // bit 1 - is_identified; bit 2 - is_damaged; bit 3 - place_in_etc_tab
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x39, 0x0b)]
struct EquippableItemListPacket {
    pub packet_length: u16,
    pub inventory_type: u8,
    #[repeating((self.packet_length - 5) / 68)]
    pub item_information: Vec<EquippableItemInformation>,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct EquippableSwitchItemInformation {
    pub index: ItemIndex,
    pub position: u32,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x9b, 0x0a)]
struct EquippableSwitchItemListPacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 6)] // TODO: (remaining / 6)
    pub item_information: Vec<EquippableSwitchItemInformation>,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x9b, 0x09)]
struct MapTypePacket {
    pub map_type: u16,
    pub flags: u32,
}

/// Sent by the map server to the client when there is a new chat message from
/// ??. Provides the message to be displayed in the chat window, as well as
/// information on how the message should be displayed.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc3, 0x01)]
struct BroadcastMessagePacket {
    pub packet_length: u16,
    pub font_color: ColorRGBA,
    pub font_type: u16,
    pub font_size: u16,
    pub font_alignment: u16,
    pub font_y: u16,
    #[length_hint(self.packet_length - 16)]
    pub message: String,
}

/// Sent by the map server to the client when there is a new chat message from
/// an entity. Provides the message to be displayed in the chat window, the
/// color of the message, and the ID of the entity it originated from.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc1, 0x02)]
struct EntityMessagePacket {
    pub packet_length: u16,
    pub entity_id: EntityId,
    pub color: ColorBGRA,
    #[length_hint(self.packet_length - 12)]
    pub message: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc0, 0x00)]
struct DisplayEmotionPacket {
    pub entity_id: EntityId,
    pub emotion: u8,
}

/// Every value that can be set from the server through [UpdateStatusPacket],
/// [UpdateStatusPacket1], [UpdateStatusPacket2], and [UpdateStatusPacket3].
/// All UpdateStatusPackets do the same, they just have different sizes
/// correlating to the space the updated value requires.
#[derive(Clone, Debug)]
pub enum StatusType {
    Weight(u32),
    MaximumWeight(u32),
    MovementSpeed(u32),
    BaseLevel(u32),
    JobLevel(u32),
    Karma(u32),
    Manner(u32),
    StatusPoint(u32),
    SkillPoint(u32),
    Hit(u32),
    Flee1(u32),
    Flee2(u32),
    MaximumHealthPoints(u32),
    MaximumSpellPoints(u32),
    HealthPoints(u32),
    SpellPoints(u32),
    AttackSpeed(u32),
    Attack1(u32),
    Defense1(u32),
    MagicDefense1(u32),
    Attack2(u32),
    Defense2(u32),
    MagicDefense2(u32),
    Critical(u32),
    MagicAttack1(u32),
    MagicAttack2(u32),
    Zeny(u32),
    BaseExperience(u64),
    JobExperience(u64),
    NextBaseExperience(u64),
    NextJobExperience(u64),
    SpUstr(u8),
    SpUagi(u8),
    SpUvit(u8),
    SpUint(u8),
    SpUdex(u8),
    SpUluk(u8),
    Strength(u32, u32),
    Agility(u32, u32),
    Vitality(u32, u32),
    Intelligence(u32, u32),
    Dexterity(u32, u32),
    Luck(u32, u32),
    CartInfo(u16, u32, u32),
    ActivityPoints(u32),
    TraitPoint(u32),
    MaximumActivityPoints(u32),
    Power(u32, u32),
    Stamina(u32, u32),
    Wisdom(u32, u32),
    Spell(u32, u32),
    Concentration(u32, u32),
    Creativity(u32, u32),
    SpUpow(u8),
    SpUsta(u8),
    SpUwis(u8),
    SpUspl(u8),
    SpUcon(u8),
    SpUcrt(u8),
    PhysicalAttack(u32),
    SpellMagicAttack(u32),
    Resistance(u32),
    MagicResistance(u32),
    HealingPlus(u32),
    CriticalDamageRate(u32),
}

impl ByteConvertable for StatusType {
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        let data = byte_stream.slice(length_hint.unwrap());
        let mut byte_stream = ByteStream::new(&data);

        match u16::from_bytes(&mut byte_stream, None) {
            0 => Self::MovementSpeed(u32::from_bytes(&mut byte_stream, None)),
            1 => Self::BaseExperience(u64::from_bytes(&mut byte_stream, None)),
            2 => Self::JobExperience(u64::from_bytes(&mut byte_stream, None)),
            3 => Self::Karma(u32::from_bytes(&mut byte_stream, None)),
            4 => Self::Manner(u32::from_bytes(&mut byte_stream, None)),
            5 => Self::HealthPoints(u32::from_bytes(&mut byte_stream, None)),
            6 => Self::MaximumHealthPoints(u32::from_bytes(&mut byte_stream, None)),
            7 => Self::SpellPoints(u32::from_bytes(&mut byte_stream, None)),
            8 => Self::MaximumSpellPoints(u32::from_bytes(&mut byte_stream, None)),
            9 => Self::StatusPoint(u32::from_bytes(&mut byte_stream, None)),
            11 => Self::BaseLevel(u32::from_bytes(&mut byte_stream, None)),
            12 => Self::SkillPoint(u32::from_bytes(&mut byte_stream, None)),
            13 => Self::Strength(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            14 => Self::Agility(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            15 => Self::Vitality(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            16 => Self::Intelligence(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            17 => Self::Dexterity(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            18 => Self::Luck(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            20 => Self::Zeny(u32::from_bytes(&mut byte_stream, None)),
            22 => Self::NextBaseExperience(u64::from_bytes(&mut byte_stream, None)),
            23 => Self::NextJobExperience(u64::from_bytes(&mut byte_stream, None)),
            24 => Self::Weight(u32::from_bytes(&mut byte_stream, None)),
            25 => Self::MaximumWeight(u32::from_bytes(&mut byte_stream, None)),
            32 => Self::SpUstr(u8::from_bytes(&mut byte_stream, None)),
            33 => Self::SpUagi(u8::from_bytes(&mut byte_stream, None)),
            34 => Self::SpUvit(u8::from_bytes(&mut byte_stream, None)),
            35 => Self::SpUint(u8::from_bytes(&mut byte_stream, None)),
            36 => Self::SpUdex(u8::from_bytes(&mut byte_stream, None)),
            37 => Self::SpUluk(u8::from_bytes(&mut byte_stream, None)),
            41 => Self::Attack1(u32::from_bytes(&mut byte_stream, None)),
            42 => Self::Attack2(u32::from_bytes(&mut byte_stream, None)),
            43 => Self::MagicAttack1(u32::from_bytes(&mut byte_stream, None)),
            44 => Self::MagicAttack2(u32::from_bytes(&mut byte_stream, None)),
            45 => Self::Defense1(u32::from_bytes(&mut byte_stream, None)),
            46 => Self::Defense2(u32::from_bytes(&mut byte_stream, None)),
            47 => Self::MagicDefense1(u32::from_bytes(&mut byte_stream, None)),
            48 => Self::MagicDefense2(u32::from_bytes(&mut byte_stream, None)),
            49 => Self::Hit(u32::from_bytes(&mut byte_stream, None)),
            50 => Self::Flee1(u32::from_bytes(&mut byte_stream, None)),
            51 => Self::Flee2(u32::from_bytes(&mut byte_stream, None)),
            52 => Self::Critical(u32::from_bytes(&mut byte_stream, None)),
            53 => Self::AttackSpeed(u32::from_bytes(&mut byte_stream, None)),
            55 => Self::JobLevel(u32::from_bytes(&mut byte_stream, None)),
            99 => Self::CartInfo(
                u16::from_bytes(&mut byte_stream, None),
                u32::from_bytes(&mut byte_stream, None),
                u32::from_bytes(&mut byte_stream, None),
            ),
            219 => Self::Power(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            220 => Self::Stamina(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            221 => Self::Wisdom(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            222 => Self::Spell(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            223 => Self::Concentration(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            224 => Self::Creativity(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            225 => Self::PhysicalAttack(u32::from_bytes(&mut byte_stream, None)),
            226 => Self::SpellMagicAttack(u32::from_bytes(&mut byte_stream, None)),
            227 => Self::Resistance(u32::from_bytes(&mut byte_stream, None)),
            228 => Self::MagicResistance(u32::from_bytes(&mut byte_stream, None)),
            229 => Self::HealingPlus(u32::from_bytes(&mut byte_stream, None)),
            230 => Self::CriticalDamageRate(u32::from_bytes(&mut byte_stream, None)),
            231 => Self::TraitPoint(u32::from_bytes(&mut byte_stream, None)),
            232 => Self::ActivityPoints(u32::from_bytes(&mut byte_stream, None)),
            233 => Self::MaximumActivityPoints(u32::from_bytes(&mut byte_stream, None)),
            247 => Self::SpUpow(u8::from_bytes(&mut byte_stream, None)),
            248 => Self::SpUsta(u8::from_bytes(&mut byte_stream, None)),
            249 => Self::SpUwis(u8::from_bytes(&mut byte_stream, None)),
            250 => Self::SpUspl(u8::from_bytes(&mut byte_stream, None)),
            251 => Self::SpUcon(u8::from_bytes(&mut byte_stream, None)),
            252 => Self::SpUcrt(u8::from_bytes(&mut byte_stream, None)),
            invalid => panic!("invalid status code {invalid}"),
        }
    }
}

// TODO: make StatusType derivable
impl PrototypeElement for StatusType {
    fn to_element(&self, display: String) -> ElementCell {
        "<todo>".to_element(display)
    }
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb0, 0x00)]
struct UpdateStatusPacket {
    #[length_hint(6)]
    pub status_type: StatusType,
}

/// Sent by the character server to the client when loading onto a new map.
/// This packet is ignored by Korangar since all of the provided values are set
/// again individually using the UpdateStatusPackets.
#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xbd, 0x00)]
struct InitialStatusPacket {
    pub status_points: u16,
    pub strength: u8,
    pub required_strength: u8,
    pub agility: u8,
    pub required_agility: u8,
    pub vitatity: u8,
    pub required_vitatity: u8,
    pub intelligence: u8,
    pub required_intelligence: u8,
    pub dexterity: u8,
    pub required_dexterity: u8,
    pub luck: u8,
    pub required_luck: u8,
    pub left_attack: u16,
    pub rigth_attack: u16,
    pub rigth_magic_attack: u16,
    pub left_magic_attack: u16,
    pub left_defense: u16,
    pub rigth_defense: u16,
    pub rigth_magic_defense: u16,
    pub left_magic_defense: u16,
    pub hit: u16, // ?
    pub flee: u16,
    pub flee2: u16,
    pub crit: u16,
    pub attack_speed: u16,
    /// Always 0 on rAthena
    pub bonus_attack_speed: u16,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x41, 0x01)]
struct UpdateStatusPacket1 {
    #[length_hint(12)]
    pub status_type: StatusType,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xcb, 0x0a)]
struct UpdateStatusPacket2 {
    #[length_hint(10)]
    pub status_type: StatusType,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xbe, 0x00)]
struct UpdateStatusPacket3 {
    #[length_hint(3)]
    pub status_type: StatusType,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x3a, 0x01)]
struct UpdateAttackRangePacket {
    pub attack_range: u16,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0xd4, 0x08)]
struct SwitchCharacterSlotPacket {
    pub origin_slot: u16,
    pub destination_slot: u16,
    /// 1 instead of default, just in case the sever actually uses this value
    /// (rAthena does not)
    #[new(value = "1")]
    pub remaining_moves: u16,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
enum Action {
    Attack,
    PickUpItem,
    SitDown,
    StandUp,
    #[numeric_value(7)]
    ContinousAttack,
    /// Unsure what this does
    #[numeric_value(12)]
    TouchSkill,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x37, 0x04)]
struct RequestActionPacket {
    pub npc_id: EntityId,
    pub action: Action,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0xf3, 0x00)]
struct GlobalMessagePacket {
    pub packet_length: u16,
    pub message: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x39, 0x01)]
struct RequestPlayerAttackFailedPacket {
    pub target_entity_id: EntityId,
    pub target_x: u16,
    pub target_y: u16,
    pub x: u16,
    pub y: u16,
    pub attack_range: u16,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x77, 0x09)]
struct UpdateEntityHealthPointsPacket {
    pub entity_id: EntityId,
    pub health_points: u32,
    pub maximum_health_points: u32,
}

/*#[derive(Clone, Debug, ByteConvertable)]
enum DamageType {
}*/

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc8, 0x08)]
struct DamagePacket {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub source_movement_speed: u32,
    pub destination_movement_speed: u32,
    pub damage_amount: u32,
    pub is_special_damage: u8,
    pub amount_of_hits: u16,
    pub damage_type: u8,
    /// Assassin dual wield damage
    pub damage_amount2: u32,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x7f, 0x00)]
#[ping]
struct ServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x60, 0x03)]
#[ping]
struct RequestServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Clone, Debug, PartialEq, Eq, ByteConvertable, PrototypeElement)]
#[numeric_type(u16)]
pub enum SwitchCharacterSlotResponseStatus {
    Success,
    Error,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x70, 0x0b)]
struct SwitchCharacterSlotResponsePacket {
    pub unknown: u16, // is always 8 ?
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x91, 0x00)]
struct ChangeMapPacket {
    #[length_hint(16)]
    pub map_name: String,
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
enum DissapearanceReason {
    OutOfSight,
    Died,
    LoggedOut,
    Teleported,
    TrickDead,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x80, 0x00)]
struct EntityDisappearedPacket {
    pub entity_id: EntityId,
    pub reason: DissapearanceReason,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xfd, 0x09)]
struct MovingEntityAppearedPacket {
    pub packet_length: u16,
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job: u16,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub move_start_time: u32,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition2,
    pub x_size: u8,
    pub y_size: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length_hint(24)]
    pub name: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xff, 0x09)]
struct EntityAppearedPacket {
    pub packet_length: u16,
    pub object_type: u8,
    pub entity_id: EntityId,
    pub group_id: u32, // may be reversed - or completely wrong
    pub movement_speed: u16,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub job: u16,
    pub head: u16,
    pub weapon: u32,
    pub shield: u32,
    pub accessory: u16,
    pub accessory2: u16,
    pub accessory3: u16,
    pub head_palette: u16,
    pub body_palette: u16,
    pub head_direction: u16,
    pub robe: u16,
    pub guild_id: u32, // may be reversed - or completely wrong
    pub emblem_version: u16,
    pub honor: u16,
    pub virtue: u32,
    pub is_pk_mode_on: u8,
    pub sex: Sex,
    pub position: WorldPosition,
    pub x_size: u8,
    pub y_size: u8,
    pub state: u8,
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length_hint(24)]
    pub name: String,
}

pub struct EntityData {
    pub entity_id: EntityId,
    pub movement_speed: u16,
    pub job: u16,
    pub position: Vector2<usize>,
    pub destination: Option<Vector2<usize>>,
    pub health_points: i32,
    pub maximum_health_points: i32,
    pub head_direction: usize,
}

impl EntityData {
    pub fn from_character(character_information: CharacterInformation, position: Vector2<usize>) -> Self {
        Self {
            entity_id: EntityId(character_information.character_id.0), // TODO: should not mix like that
            movement_speed: character_information.movement_speed as u16,
            job: character_information.job as u16,
            position,
            destination: None,
            health_points: character_information.health_points as i32,
            maximum_health_points: character_information.maximum_health_points as i32,
            head_direction: 0, // TODO: get correct rotation
        }
    }
}

impl From<EntityAppearedPacket> for EntityData {
    fn from(packet: EntityAppearedPacket) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job: packet.job,
            position: packet.position.to_vector(),
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
        }
    }
}

impl From<MovingEntityAppearedPacket> for EntityData {
    fn from(packet: MovingEntityAppearedPacket) -> Self {
        let (origin, destination) = packet.position.to_vectors();

        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job: packet.job,
            position: origin,
            destination: Some(destination),
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
        }
    }
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct SkillInformation {
    pub skill_id: u16,
    pub skill_type: u32,
    pub skill_level: u16,
    pub spell_point_cost: u16,
    pub attack_range: u16,
    #[length_hint(24)]
    pub skill_name: String,
    pub upgraded: u8,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x0f, 0x01)]
struct UpdateSkillTreePacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 37)]
    pub skill_information: Vec<SkillInformation>,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct HotkeyData {
    pub is_skill: u8,
    pub skill_id: u32,
    pub quantity_or_skill_level: u16,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x20, 0x0b)]
struct UpdateHotkeysPacket {
    pub rotate: u8,
    pub tab: u16,
    pub hotkeys: [HotkeyData; 38],
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xc9, 0x02)]
struct UpdatePartyInvitationStatePacket {
    pub allowed: u8, // always 0 on rAthena
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xda, 0x02)]
struct UpdateShowEquipPacket {
    pub open_equip_window: u8,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xd9, 0x02)]
struct UpdateConfigurationPacket {
    pub config_type: u32,
    pub value: u32, // only enabled and disabled ?
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xe2, 0x08)]
struct NavigateToMonsterPacket {
    pub target_type: u8, // 3 - entity; 0 - coordinates; 1 - coordinates but fails if you're alweady on the map
    pub flags: u8,
    pub hide_window: u8,
    #[length_hint(16)]
    pub map_name: String,
    pub target_x: u16,
    pub target_y: u16,
    pub target_monster_id: u16,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u32)]
enum MarkerType {
    DisplayFor15Seconds,
    DisplayUntilLeave,
    RemoveMark,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x44, 0x01)]
struct MarkMinimapPositionPacket {
    pub npc_id: EntityId,
    pub marker_type: MarkerType,
    pub position: Vector2<u32>,
    pub id: u8,
    pub color: ColorRGBA,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb5, 0x00)]
struct NextButtonPacket {
    pub entity_id: EntityId,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb6, 0x00)]
struct CloseButtonPacket {
    pub entity_id: EntityId,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb7, 0x00)]
struct DialogMenuPacket {
    pub packet_length: u16,
    pub entity_id: EntityId,
    #[length_hint(self.packet_length - 8)]
    pub message: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xf3, 0x01)]
struct DisplaySpecialEffectPacket {
    pub entity_id: EntityId,
    pub effect_id: u32,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xcb, 0x09)]
struct DisplaySkillEffectPacket {
    pub skill_id: u16,
    pub heal: u32,
    pub destination_entity_id: EntityId,
    pub source_entity_id: EntityId,
    pub result: u8,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x83, 0x09)]
struct StatusChangePacket {
    pub index: u16,
    pub entity_id: EntityId,
    pub state: u8,
    pub duration_in_milliseconds: u32,
    pub remaining_in_milliseconds: u32,
    pub value: [u32; 3],
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct ObjectiveDetails1 {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub mob_count: u16,
    #[length_hint(24)]
    pub mob_name: String,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xf9, 0x09)]
struct QuestNotificationPacket1 {
    pub quest_id: u32,
    pub active: u8,
    pub start_time: u32,
    pub expire_time: u32,
    pub objective_count: u16,
    /// For some reason this packet always has space for three objective
    /// details, even if none are sent
    pub objective_details: [ObjectiveDetails1; 3],
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct HuntingObjective {
    pub quest_id: u32,
    pub mob_id: u32,
    pub total_count: u16,
    pub current_count: u16,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xfe, 0x08)]
struct HuntingQuestNotificationPacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 12)]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xfa, 0x09)]
struct HuntingQuestUpdateObjectivePacket {
    pub packet_length: u16,
    pub objective_count: u16,
    #[repeating((self.packet_length - 4) / 12)]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb4, 0x02)]
struct QuestRemovedPacket {
    pub quest_id: u32,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct QuestDetails {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub kill_count: u16,
    pub total_count: u16,
    #[length_hint(24)]
    pub mob_name: String,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
struct Quest {
    pub quest_id: u32,
    pub active: u8,
    pub remaining_time: u32, // TODO: double check these
    pub expire_time: u32,    // TODO: double check these
    pub objective_count: u16,
    #[repeating(self.objective_count)]
    pub objective_details: Vec<QuestDetails>,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xf8, 0x09)]
struct QuestListPacket {
    pub packet_length: u16,
    pub quest_count: u32,
    #[repeating(self.quest_count)]
    pub quests: Vec<Quest>,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u32)]
enum VisualEffect {
    BaseLevelUp,
    JobLevelUp,
    RefineFailure,
    RefineSuccess,
    GameOver,
    PharmacySuccess,
    PharmacyFailure,
    BaseLevelUpSuperNovice,
    JobLevelUpSuperNovice,
    BaseLevelUpTaekwon,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x9b, 0x01)]
struct VisualEffectPacket {
    pub entity_id: EntityId,
    pub effect: VisualEffect,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u16)]
enum ExperienceType {
    #[numeric_value(1)]
    BaseExperience,
    JobExperience,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u16)]
enum ExperienceSource {
    Regular,
    Quest,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xcc, 0x0a)]
struct DisplayGainedExperiencePacket {
    pub account_id: AccountId,
    pub amount: u64,
    pub experience_type: ExperienceType,
    pub experience_source: ExperienceSource,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
enum ImageLocation {
    BottomLeft,
    BottomMiddle,
    BottomRight,
    MiddleFloating,
    MiddleColorless,
    #[numeric_value(255)]
    ClearAll,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb3, 0x01)]
struct DisplayImagePacket {
    #[length_hint(64)]
    pub image_name: String,
    pub location: ImageLocation,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x29, 0x02)]
struct StateChangePacket {
    pub entity_id: EntityId,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub is_pk_mode_on: u8,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x41, 0x0b)]
struct ItemPickupPacket {
    pub index: ItemIndex,
    pub count: u16,
    pub item_id: ItemId,
    pub is_identified: u8,
    pub is_broken: u8,
    pub cards: [u32; 4],
    pub equip_position: EquipPosition,
    pub item_type: u8,
    pub result: u8,
    pub hire_expiration_date: u32,
    pub bind_on_equip_type: u16,
    pub option_data: [ItemOptions; 5], // fix count
    pub favorite: u8,
    pub look: u16,
    pub refinement_level: u8,
    pub enchantment_level: u8,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u16)]
enum RemoveItemReason {
    Normal,
    ItemUsedForSkill,
    RefinsFailed,
    MaterialChanged,
    MovedToStorage,
    MovedToCart,
    ItemSold,
    ConsumedByFourSpiritAnalysis,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xfa, 0x07)]
struct RemoveItemFromInventoryPacket {
    pub remove_reason: RemoveItemReason,
    pub index: u16,
    pub amount: u16,
}

// TODO: improve names
#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u16)]
pub enum QuestEffect {
    Quest,
    Quest2,
    Job,
    Job2,
    Event,
    Event2,
    ClickMe,
    DailyQuest,
    Event3,
    JobQuest,
    JumpingPoring,
    #[numeric_value(9999)]
    None,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
#[numeric_type(u16)]
pub enum QuestColor {
    Yellow,
    Orange,
    Green,
    Purple,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x46, 0x04)]
pub struct QuestEffectPacket {
    pub entity_id: EntityId,
    pub position: Vector2<u16>,
    pub effect: QuestEffect,
    pub color: QuestColor,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0xb4, 0x00)]
struct NpcDialogPacket {
    pub packet_length: u16,
    pub npc_id: EntityId,
    #[length_hint(self.packet_length - 8)]
    pub text: String,
}

#[derive(Clone, Debug, Default, Packet, PrototypeElement)]
#[header(0x7d, 0x00)]
struct MapLoadedPacket {}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x87, 0x01)]
#[ping]
struct CharacterServerKeepalivePacket {
    /// rAthena never reads this value, so just set it to 0.
    #[new(value = "AccountId(0)")]
    pub account_id: AccountId,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x90, 0x00)]
struct StartDialogPacket {
    pub npc_id: EntityId,
    #[new(value = "1")]
    pub dialog_type: u8,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0xb9, 0x00)]
struct NextDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x46, 0x01)]
struct CloseDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0xb8, 0x00)]
struct ChooseDialogOptionPacket {
    pub npc_id: EntityId,
    pub option: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ByteConvertable, PrototypeElement)]
#[numeric_type(u32)]
pub enum EquipPosition {
    #[numeric_value(0)]
    None,
    #[numeric_value(1)]
    HeadLower,
    #[numeric_value(512)]
    HeadMiddle,
    #[numeric_value(256)]
    HeadTop,
    #[numeric_value(2)]
    RightHand,
    #[numeric_value(32)]
    LeftHand,
    #[numeric_value(16)]
    Armor,
    #[numeric_value(64)]
    Shoes,
    #[numeric_value(4)]
    Garment,
    #[numeric_value(8)]
    LeftAccessory,
    #[numeric_value(128)]
    RigthAccessory,
    #[numeric_value(1024)]
    CostumeHeadTop,
    #[numeric_value(2048)]
    CostumeHeadMiddle,
    #[numeric_value(4196)]
    CostumeHeadLower,
    #[numeric_value(8192)]
    CostumeGarment,
    #[numeric_value(32768)]
    Ammo,
    #[numeric_value(65536)]
    ShadowArmor,
    #[numeric_value(131072)]
    ShadowWeapon,
    #[numeric_value(262144)]
    ShadowShield,
    #[numeric_value(524288)]
    ShadowShoes,
    #[numeric_value(1048576)]
    ShadowRightAccessory,
    #[numeric_value(2097152)]
    ShadowLeftAccessory,
    #[numeric_value(34)]
    LeftRightAccessory,
    #[numeric_value(3145728)]
    ShadowLeftRightAccessory,
}

impl EquipPosition {
    pub fn display_name(&self) -> &'static str {
        match self {
            EquipPosition::None => panic!(),
            EquipPosition::HeadLower => "head lower",
            EquipPosition::HeadMiddle => "head middle",
            EquipPosition::HeadTop => "head top",
            EquipPosition::RightHand => "right hand",
            EquipPosition::LeftHand => "left hand",
            EquipPosition::Armor => "armor",
            EquipPosition::Shoes => "shoes",
            EquipPosition::Garment => "garment",
            EquipPosition::LeftAccessory => "left accessory",
            EquipPosition::RigthAccessory => "right accessory",
            EquipPosition::CostumeHeadTop => "costume head top",
            EquipPosition::CostumeHeadMiddle => "costume head middle",
            EquipPosition::CostumeHeadLower => "costume head lower",
            EquipPosition::CostumeGarment => "costume garment",
            EquipPosition::Ammo => "ammo",
            EquipPosition::ShadowArmor => "shadow ammo",
            EquipPosition::ShadowWeapon => "shadow weapon",
            EquipPosition::ShadowShield => "shadow shield",
            EquipPosition::ShadowShoes => "shadow shoes",
            EquipPosition::ShadowRightAccessory => "shadow right accessory",
            EquipPosition::ShadowLeftAccessory => "shadow left accessory",
            EquipPosition::LeftRightAccessory => "accessory",
            EquipPosition::ShadowLeftRightAccessory => "shadow accessory",
        }
    }
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0x98, 0x09)]
struct RequestEquipItemPacket {
    pub inventory_index: ItemIndex,
    pub equip_position: EquipPosition,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
enum RequestEquipItemStatus {
    Success,
    Failed,
    FailedDueToLevelRequirement,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x99, 0x09)]
struct RequestEquipItemStatusPacket {
    pub inventory_index: ItemIndex,
    pub equipped_position: EquipPosition,
    pub view_id: u16,
    pub result: RequestEquipItemStatus,
}

#[derive(Clone, Debug, Packet, PrototypeElement, new)]
#[header(0xab, 0x00)]
struct RequestUnequipItemPacket {
    pub inventory_index: ItemIndex,
}

#[derive(Clone, Debug, ByteConvertable, PrototypeElement)]
enum RequestUnequipItemStatus {
    Success,
    Failed,
}

#[derive(Clone, Debug, Packet, PrototypeElement)]
#[header(0x9a, 0x09)]
struct RequestUnequipItemStatusPacket {
    pub inventory_index: ItemIndex,
    pub equipped_position: EquipPosition,
    pub result: RequestUnequipItemStatus,
}

#[derive(new)]
struct NetworkTimer {
    period: Duration,
    #[new(default)]
    accumulator: Duration,
}

impl NetworkTimer {
    pub fn update(&mut self, elapsed_time: f64) -> bool {
        self.accumulator += Duration::from_secs_f64(elapsed_time);
        let reset = self.accumulator > self.period;

        if reset {
            self.accumulator -= self.period;
        }

        reset
    }
}

#[derive(new)]
struct LoginData {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub sex: Sex,
}

pub struct NetworkingSystem {
    login_settings: LoginSettings,
    login_stream: TcpStream,
    character_stream: Option<TcpStream>,
    map_stream: Option<TcpStream>,
    login_data: Option<LoginData>,
    characters: TrackedState<Vec<CharacterInformation>>,
    move_request: TrackedState<Option<usize>>,
    login_keep_alive_timer: NetworkTimer,
    character_keep_alive_timer: NetworkTimer,
    map_keep_alive_timer: NetworkTimer,
    player_name: String,
    #[cfg(feature = "debug_network")]
    packet_history: TrackedState<Vec<PacketEntry>>,
}

impl NetworkingSystem {
    pub fn new() -> Self {
        let login_server_ip = match cfg!(feature = "local") {
            true => "127.0.0.1:6900",
            false => "167.235.227.244:6900",
        };

        let login_settings = LoginSettings::new();
        let login_stream = TcpStream::connect(login_server_ip).expect("failed to connect to login server");

        let character_stream = None;
        let map_stream = None;
        let login_data = None;
        let characters = TrackedState::default();
        let move_request = TrackedState::default();
        let login_keep_alive_timer = NetworkTimer::new(Duration::from_secs(58));
        let character_keep_alive_timer = NetworkTimer::new(Duration::from_secs(10));
        let map_keep_alive_timer = NetworkTimer::new(Duration::from_secs(4));
        let player_name = String::new();
        #[cfg(feature = "debug_network")]
        let packet_history = TrackedState::default();

        login_stream.set_read_timeout(Duration::from_secs(1).into()).unwrap();

        Self {
            login_settings,
            login_stream,
            character_stream,
            move_request,
            login_data,
            map_stream,
            characters,
            login_keep_alive_timer,
            character_keep_alive_timer,
            map_keep_alive_timer,
            player_name,
            #[cfg(feature = "debug_network")]
            packet_history,
        }
    }

    pub fn get_login_settings(&self) -> &LoginSettings {
        &self.login_settings
    }

    pub fn toggle_remember_username(&mut self) {
        self.login_settings.remember_username = !self.login_settings.remember_username;
    }

    pub fn toggle_remember_password(&mut self) {
        self.login_settings.remember_password = !self.login_settings.remember_password;
    }

    pub fn log_in(&mut self, username: String, password: String) -> Result<CharacterSelectionWindow, String> {
        #[cfg(feature = "debug_network")]
        let timer = Timer::new("log in");

        self.send_packet_to_login_server(LoginServerLoginPacket::new(username.clone(), password.clone()));

        let response = self.get_data_from_login_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(login_failed_packet) = LoginFailedPacket::try_from_bytes(&mut byte_stream) {
            match login_failed_packet.reason {
                LoginFailedReason::ServerClosed => return Err("server closed".to_string()),
                LoginFailedReason::AlreadyLoggedIn => return Err("someone has already logged in with this id".to_string()),
                LoginFailedReason::AlreadyOnline => return Err("already online".to_string()),
            }
        }

        if let Ok(login_failed_packet) = LoginFailedPacket2::try_from_bytes(&mut byte_stream) {
            match login_failed_packet.reason {
                LoginFailedReason2::UnregisteredId => return Err("unregistered id".to_string()),
                LoginFailedReason2::IncorrectPassword => return Err("incorrect password".to_string()),
                LoginFailedReason2::IdExpired => return Err("id has expired".to_string()),
                LoginFailedReason2::RejectedFromServer => return Err("rejected from server".to_string()),
                LoginFailedReason2::BlockedByGMTeam => return Err("blocked by gm team".to_string()),
                LoginFailedReason2::GameOutdated => return Err("game outdated".to_string()),
                LoginFailedReason2::LoginProhibitedUntil => return Err("login prohibited until".to_string()),
                LoginFailedReason2::ServerFull => return Err("server is full".to_string()),
                LoginFailedReason2::CompanyAccountLimitReached => return Err("company account limit reached".to_string()),
            }
        }

        let login_server_login_success_packet = LoginServerLoginSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();
        self.login_data = LoginData::new(
            login_server_login_success_packet.account_id,
            login_server_login_success_packet.login_id1,
            login_server_login_success_packet.sex,
        )
        .into();

        let character_server_information = login_server_login_success_packet
            .character_server_information
            .into_iter()
            .next()
            .ok_or("no character server available")?;

        let server_ip = IpAddr::V4(character_server_information.server_ip);
        let socket_address = SocketAddr::new(server_ip, character_server_information.server_port);
        self.character_stream = TcpStream::connect_timeout(&socket_address, Duration::from_secs(1))
            .map_err(|_| "Failed to connect to character server. Please try again")?
            .into();

        let character_server_login_packet = CharacterServerLoginPacket::new(
            login_server_login_success_packet.account_id,
            login_server_login_success_packet.login_id1,
            login_server_login_success_packet.login_id2,
            login_server_login_success_packet.sex,
        );

        let character_stream = self.character_stream.as_mut().ok_or("no character server connection")?;
        character_stream
            .write_all(&character_server_login_packet.to_bytes())
            .map_err(|_| "failed to send packet to character server")?;

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        let response = self.get_data_from_character_server();

        let mut byte_stream = ByteStream::new(&response);
        let account_id = AccountId::from_bytes(&mut byte_stream, None);
        assert_eq!(account_id, login_server_login_success_packet.account_id);

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(login_failed_packet) = LoginFailedPacket::try_from_bytes(&mut byte_stream) {
            match login_failed_packet.reason {
                LoginFailedReason::ServerClosed => return Err("server closed".to_string()),
                LoginFailedReason::AlreadyLoggedIn => return Err("someone has already logged in with this id".to_string()),
                LoginFailedReason::AlreadyOnline => return Err("already online".to_string()),
            }
        }

        let character_server_login_success_packet = CharacterServerLoginSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        self.send_packet_to_character_server(RequestCharacterListPacket::default());

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        let request_character_list_success_packet = RequestCharacterListSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();
        self.characters.set(request_character_list_success_packet.character_information);

        self.login_settings.username = match self.login_settings.remember_username {
            true => username,
            // clear in case it was previously saved
            false => String::new(),
        };

        self.login_settings.password = match self.login_settings.remember_password {
            true => password,
            // clear in case it was previously saved
            false => String::new(),
        };

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        #[cfg(feature = "debug_network")]
        timer.stop();

        Ok(CharacterSelectionWindow::new(
            self.characters.clone(),
            self.move_request.clone(),
            character_server_login_success_packet.normal_slot_count as usize,
        ))
    }

    pub fn log_out(&mut self) -> Result<(), String> {
        #[cfg(feature = "debug_network")]
        let timer = Timer::new("log out");

        #[cfg(feature = "debug_network")]
        timer.stop();

        Ok(())
    }

    fn send_packet_to_login_server<T>(&mut self, packet: T)
    where
        T: Packet + 'static,
    {
        #[cfg(feature = "debug_network")]
        self.packet_history
            .push(PacketEntry::new_outgoing(&packet, T::PACKET_NAME, T::IS_PING));

        let packet_bytes = packet.to_bytes();
        self.login_stream
            .write_all(&packet_bytes)
            .expect("failed to send packet to login server");
    }

    fn send_packet_to_character_server<T>(&mut self, packet: T)
    where
        T: Packet + 'static,
    {
        #[cfg(feature = "debug_network")]
        self.packet_history
            .push(PacketEntry::new_outgoing(&packet, T::PACKET_NAME, T::IS_PING));

        let packet_bytes = packet.to_bytes();
        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        character_stream
            .write_all(&packet_bytes)
            .expect("failed to send packet to character server");
    }

    fn send_packet_to_map_server<T>(&mut self, packet: T)
    where
        T: Packet + 'static,
    {
        #[cfg(feature = "debug_network")]
        self.packet_history
            .push(PacketEntry::new_outgoing(&packet, T::PACKET_NAME, T::IS_PING));

        let packet_bytes = packet.to_bytes();
        let map_stream = self.map_stream.as_mut().expect("no map server connection");
        map_stream.write_all(&packet_bytes).expect("failed to send packet to map server");
    }

    fn get_data_from_login_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let response_lenght = self
            .login_stream
            .read(&mut buffer)
            .expect("failed to get response from login server");
        buffer[..response_lenght].to_vec()
    }

    fn get_data_from_character_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        let response_lenght = character_stream
            .read(&mut buffer)
            .expect("failed to get response from character server");
        buffer[..response_lenght].to_vec()
    }

    fn get_data_from_map_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let map_stream = self.map_stream.as_mut().expect("no map server connection");
        let response_lenght = map_stream.read(&mut buffer).expect("failed to get response from map server");
        buffer[..response_lenght].to_vec()
    }

    fn try_get_data_from_map_server(&mut self) -> Option<Vec<u8>> {
        let mut buffer = [0; 8096];
        let map_stream = self.map_stream.as_mut()?;
        map_stream.set_read_timeout(Duration::from_micros(1).into()).unwrap();
        let response_lenght = map_stream.read(&mut buffer).ok()?;

        match response_lenght {
            // TODO: make sure this will always work
            1400 => {
                let mut first_buffer = buffer[..response_lenght].to_vec();
                let mut second_buffer = self.try_get_data_from_map_server().unwrap();
                first_buffer.append(&mut second_buffer);

                println!("combined {}", first_buffer.len());
                Some(first_buffer)
            }
            length => Some(buffer[..length].to_vec()),
        }
    }

    pub fn keep_alive(&mut self, delta_time: f64, client_tick: ClientTick) {
        if self.login_keep_alive_timer.update(delta_time) {
            self.send_packet_to_login_server(LoginServerKeepalivePacket::default());
        }

        if self.character_keep_alive_timer.update(delta_time) && self.character_stream.is_some() {
            self.send_packet_to_character_server(CharacterServerKeepalivePacket::new());
        }

        if self.map_keep_alive_timer.update(delta_time) && self.map_stream.is_some() {
            self.send_packet_to_map_server(RequestServerTickPacket::new(client_tick));
        }
    }

    pub fn create_character(&mut self, slot: usize, name: String) -> Result<(), String> {
        let hair_color = 0;
        let hair_style = 0;
        let start_job = 0;
        let sex = Sex::Male;

        self.send_packet_to_character_server(CreateCharacterPacket::new(
            name, slot as u8, hair_color, hair_style, start_job, sex,
        ));

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(character_creation_failed_packet) = CharacterCreationFailedPacket::try_from_bytes(&mut byte_stream) {
            match character_creation_failed_packet.reason {
                CharacterCreationFailedReason::CharacterNameAlreadyUsed => return Err("character name is already used".to_string()),
                CharacterCreationFailedReason::NotOldEnough => return Err("you are not old enough to create a character".to_string()),
                CharacterCreationFailedReason::NotAllowedToUseSlot => {
                    return Err("you are not allowed to use that character slot".to_string());
                }
                CharacterCreationFailedReason::CharacterCerationFailed => return Err("character creation failed".to_string()),
            }
        }

        let create_character_success_packet = CreateCharacterSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        self.characters.push(create_character_success_packet.character_information);
        Ok(())
    }

    pub fn delete_character(&mut self, character_id: CharacterId) -> Result<(), String> {
        let email = "a@a.com".to_string();

        self.send_packet_to_character_server(DeleteCharacterPacket::new(character_id, email));

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(character_creation_failed_packet) = CharacterDeletionFailedPacket::try_from_bytes(&mut byte_stream) {
            match character_creation_failed_packet.reason {
                CharacterDeletionFailedReason::NotAllowed => return Err("you are not allowed to delete this character".to_string()),
                CharacterDeletionFailedReason::CharacterNotFound => return Err("character was not found".to_string()),
                CharacterDeletionFailedReason::NotEligible => return Err("character is not eligible for deletion".to_string()),
            }
        }

        CharacterDeletionSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        self.characters.retain(|character| character.character_id != character_id);
        Ok(())
    }

    pub fn select_character(&mut self, slot: usize) -> Result<(CharacterInformation, String), String> {
        self.send_packet_to_character_server(SelectCharacterPacket::new(slot as u8));

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(character_selection_failed_packet) = CharacterSelectionFailedPacket::try_from_bytes(&mut byte_stream) {
            match character_selection_failed_packet.reason {
                CharacterSelectionFailedReason::RejectedFromServer => return Err("rejected from server".to_string()),
            }
        }

        if let Ok(login_failed_packet) = LoginFailedPacket::try_from_bytes(&mut byte_stream) {
            match login_failed_packet.reason {
                LoginFailedReason::ServerClosed => return Err("Server closed".to_string()),
                LoginFailedReason::AlreadyLoggedIn => return Err("Someone has already logged in with this ID".to_string()),
                LoginFailedReason::AlreadyOnline => return Err("Already online".to_string()),
            }
        }

        if let Ok(_map_server_unavailable_packet) = MapServerUnavailablePacket::try_from_bytes(&mut byte_stream) {
            return Err("Map server currently unavailable".to_string());
        }

        let character_selection_success_packet = CharacterSelectionSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        let server_ip = IpAddr::V4(character_selection_success_packet.map_server_ip);
        let socket_address = SocketAddr::new(server_ip, character_selection_success_packet.map_server_port);
        self.map_stream = TcpStream::connect_timeout(&socket_address, Duration::from_secs(1))
            .map_err(|_| "Failed to connect to map server. Please try again")?
            .into();

        let login_data = self.login_data.as_ref().unwrap();
        self.send_packet_to_map_server(MapServerLoginPacket::new(
            login_data.account_id,
            character_selection_success_packet.character_id,
            login_data.login_id1,
            ClientTick(100), // TODO: what is the logic here?
            login_data.sex,
        ));

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        let character_information = self
            .characters
            .borrow()
            .iter()
            .find(|character| character.character_number as usize == slot)
            .cloned()
            .unwrap();

        self.player_name = character_information.name.clone();

        Ok((
            character_information,
            character_selection_success_packet.map_name.replace(".gat", ""),
        ))
    }

    pub fn request_switch_character_slot(&mut self, origin_slot: usize) {
        self.move_request.set(Some(origin_slot));
    }

    pub fn cancel_switch_character_slot(&mut self) {
        self.move_request.take();
    }

    pub fn switch_character_slot(&mut self, destination_slot: usize) -> Result<(), String> {
        let origin_slot = self.move_request.take().unwrap();

        self.send_packet_to_character_server(SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16));

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        let switch_character_slot_response_packet = SwitchCharacterSlotResponsePacket::try_from_bytes(&mut byte_stream).unwrap();

        match switch_character_slot_response_packet.status {
            SwitchCharacterSlotResponseStatus::Success => {
                let _character_server_login_success_packet = CharacterServerLoginSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();
                let _packet_006b = Packet6b00::try_from_bytes(&mut byte_stream).unwrap();

                let character_count = self.characters.len();
                self.characters.clear();

                for _index in 0..character_count {
                    let character_information = CharacterInformation::from_bytes(&mut byte_stream, None);
                    self.characters.push(character_information);
                }

                // packet_length and packet 0xa0 0x09 are left unread because we
                // don't need them
            }
            SwitchCharacterSlotResponseStatus::Error => return Err("failed to move character to a different slot".to_string()),
        }

        #[cfg(feature = "debug_network")]
        byte_stream.transfer_packet_history(&mut self.packet_history);

        self.move_request.take();
        Ok(())
    }

    pub fn request_player_move(&mut self, destination: Vector2<usize>) {
        self.send_packet_to_map_server(RequestPlayerMovePacket::new(WorldPosition::new(destination.x, destination.y)));
    }

    pub fn request_warp_to_map(&mut self, map_name: String, position: Vector2<usize>) {
        self.send_packet_to_map_server(RequestWarpToMapPacket::new(map_name, position.x as u16, position.y as u16));
    }

    pub fn map_loaded(&mut self) {
        self.send_packet_to_map_server(MapLoadedPacket::default());
    }

    pub fn request_entity_details(&mut self, entity_id: EntityId) {
        self.send_packet_to_map_server(RequestDetailsPacket::new(entity_id));
    }

    pub fn request_player_attack(&mut self, entity_id: EntityId) {
        self.send_packet_to_map_server(RequestActionPacket::new(entity_id, Action::Attack));
    }

    pub fn send_message(&mut self, message: String) {
        let complete_message = format!("{} : {}", self.player_name, message);

        self.send_packet_to_map_server(GlobalMessagePacket::new(
            complete_message.bytes().len() as u16 + 5,
            complete_message,
        ));
    }

    pub fn start_dialog(&mut self, npc_id: EntityId) {
        self.send_packet_to_map_server(StartDialogPacket::new(npc_id));
    }

    pub fn next_dialog(&mut self, npc_id: EntityId) {
        self.send_packet_to_map_server(NextDialogPacket::new(npc_id));
    }

    pub fn close_dialog(&mut self, npc_id: EntityId) {
        self.send_packet_to_map_server(CloseDialogPacket::new(npc_id));
    }

    pub fn choose_dialog_option(&mut self, npc_id: EntityId, option: i8) {
        self.send_packet_to_map_server(ChooseDialogOptionPacket::new(npc_id, option));
    }

    pub fn request_item_equip(&mut self, item_index: ItemIndex, equip_position: EquipPosition) {
        self.send_packet_to_map_server(RequestEquipItemPacket::new(item_index, equip_position));
    }

    pub fn request_item_unequip(&mut self, item_index: ItemIndex) {
        self.send_packet_to_map_server(RequestUnequipItemPacket::new(item_index));
    }

    pub fn network_events(&mut self) -> Vec<NetworkEvent> {
        let mut events = Vec::new();

        while let Some(data) = self.try_get_data_from_map_server() {
            let mut byte_stream = ByteStream::new(&data);

            while !byte_stream.is_empty() {
                if let Ok(packet) = BroadcastMessagePacket::try_from_bytes(&mut byte_stream) {
                    let chat_message = ChatMessage::new(packet.message, packet.font_color.into());
                    events.push(NetworkEvent::ChatMessage(chat_message));
                } else if let Ok(packet) = ServerMessagePacket::try_from_bytes(&mut byte_stream) {
                    let chat_message = ChatMessage::new(packet.message, Color::monochrome(255));
                    events.push(NetworkEvent::ChatMessage(chat_message));
                } else if let Ok(packet) = EntityMessagePacket::try_from_bytes(&mut byte_stream) {
                    let chat_message = ChatMessage::new(packet.message, packet.color.into());
                    events.push(NetworkEvent::ChatMessage(chat_message));
                } else if let Ok(_) = DisplayEmotionPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(packet) = EntityMovePacket::try_from_bytes(&mut byte_stream) {
                    let (origin, destination) = packet.from_to.to_vectors();
                    events.push(NetworkEvent::EntityMove(
                        packet.entity_id,
                        origin,
                        destination,
                        packet.timestamp,
                    ));
                } else if let Ok(_) = EntityStopMovePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(packet) = PlayerMovePacket::try_from_bytes(&mut byte_stream) {
                    let (origin, destination) = packet.from_to.to_vectors();
                    events.push(NetworkEvent::PlayerMove(origin, destination, packet.timestamp));
                } else if let Ok(packet) = ChangeMapPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::ChangeMap(
                        packet.map_name.replace(".gat", ""),
                        Vector2::new(packet.x as usize, packet.y as usize),
                    ));
                } else if let Ok(packet) = EntityAppearedPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::AddEntity(packet.into()));
                } else if let Ok(packet) = MovingEntityAppearedPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::AddEntity(packet.into()));
                } else if let Ok(packet) = EntityDisappearedPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::RemoveEntity(packet.entity_id));
                } else if let Ok(packet) = UpdateStatusPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateStatus(packet.status_type));
                } else if let Ok(packet) = UpdateStatusPacket1::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateStatus(packet.status_type));
                } else if let Ok(packet) = UpdateStatusPacket2::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateStatus(packet.status_type));
                } else if let Ok(packet) = UpdateStatusPacket3::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateStatus(packet.status_type));
                } else if let Ok(_) = UpdateAttackRangePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = NewMailStatusPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = AchievementUpdatePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = AchievementListPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = CriticalWeightUpdatePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = SpriteChangePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = InventoyStartPacket::try_from_bytes(&mut byte_stream) {
                    let mut item_data = Vec::new();

                    while InventoyEndPacket::try_from_bytes(&mut byte_stream).is_err() {
                        if let Ok(packet) = RegularItemListPacket::try_from_bytes(&mut byte_stream) {
                            for item_information in packet.item_information {
                                item_data.push((
                                    item_information.index,
                                    item_information.item_id,
                                    EquipPosition::None,
                                    EquipPosition::None,
                                )); // TODO: Don't add that data here, only equippable itemes need this data
                            }
                        } else if let Ok(packet) = EquippableItemListPacket::try_from_bytes(&mut byte_stream) {
                            for item_information in packet.item_information {
                                item_data.push((
                                    item_information.index,
                                    item_information.item_id,
                                    item_information.equip_position,
                                    item_information.equipped_position,
                                ));
                            }
                        } else {
                            panic!("unexpected packet with header: {:x?}", byte_stream.slice(2));
                        }
                    }

                    events.push(NetworkEvent::Inventory(item_data));
                } else if let Ok(_) = EquippableSwitchItemListPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = MapTypePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = UpdateSkillTreePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = UpdateHotkeysPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = InitialStatusPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = UpdatePartyInvitationStatePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = UpdateShowEquipPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = UpdateConfigurationPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = NavigateToMonsterPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = MarkMinimapPositionPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = NextButtonPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::AddNextButton);
                } else if let Ok(_) = CloseButtonPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::AddCloseButton);
                } else if let Ok(packet) = DialogMenuPacket::try_from_bytes(&mut byte_stream) {
                    let choices = packet
                        .message
                        .split(':')
                        .map(String::from)
                        .filter(|text| !text.is_empty())
                        .collect();

                    events.push(NetworkEvent::AddChoiceButtons(choices));
                } else if let Ok(_) = DisplaySpecialEffectPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = DisplaySkillEffectPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = StatusChangePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = QuestNotificationPacket1::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = HuntingQuestNotificationPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = HuntingQuestUpdateObjectivePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = QuestRemovedPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = QuestListPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = VisualEffectPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = DisplayGainedExperiencePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = DisplayImagePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = StateChangePacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(packet) = QuestEffectPacket::try_from_bytes(&mut byte_stream) {
                    let event = match packet.effect {
                        QuestEffect::None => NetworkEvent::RemoveQuestEffect(packet.entity_id),
                        _ => NetworkEvent::AddQuestEffect(packet),
                    };
                    events.push(event);
                } else if let Ok(packet) = ItemPickupPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::AddIventoryItem(
                        packet.index,
                        packet.item_id,
                        packet.equip_position,
                        EquipPosition::None,
                    ));
                } else if let Ok(_) = RemoveItemFromInventoryPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(packet) = ServerTickPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateClientTick(packet.client_tick));
                } else if let Ok(packet) = RequestPlayerDetailsSuccessPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateEntityDetails(EntityId(packet.character_id.0), packet.name));
                } else if let Ok(packet) = RequestEntityDetailsSuccessPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateEntityDetails(packet.entity_id, packet.name));
                } else if let Ok(packet) = UpdateEntityHealthPointsPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateEntityHealth(
                        packet.entity_id,
                        packet.health_points as usize,
                        packet.maximum_health_points as usize,
                    ));
                } else if let Ok(_) = RequestPlayerAttackFailedPacket::try_from_bytes(&mut byte_stream) {
                } else if let Ok(packet) = DamagePacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::DamageEffect(
                        packet.destination_entity_id,
                        packet.damage_amount as usize,
                    ));
                } else if let Ok(packet) = NpcDialogPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::OpenDialog(packet.text, packet.npc_id));
                } else if let Ok(packet) = RequestEquipItemStatusPacket::try_from_bytes(&mut byte_stream) {
                    if let RequestEquipItemStatus::Success = packet.result {
                        events.push(NetworkEvent::UpdateEquippedPosition {
                            index: packet.inventory_index,
                            equipped_position: packet.equipped_position,
                        });
                    }
                } else if let Ok(packet) = RequestUnequipItemStatusPacket::try_from_bytes(&mut byte_stream) {
                    if let RequestUnequipItemStatus::Success = packet.result {
                        events.push(NetworkEvent::UpdateEquippedPosition {
                            index: packet.inventory_index,
                            equipped_position: EquipPosition::None,
                        });
                    }
                } else if let Ok(_) = Packet8302::try_from_bytes(&mut byte_stream) {
                } else if let Ok(_) = Packet180b::try_from_bytes(&mut byte_stream) {
                } else if let Ok(packet) = MapServerLoginSuccessPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdateClientTick(packet.client_tick))
                } else {
                    #[cfg(feature = "debug_network")]
                    {
                        let remaining_bytes = byte_stream.remaining_bytes();
                        byte_stream.incoming_unknown_packet(remaining_bytes);
                    }

                    break;
                }
            }

            #[cfg(feature = "debug_network")]
            byte_stream.transfer_packet_history(&mut self.packet_history);
        }

        events
    }

    #[cfg(feature = "debug_network")]
    pub fn packets(&self) -> TrackedState<Vec<PacketEntry>> {
        self.packet_history.clone()
    }
}
