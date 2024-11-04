pub mod handler;
mod position;

use std::net::Ipv4Addr;

use ragnarok_bytes::{
    ByteConvertable, ByteStream, ConversionError, ConversionResult, ConversionResultExt, FixedByteSize, FromBytes, ToBytes,
};
#[cfg(feature = "derive")]
pub use ragnarok_procedural::{CharacterServer, ClientPacket, LoginServer, MapServer, Packet, ServerPacket};
#[cfg(not(feature = "derive"))]
use ragnarok_procedural::{CharacterServer, ClientPacket, LoginServer, MapServer, Packet, ServerPacket};

pub use self::position::{WorldPosition, WorldPosition2};

// To make proc macros work in korangar_interface.
extern crate self as ragnarok_packets;

/// The header of a Ragnarok Online packet. It is always two bytes long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ByteConvertable, PartialOrd, Ord, Hash)]
pub struct PacketHeader(pub u16);

/// Base trait that all packets implement.
/// All packets in Ragnarok online consist of a header, two bytes in size,
/// followed by the packet data. If the packet does not have a fixed size,
/// the first two bytes will be the size of the packet in bytes *including* the
/// header. Packets are sent in little endian.
pub trait Packet: std::fmt::Debug + Clone {
    /// Any scheduled packet that does not depend on in-game events should be
    /// marked as a ping. This is mostly for filtering when logging
    /// packet traffic.
    const IS_PING: bool;
    /// The header of the Packet.
    const HEADER: PacketHeader;

    /// Read packet **without the header**. To read the packet with the header,
    /// use [`PacketExt::packet_from_bytes`].
    fn payload_from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self>;

    /// Write packet **without the header**. To write the packet with the
    /// header, use [`PacketExt::packet_to_bytes`].
    fn payload_to_bytes(&self) -> ConversionResult<Vec<u8>>;

    /// Implementation detail of Korangar. Can be used to convert a packet to an
    /// UI element in the packet viewer.
    #[cfg(feature = "packet-to-prototype-element")]
    fn to_prototype_element<App: korangar_interface::application::Application>(
        &self,
    ) -> Box<dyn korangar_interface::elements::PrototypeElement<App> + Send>;
}

/// Extension trait for reading and writing packets with the header.
pub trait PacketExt: Packet {
    /// Read packet **with the header**. To read the packet without the header,
    /// use [`Packet::payload_from_bytes`].
    fn packet_from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self>;

    /// Write packet **with the header**. To write the packet without the
    /// header, use [`Packet::payload_to_bytes`].
    fn packet_to_bytes(&self) -> ConversionResult<Vec<u8>>;
}

impl<T> PacketExt for T
where
    T: Packet,
{
    fn packet_from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let header = PacketHeader::from_bytes(byte_stream)?;

        if header != Self::HEADER {
            return Err(ConversionError::from_message("mismatched header"));
        }

        Self::payload_from_bytes(byte_stream)
    }

    fn packet_to_bytes(&self) -> ConversionResult<Vec<u8>> {
        let mut bytes = Self::HEADER.to_bytes()?;

        bytes.extend(self.payload_to_bytes()?);

        Ok(bytes)
    }
}

/// Marker trait for packets sent by the client.
pub trait ClientPacket: Packet {}

/// Marker trait for packets sent by the server.
pub trait ServerPacket: Packet {}

/// Marker trait for login server packets.
pub trait LoginServerPacket: Packet {}

/// Marker trait for character server packets.
pub trait CharacterServerPacket: Packet {}

/// Marker trait for map server packets.
pub trait MapServerPacket: Packet {}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ClientTick(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct AccountId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct CharacterId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct PartyId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct EntityId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SkillId(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SkillLevel(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct HotbarTab(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct HotbarSlot(pub u16);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ShopId(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Price(pub u32);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ServerAddress(pub [u8; 4]);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct UserId(pub [u8; 24]);

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct AuthToken(pub [u8; 17]);

impl From<ServerAddress> for Ipv4Addr {
    fn from(value: ServerAddress) -> Self {
        value.0.into()
    }
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct TilePosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct LargeTilePosition {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ColorBGRA {
    pub blue: u8,
    pub green: u8,
    pub red: u8,
    pub alpha: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ColorRGBA {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

/// Item index is always actual index + 2.
#[derive(Clone, Copy, Debug, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct InventoryIndex(pub u16);

impl FromBytes for InventoryIndex {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        u16::from_bytes(byte_stream).map(|raw| Self(raw - 2))
    }
}

impl ToBytes for InventoryIndex {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        u16::to_bytes(&(self.0 + 2))
    }
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ItemId(pub u32);

#[derive(Copy, Debug, Clone, ByteConvertable, FixedByteSize, PartialEq)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum Sex {
    Female,
    Male,
    Both,
    Server,
}

/// Sent by the client to the login server.
/// The very first packet sent when logging in, it is sent after the user has
/// entered email and password.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0064)]
pub struct LoginServerLoginPacket {
    /// Unused
    #[new_default]
    pub version: [u8; 4],
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub password: String,
    /// Unused
    #[new_default]
    pub client_type: u8,
}

/// Sent by the login server as a response to [LoginServerLoginPacket]
/// succeeding. After receiving this packet, the client will connect to one of
/// the character servers provided by this packet.
#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0AC4)]
#[variable_length]
pub struct LoginServerLoginSuccessPacket {
    pub login_id1: u32,
    pub account_id: AccountId,
    pub login_id2: u32,
    /// Deprecated and always 0 on rAthena
    #[new_default]
    pub ip_address: u32,
    /// Deprecated and always 0 on rAthena
    #[new_default]
    pub name: [u8; 24],
    /// Always 0 on rAthena
    #[new_default]
    pub unknown: u16,
    pub sex: Sex,
    pub auth_token: AuthToken,
    #[repeating_remaining]
    pub character_server_information: Vec<CharacterServerInformation>,
}

/// Sent by the character server as a response to [CharacterServerLoginPacket]
/// succeeding. Provides basic information about the number of available
/// character slots.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x082D)]
pub struct CharacterServerLoginSuccessPacket {
    /// Always 29 on rAthena
    pub unknown: u16,
    pub normal_slot_count: u8,
    pub vip_slot_count: u8,
    pub billing_slot_count: u8,
    pub poducilble_slot_count: u8,
    pub vaild_slot: u8,
    #[new_default]
    pub unused: [u8; 20],
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x006B)]
pub struct Packet006b {
    pub unused: u16,
    pub maximum_slot_count: u8,
    pub available_slot_count: u8,
    pub vip_slot_count: u8,
    #[new_default]
    pub unknown: [u8; 20],
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B18)]
pub struct Packet0b18 {
    /// Possibly inventory related
    #[new_default]
    pub unknown: u16,
}

/// Sent by the map server as a response to [MapServerLoginPacket] succeeding.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02EB)]
pub struct MapServerLoginSuccessPacket {
    pub client_tick: ClientTick,
    pub position: WorldPosition,
    /// Always [5, 5] on rAthena
    #[new_default]
    pub ignored: [u8; 2],
    pub font: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum LoginFailedReason {
    #[numeric_value(1)]
    ServerClosed,
    #[numeric_value(2)]
    AlreadyLoggedIn,
    #[numeric_value(8)]
    AlreadyOnline,
}

#[derive(Debug, Clone, Packet, ServerPacket, LoginServer, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0081)]
pub struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0840)]
#[variable_length]
pub struct MapServerUnavailablePacket {
    #[length_remaining]
    pub unknown: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
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

#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x083E)]
pub struct LoginFailedPacket2 {
    pub reason: LoginFailedReason2,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum CharacterSelectionFailedReason {
    RejectedFromServer,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// failing. Provides a reason for the character selection failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x006C)]
pub struct CharacterSelectionFailedPacket {
    pub reason: CharacterSelectionFailedReason,
}

/// Sent by the character server as a response to [SelectCharacterPacket]
/// succeeding. Provides a map server to connect to, along with the ID of our
/// selected character.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0AC5)]
pub struct CharacterSelectionSuccessPacket {
    pub character_id: CharacterId,
    #[length(16)]
    pub map_name: String,
    pub map_server_ip: ServerAddress,
    pub map_server_port: u16,
    // NOTE: Could be `new_default` but Rust doesn't implement `[u8; 128]: Default`.
    #[new_value([0; 128])]
    pub unknown: [u8; 128],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
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
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x006E)]
pub struct CharacterCreationFailedPacket {
    pub reason: CharacterCreationFailedReason,
}

/// Sent by the client to the login server every 60 seconds to keep the
/// connection alive.
#[derive(Debug, Clone, Packet, ClientPacket, LoginServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0200)]
#[ping]
pub struct LoginServerKeepalivePacket {
    #[new_value(UserId([0; 24]))]
    pub user_id: UserId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct CharacterServerInformation {
    pub server_ip: ServerAddress,
    pub server_port: u16,
    #[length(20)]
    pub server_name: String,
    pub user_count: u16,
    pub server_type: u16, // ServerType
    pub display_new: u16, // bool16 ?
    #[new_value([0; 128])]
    pub unknown: [u8; 128],
}

/// Sent by the client to the character server after after successfully logging
/// into the login server.
/// Attempts to log into the character server using the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0065)]
pub struct CharacterServerLoginPacket {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    #[new_default]
    pub unknown: u16,
    pub sex: Sex,
}

/// Sent by the client to the map server after after successfully selecting a
/// character. Attempts to log into the map server using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0436)]
pub struct MapServerLoginPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub login_id1: u32,
    pub client_tick: ClientTick,
    pub sex: Sex,
    #[new_default]
    pub unknown: [u8; 4],
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0283)]
pub struct Packet8302 {
    pub entity_id: EntityId,
}

/// Sent by the client to the character server when the player tries to create
/// a new character.
/// Attempts to create a new character in an empty slot using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0A39)]
pub struct CreateCharacterPacket {
    #[length(24)]
    pub name: String,
    pub slot: u8,
    pub hair_color: u16, // TODO: HairColor
    pub hair_style: u16, // TODO: HairStyle
    pub start_job: u16,  // TODO: Job
    #[new_default]
    pub unknown: [u8; 2],
    pub sex: Sex,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
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
    #[length(24)]
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
    #[length(16)]
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
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B6F)]
pub struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}

/// Sent by the client to the character server.
/// Requests a list of every character associated with the account.
#[derive(Debug, Clone, Default, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09A1)]
pub struct RequestCharacterListPacket {}

/// Sent by the character server as a response to [RequestCharacterListPacket]
/// succeeding. Provides the requested list of character information.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B72)]
#[variable_length]
pub struct RequestCharacterListSuccessPacket {
    #[repeating_remaining]
    pub character_information: Vec<CharacterInformation>,
}

/// Sent by the map server to the client.
#[derive(Debug, Clone, Default, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B1D)]
#[ping]
pub struct MapServerPingPacket {}

/// Sent by the client to the map server when the player wants to move.
/// Attempts to path the player towards the provided position.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0881)]
pub struct RequestPlayerMovePacket {
    pub position: WorldPosition,
}

/// Sent by the client to the map server when the player wants to warp.
/// Attempts to warp the player to a specific position on a specific map using
/// the provided information.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0140)]
pub struct RequestWarpToMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

/// Sent by the map server to the client.
/// Informs the client that an entity is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0086)]
pub struct EntityMovePacket {
    pub entity_id: EntityId,
    pub from_to: WorldPosition2,
    pub timestamp: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0088)]
pub struct EntityStopMovePacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
}

/// Sent by the map server to the client.
/// Informs the client that the player is pathing towards a new position.
/// Provides the initial position and destination of the movement, as well as a
/// timestamp of when it started (for synchronization).
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0087)]
pub struct PlayerMovePacket {
    pub timestamp: ClientTick,
    pub from_to: WorldPosition2,
}

/// Sent by the client to the character server when the user tries to delete a
/// character.
/// Attempts to delete a character from the user account using the provided
/// information.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x01FB)]
pub struct DeleteCharacterPacket {
    pub character_id: CharacterId,
    /// This field can be used for email or date of birth, depending on the
    /// configuration of the character server.
    #[length(40)]
    pub email: String,
    /// Ignored by rAthena
    #[new_default]
    pub unknown: [u8; 10],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum CharacterDeletionFailedReason {
    NotAllowed,
    CharacterNotFound,
    NotEligible,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// failing. Provides a reason for the character deletion failing.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0070)]
pub struct CharacterDeletionFailedPacket {
    pub reason: CharacterDeletionFailedReason,
}

/// Sent by the character server as a response to [DeleteCharacterPacket]
/// succeeding.
#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x006F)]
pub struct CharacterDeletionSuccessPacket {}

/// Sent by the client to the character server when the user selects a
/// character. Attempts to select the character in the specified slot.
#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0066)]
pub struct SelectCharacterPacket {
    pub selected_slot: u8,
}

/// Sent by the map server to the client when there is a new chat message from
/// the server. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x008E)]
#[variable_length]
pub struct ServerMessagePacket {
    #[length_remaining]
    pub message: String,
}

/// Sent by the client to the map server when the user hovers over an entity.
/// Attempts to fetch additional information about the entity, such as the
/// display name.
#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0368)]
pub struct RequestDetailsPacket {
    pub entity_id: EntityId,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the player.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0A30)]
pub struct RequestPlayerDetailsSuccessPacket {
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub party_name: String,
    #[length(24)]
    pub guild_name: String,
    #[length(24)]
    pub position_name: String,
    pub title_id: u32,
}

/// Sent by the map server to the client as a response to
/// [RequestDetailsPacket]. Provides additional information about the entity.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0ADF)]
pub struct RequestEntityDetailsSuccessPacket {
    pub entity_id: EntityId,
    pub group_id: u32,
    #[length(24)]
    pub name: String,
    #[length(24)]
    pub title: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09E7)]
pub struct NewMailStatusPacket {
    pub new_available: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct AchievementData {
    pub acheivement_id: u32,
    pub is_completed: u8,
    pub objectives: [u32; 10],
    pub completion_timestamp: u32,
    pub got_rewarded: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0A24)]
pub struct AchievementUpdatePacket {
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    pub acheivement_data: AchievementData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0A23)]
#[variable_length]
pub struct AchievementListPacket {
    #[new_derive]
    pub achievement_count: u32,
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_to_next_level: u32, // "to_next_level" might be wrong
    #[repeating(achievement_count)]
    pub acheivement_data: Vec<AchievementData>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0ADE)]
pub struct CriticalWeightUpdatePacket {
    pub weight: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x01D7)]
pub struct SpriteChangePacket {
    pub account_id: AccountId,
    pub sprite_type: SpriteChangeType,
    pub value: u32,
    pub value2: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum SpriteChangeType {
    Base,
    Hair,
    Weapon,
    HeadBottom,
    HeadTop,
    HeadMiddle,
    HairCollor,
    ClothesColor,
    Shield,
    Shoes,
    Body,
    ResetCostumes,
    Robe,
    Body2,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B08)]
#[variable_length]
pub struct InventoyStartPacket {
    pub inventory_type: u8,
    #[length_remaining]
    pub inventory_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B0B)]
pub struct InventoyEndPacket {
    pub inventory_type: u8,
    pub flag: u8, // maybe char ?
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ItemOptions {
    pub index: u16,
    pub value: u16,
    pub parameter: u8,
}

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
    pub struct RegularItemFlags: u8 {
        const IDENTIFIED = 0b01;
        const IN_ETC_TAB = 0b10;
    }
}

impl FixedByteSize for RegularItemFlags {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for RegularItemFlags {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_stream).map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}

impl ToBytes for RegularItemFlags {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        self.bits().to_bytes()
    }
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct RegularItemInformation {
    pub index: InventoryIndex,
    pub item_id: ItemId,
    pub item_type: u8,
    pub amount: u16,
    pub equipped_position: EquipPosition,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: u32,
    pub flags: RegularItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B09)]
#[variable_length]
pub struct RegularItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<RegularItemInformation>,
}

bitflags::bitflags! {
    #[derive(Debug, Clone)]
    #[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
    pub struct EquippableItemFlags: u8 {
        const IDENTIFIED = 0b001;
        const IS_BROKEN = 0b010;
        const IN_ETC_TAB = 0b110;
    }
}

impl FixedByteSize for EquippableItemFlags {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for EquippableItemFlags {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_stream).map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}

impl ToBytes for EquippableItemFlags {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        self.bits().to_bytes()
    }
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
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
    pub refinement_level: u8,
    pub enchantment_level: u8,
    pub flags: EquippableItemFlags,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B39)]
#[variable_length]
pub struct EquippableItemListPacket {
    pub inventory_type: u8,
    #[repeating_remaining]
    pub item_information: Vec<EquippableItemInformation>,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct EquippableSwitchItemInformation {
    pub index: InventoryIndex,
    pub position: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0A9B)]
#[variable_length]
pub struct EquippableSwitchItemListPacket {
    #[repeating_remaining]
    pub item_information: Vec<EquippableSwitchItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x099B)]
pub struct MapTypePacket {
    pub map_type: u16,
    pub flags: u32,
}

/// Sent by the map server to the client when there is a new chat message from
/// ??. Provides the message to be displayed in the chat window, as well as
/// information on how the message should be displayed.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x01C3)]
#[variable_length]
pub struct Broadcast2MessagePacket {
    pub font_color: ColorRGBA,
    pub font_type: u16,
    pub font_size: u16,
    pub font_alignment: u16,
    pub font_y: u16,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone uses the @broadcast
/// command. Provides the message to be displayed in the chat window.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x009A)]
#[variable_length]
pub struct BroadcastMessagePacket {
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when when someone writes in proximity
/// chat. Provides the source player and message to be displayed in the chat
/// window and the speach bubble.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x008D)]
#[variable_length]
pub struct OverheadMessagePacket {
    pub entity_id: EntityId,
    #[length_remaining]
    pub message: String,
}

/// Sent by the map server to the client when there is a new chat message from
/// an entity. Provides the message to be displayed in the chat window, the
/// color of the message, and the ID of the entity it originated from.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02C1)]
#[variable_length]
pub struct EntityMessagePacket {
    pub entity_id: EntityId,
    pub color: ColorBGRA,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00C0)]
pub struct DisplayEmotionPacket {
    pub entity_id: EntityId,
    pub emotion: u8,
}

/// Every value that can be set from the server through [UpdateStatusPacket],
/// [UpdateStatusPacket1], [UpdateStatusPacket2], and [UpdateStatusPacket3].
/// All UpdateStatusPackets do the same, they just have different sizes
/// correlating to the space the updated value requires.
#[derive(Debug, Clone)]
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

impl FromBytes for StatusType {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        let status = match u16::from_bytes(byte_stream).trace::<Self>()? {
            0 => u32::from_bytes(byte_stream).map(Self::MovementSpeed),
            1 => u64::from_bytes(byte_stream).map(Self::BaseExperience),
            2 => u64::from_bytes(byte_stream).map(Self::JobExperience),
            3 => u32::from_bytes(byte_stream).map(Self::Karma),
            4 => u32::from_bytes(byte_stream).map(Self::Manner),
            5 => u32::from_bytes(byte_stream).map(Self::HealthPoints),
            6 => u32::from_bytes(byte_stream).map(Self::MaximumHealthPoints),
            7 => u32::from_bytes(byte_stream).map(Self::SpellPoints),
            8 => u32::from_bytes(byte_stream).map(Self::MaximumSpellPoints),
            9 => u32::from_bytes(byte_stream).map(Self::StatusPoint),
            11 => u32::from_bytes(byte_stream).map(Self::BaseLevel),
            12 => u32::from_bytes(byte_stream).map(Self::SkillPoint),
            13 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Strength(a, u32::from_bytes(byte_stream)?))),
            14 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Agility(a, u32::from_bytes(byte_stream)?))),
            15 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Vitality(a, u32::from_bytes(byte_stream)?))),
            16 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Intelligence(a, u32::from_bytes(byte_stream)?))),
            17 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Dexterity(a, u32::from_bytes(byte_stream)?))),
            18 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Luck(a, u32::from_bytes(byte_stream)?))),
            20 => u32::from_bytes(byte_stream).map(Self::Zeny),
            22 => u64::from_bytes(byte_stream).map(Self::NextBaseExperience),
            23 => u64::from_bytes(byte_stream).map(Self::NextJobExperience),
            24 => u32::from_bytes(byte_stream).map(Self::Weight),
            25 => u32::from_bytes(byte_stream).map(Self::MaximumWeight),
            32 => u8::from_bytes(byte_stream).map(Self::SpUstr),
            33 => u8::from_bytes(byte_stream).map(Self::SpUagi),
            34 => u8::from_bytes(byte_stream).map(Self::SpUvit),
            35 => u8::from_bytes(byte_stream).map(Self::SpUint),
            36 => u8::from_bytes(byte_stream).map(Self::SpUdex),
            37 => u8::from_bytes(byte_stream).map(Self::SpUluk),
            41 => u32::from_bytes(byte_stream).map(Self::Attack1),
            42 => u32::from_bytes(byte_stream).map(Self::Attack2),
            43 => u32::from_bytes(byte_stream).map(Self::MagicAttack1),
            44 => u32::from_bytes(byte_stream).map(Self::MagicAttack2),
            45 => u32::from_bytes(byte_stream).map(Self::Defense1),
            46 => u32::from_bytes(byte_stream).map(Self::Defense2),
            47 => u32::from_bytes(byte_stream).map(Self::MagicDefense1),
            48 => u32::from_bytes(byte_stream).map(Self::MagicDefense2),
            49 => u32::from_bytes(byte_stream).map(Self::Hit),
            50 => u32::from_bytes(byte_stream).map(Self::Flee1),
            51 => u32::from_bytes(byte_stream).map(Self::Flee2),
            52 => u32::from_bytes(byte_stream).map(Self::Critical),
            53 => u32::from_bytes(byte_stream).map(Self::AttackSpeed),
            55 => u32::from_bytes(byte_stream).map(Self::JobLevel),
            99 => u16::from_bytes(byte_stream)
                .and_then(|a| Ok(Self::CartInfo(a, u32::from_bytes(byte_stream)?, u32::from_bytes(byte_stream)?))),
            219 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Power(a, u32::from_bytes(byte_stream)?))),
            220 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Stamina(a, u32::from_bytes(byte_stream)?))),
            221 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Wisdom(a, u32::from_bytes(byte_stream)?))),
            222 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Spell(a, u32::from_bytes(byte_stream)?))),
            223 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Concentration(a, u32::from_bytes(byte_stream)?))),
            224 => u32::from_bytes(byte_stream).and_then(|a| Ok(Self::Creativity(a, u32::from_bytes(byte_stream)?))),
            225 => u32::from_bytes(byte_stream).map(Self::PhysicalAttack),
            226 => u32::from_bytes(byte_stream).map(Self::SpellMagicAttack),
            227 => u32::from_bytes(byte_stream).map(Self::Resistance),
            228 => u32::from_bytes(byte_stream).map(Self::MagicResistance),
            229 => u32::from_bytes(byte_stream).map(Self::HealingPlus),
            230 => u32::from_bytes(byte_stream).map(Self::CriticalDamageRate),
            231 => u32::from_bytes(byte_stream).map(Self::TraitPoint),
            232 => u32::from_bytes(byte_stream).map(Self::ActivityPoints),
            233 => u32::from_bytes(byte_stream).map(Self::MaximumActivityPoints),
            247 => u8::from_bytes(byte_stream).map(Self::SpUpow),
            248 => u8::from_bytes(byte_stream).map(Self::SpUsta),
            249 => u8::from_bytes(byte_stream).map(Self::SpUwis),
            250 => u8::from_bytes(byte_stream).map(Self::SpUspl),
            251 => u8::from_bytes(byte_stream).map(Self::SpUcon),
            252 => u8::from_bytes(byte_stream).map(Self::SpUcrt),
            invalid => Err(ConversionError::from_message(format!("invalid status code {invalid}"))),
        };

        status.trace::<Self>()
    }
}

impl ToBytes for StatusType {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        panic!("this should be derived");
    }
}

// TODO: make StatusType derivable
#[cfg(feature = "interface")]
impl<App: korangar_interface::application::Application> korangar_interface::elements::PrototypeElement<App> for StatusType {
    fn to_element(&self, display: String) -> korangar_interface::elements::ElementCell<App> {
        format!("{self:?}").to_element(display)
    }
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B0)]
pub struct UpdateStatusPacket {
    #[length(6)]
    pub status_type: StatusType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0196)]
pub struct StatusChangeSequencePacket {
    pub index: u16,
    pub id: u32,
    pub state: u8,
}

/// Sent by the character server to the client when loading onto a new map.
/// This packet is ignored by Korangar since all of the provided values are set
/// again individually using the UpdateStatusPackets.
#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00BD)]
pub struct InitialStatusPacket {
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
    #[new_default]
    pub bonus_attack_speed: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0141)]
pub struct UpdateStatusPacket1 {
    #[length(12)]
    pub status_type: StatusType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0ACB)]
pub struct UpdateStatusPacket2 {
    #[length(10)]
    pub status_type: StatusType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00BE)]
pub struct UpdateStatusPacket3 {
    #[length(3)]
    pub status_type: StatusType,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x013A)]
pub struct UpdateAttackRangePacket {
    pub attack_range: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x08D4)]
pub struct SwitchCharacterSlotPacket {
    pub origin_slot: u16,
    pub destination_slot: u16,
    /// 1 instead of default, just in case the sever actually uses this value
    /// (rAthena does not)
    #[new_value(1)]
    pub remaining_moves: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum Action {
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

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0437)]
pub struct RequestActionPacket {
    pub npc_id: EntityId,
    pub action: Action,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00F3)]
#[variable_length]
pub struct GlobalMessagePacket {
    #[length_remaining_off_by_one]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0139)]
pub struct RequestPlayerAttackFailedPacket {
    pub target_entity_id: EntityId,
    pub target_position: TilePosition,
    pub position: TilePosition,
    pub attack_range: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0977)]
pub struct UpdateEntityHealthPointsPacket {
    pub entity_id: EntityId,
    pub health_points: u32,
    pub maximum_health_points: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum DamageType {
    Damage,
    PickUpItem,
    SitDown,
    StandUp,
    DamageEndure, // Not confirmed
    Splash,       // Not confirmed
    Skill,        // Not confirmed
    RepeatDamage, // Not confirmed
    MultiHitDamage,
    MultiHitDamageEndure, // Not confirmed
    CriticalHit,
    LuckyDodge,
    TouchSkill, // Not confirmed
    CriticalMultiHit,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x008A)]
pub struct DamagePacket1 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub source_movement_speed: u32,
    pub destination_movement_speed: u32,
    pub damage_amount: i16,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: i16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x08C8)]
pub struct DamagePacket3 {
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub client_tick: ClientTick,
    pub source_movement_speed: u32,
    pub destination_movement_speed: u32,
    pub damage_amount: u32,
    pub is_special_damage: u8,
    pub number_of_hits: u16,
    pub damage_type: DamageType,
    /// Assassin dual wield damage.
    pub damage_amount_2: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x007F)]
#[ping]
pub struct ServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0360)]
#[ping]
pub struct RequestServerTickPacket {
    pub client_tick: ClientTick,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum SwitchCharacterSlotResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B70)]
pub struct SwitchCharacterSlotResponsePacket {
    #[new_default]
    pub unknown: u16, // is always 8 ?
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0091)]
pub struct ChangeMapPacket {
    #[length(16)]
    pub map_name: String,
    pub position: TilePosition,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum DisappearanceReason {
    OutOfSight,
    Died,
    LoggedOut,
    Teleported,
    TrickDead,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0080)]
pub struct EntityDisappearedPacket {
    pub entity_id: EntityId,
    pub reason: DisappearanceReason,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09FD)]
#[variable_length]
pub struct MovingEntityAppearedPacket {
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
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0148)]
pub struct ResurrectionPacket {
    pub entity_id: EntityId,
    /// Always 0 in rAthena.
    #[new_default]
    pub packet_type: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09FE)]
#[variable_length]
pub struct EntityAppearedPacket {
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
    pub c_level: u16,
    pub font: u16,
    pub maximum_health_points: i32,
    pub health_points: i32,
    pub is_boss: u8,
    pub body: u16,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09FF)]
#[variable_length]
pub struct EntityAppeared2Packet {
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
    #[length(24)]
    pub name: String,
}

#[derive(Clone, Copy, Debug, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u32)]
pub enum SkillType {
    #[numeric_value(0)]
    Passive,
    #[numeric_value(1)]
    Attack,
    #[numeric_value(2)]
    Ground,
    #[numeric_value(4)]
    SelfCast,
    #[numeric_value(16)]
    Support,
    #[numeric_value(32)]
    Trap,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SkillInformation {
    pub skill_id: SkillId,
    pub skill_type: SkillType,
    pub skill_level: SkillLevel,
    pub spell_point_cost: u16,
    pub attack_range: u16,
    #[length(24)]
    pub skill_name: String,
    pub upgraded: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x010F)]
#[variable_length]
pub struct UpdateSkillTreePacket {
    #[repeating_remaining]
    pub skill_information: Vec<SkillInformation>,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct HotkeyData {
    pub is_skill: u8,
    pub skill_id: u32,
    pub quantity_or_skill_level: SkillLevel,
}

impl HotkeyData {
    pub const UNBOUND: Self = Self {
        is_skill: 0,
        skill_id: 0,
        quantity_or_skill_level: SkillLevel(0),
    };
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B20)]
pub struct UpdateHotkeysPacket {
    pub rotate: u8,
    pub tab: HotbarTab,
    pub hotkeys: [HotkeyData; 38],
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02BA)]
pub struct SetHotkeyData1Packet {
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B21)]
pub struct SetHotkeyData2Packet {
    pub tab: HotbarTab,
    pub slot: HotbarSlot,
    pub hotkey_data: HotkeyData,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02C9)]
pub struct UpdatePartyInvitationStatePacket {
    pub allowed: u8, // always 0 on rAthena
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02DA)]
pub struct UpdateShowEquipPacket {
    pub open_equip_window: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02D9)]
pub struct UpdateConfigurationPacket {
    pub config_type: u32,
    pub value: u32, // only enabled and disabled ?
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x08E2)]
pub struct NavigateToMonsterPacket {
    pub target_type: u8, // 3 - entity; 0 - coordinates; 1 - coordinates but fails if you're alweady on the map
    pub flags: u8,
    pub hide_window: u8,
    #[length(16)]
    pub map_name: String,
    pub target_position: TilePosition,
    pub target_monster_id: u16,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u32)]
pub enum MarkerType {
    DisplayFor15Seconds,
    DisplayUntilLeave,
    RemoveMark,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0144)]
pub struct MarkMinimapPositionPacket {
    pub npc_id: EntityId,
    pub marker_type: MarkerType,
    pub position: LargeTilePosition,
    pub id: u8,
    pub color: ColorRGBA,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B5)]
pub struct NextButtonPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B6)]
pub struct CloseButtonPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B7)]
#[variable_length]
pub struct DialogMenuPacket {
    pub entity_id: EntityId,
    #[length_remaining]
    pub message: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x01F3)]
pub struct DisplaySpecialEffectPacket {
    pub entity_id: EntityId,
    pub effect_id: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x043D)]
pub struct DisplaySkillCooldownPacket {
    pub skill_id: SkillId,
    pub until: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x01DE)]
pub struct DisplaySkillEffectAndDamagePacket {
    pub skill_id: SkillId,
    pub source_entity_id: EntityId,
    pub destination_entity_id: EntityId,
    pub start_time: ClientTick,
    pub soruce_delay: u32,
    pub destination_delay: u32,
    pub damage: u32,
    pub level: SkillLevel,
    pub div: u16,
    pub skill_type: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum HealType {
    #[numeric_value(5)]
    Health,
    #[numeric_value(7)]
    SpellPoints,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0A27)]
pub struct DisplayPlayerHealEffect {
    pub heal_type: HealType,
    pub heal_amount: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09CB)]
pub struct DisplaySkillEffectNoDamagePacket {
    pub skill_id: SkillId,
    pub heal_amount: u32,
    pub destination_entity_id: EntityId,
    pub source_entity_id: EntityId,
    pub result: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0983)]
pub struct StatusChangePacket {
    pub index: u16,
    pub entity_id: EntityId,
    pub state: u8,
    pub duration_in_milliseconds: u32,
    pub remaining_in_milliseconds: u32,
    pub value: [u32; 3],
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ObjectiveDetails1 {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub mob_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09F9)]
pub struct QuestNotificationPacket1 {
    pub quest_id: u32,
    pub active: u8,
    pub start_time: u32,
    pub expire_time: u32,
    pub objective_count: u16,
    /// For some reason this packet always has space for three objective
    /// details, even if none are sent.
    pub objective_details: [ObjectiveDetails1; 3],
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct HuntingObjective {
    pub quest_id: u32,
    pub mob_id: u32,
    pub total_count: u16,
    pub current_count: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x08FE)]
#[variable_length]
pub struct HuntingQuestNotificationPacket {
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09FA)]
#[variable_length]
pub struct HuntingQuestUpdateObjectivePacket {
    pub objective_count: u16,
    #[repeating_remaining]
    pub objective_details: Vec<HuntingObjective>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02B4)]
pub struct QuestRemovedPacket {
    pub quest_id: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct QuestDetails {
    pub hunt_identification: u32,
    pub objective_type: u32,
    pub mob_id: u32,
    pub minimum_level: u16,
    pub maximum_level: u16,
    pub kill_count: u16,
    pub total_count: u16,
    #[length(24)]
    pub mob_name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Quest {
    pub quest_id: u32,
    pub active: u8,
    pub remaining_time: u32, // TODO: double check these
    pub expire_time: u32,    // TODO: double check these
    #[new_derive]
    pub objective_count: u16,
    #[repeating(objective_count)]
    pub objective_details: Vec<QuestDetails>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09F8)]
#[variable_length]
pub struct QuestListPacket {
    #[new_derive]
    pub quest_count: u32,
    #[repeating(quest_count)]
    pub quests: Vec<Quest>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u32)]
pub enum VisualEffect {
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

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x019B)]
pub struct VisualEffectPacket {
    pub entity_id: EntityId,
    pub effect: VisualEffect,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum ExperienceType {
    #[numeric_value(1)]
    BaseExperience,
    JobExperience,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum ExperienceSource {
    Regular,
    Quest,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0ACC)]
pub struct DisplayGainedExperiencePacket {
    pub account_id: AccountId,
    pub amount: u64,
    pub experience_type: ExperienceType,
    pub experience_source: ExperienceSource,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum ImageLocation {
    BottomLeft,
    BottomMiddle,
    BottomRight,
    MiddleFloating,
    MiddleColorless,
    #[numeric_value(255)]
    ClearAll,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x01B3)]
pub struct DisplayImagePacket {
    #[length(64)]
    pub image_name: String,
    pub location: ImageLocation,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0229)]
pub struct StateChangePacket {
    pub entity_id: EntityId,
    pub body_state: u16,
    pub health_state: u16,
    pub effect_state: u32,
    pub is_pk_mode_on: u8,
}

#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum ItemPickupResult {
    Success,
    Invalid,
    Overweight,
    Unknown0,
    NoSpace,
    MaximumOfItem,
    Unknown1,
    StackLimitation,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B41)]
pub struct ItemPickupPacket {
    pub index: InventoryIndex,
    pub count: u16,
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
    pub favorite: u8,
    pub look: u16,
    pub refinement_level: u8,
    pub enchantment_level: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum RemoveItemReason {
    Normal,
    ItemUsedForSkill,
    RefineFailed,
    MaterialChanged,
    MovedToStorage,
    MovedToCart,
    ItemSold,
    ConsumedByFourSpiritAnalysis,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x07FA)]
pub struct RemoveItemFromInventoryPacket {
    pub remove_reason: RemoveItemReason,
    pub index: InventoryIndex,
    pub amount: u16,
}

// TODO: improve names
#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
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

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum QuestColor {
    Yellow,
    Orange,
    Green,
    Purple,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0446)]
pub struct QuestEffectPacket {
    pub entity_id: EntityId,
    pub position: TilePosition,
    pub effect: QuestEffect,
    pub color: QuestColor,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B4)]
#[variable_length]
pub struct NpcDialogPacket {
    pub npc_id: EntityId,
    #[length_remaining]
    pub text: String,
}

#[derive(Debug, Clone, Default, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x007D)]
pub struct MapLoadedPacket {}

#[derive(Debug, Clone, Packet, ClientPacket, CharacterServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0187)]
#[ping]
pub struct CharacterServerKeepalivePacket {
    /// rAthena never reads this value, so just set it to 0.
    #[new_value(AccountId(0))]
    pub account_id: AccountId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0090)]
pub struct StartDialogPacket {
    pub npc_id: EntityId,
    #[new_value(1)]
    pub dialog_type: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B9)]
pub struct NextDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0146)]
pub struct CloseDialogPacket {
    pub npc_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B8)]
pub struct ChooseDialogOptionPacket {
    pub npc_id: EntityId,
    pub option: i8,
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    #[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
    pub struct EquipPosition: u32 {
        const NONE = 0;
        const HEAD_LOWER = 1;
        const HEAD_MIDDLE = 512;
        const HEAD_TOP = 256;
        const RIGHT_HAND = 2;
        const LEFT_HAND = 32;
        const ARMOR = 16;
        const SHOES = 64;
        const GARMENT = 4;
        const LEFT_ACCESSORY = 8;
        const RIGTH_ACCESSORY = 128;
        const COSTUME_HEAD_TOP = 1024;
        const COSTUME_HEAD_MIDDLE = 2048;
        const COSTUME_HEAD_LOWER = 4196;
        const COSTUME_GARMENT = 8192;
        const AMMO = 32768;
        const SHADOW_ARMOR = 65536;
        const SHADOW_WEAPON = 131072;
        const SHADOW_SHIELD = 262144;
        const SHADOW_SHOES = 524288;
        const SHADOW_RIGHT_ACCESSORY = 1048576;
        const SHADOW_LEFT_ACCESSORY = 2097152;
        const LEFT_RIGHT_ACCESSORY = 136;
        const LEFT_RIGHT_HAND = 34;
        const SHADOW_LEFT_RIGHT_ACCESSORY = 3145728;
    }
}

impl FixedByteSize for EquipPosition {
    fn size_in_bytes() -> usize {
        <<Self as bitflags::Flags>::Bits as FixedByteSize>::size_in_bytes()
    }
}

impl FromBytes for EquipPosition {
    fn from_bytes<Meta>(byte_stream: &mut ByteStream<Meta>) -> ConversionResult<Self> {
        <Self as bitflags::Flags>::Bits::from_bytes(byte_stream).map(|raw| Self::from_bits(raw).expect("Invalid equip position"))
    }
}

impl ToBytes for EquipPosition {
    fn to_bytes(&self) -> ConversionResult<Vec<u8>> {
        self.bits().to_bytes()
    }
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0998)]
pub struct RequestEquipItemPacket {
    pub inventory_index: InventoryIndex,
    pub equip_position: EquipPosition,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum RequestEquipItemStatus {
    Success,
    Failed,
    FailedDueToLevelRequirement,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0999)]
pub struct RequestEquipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub view_id: u16,
    pub result: RequestEquipItemStatus,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00AB)]
pub struct RequestUnequipItemPacket {
    pub inventory_index: InventoryIndex,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum RequestUnequipItemStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x099A)]
pub struct RequestUnequipItemStatusPacket {
    pub inventory_index: InventoryIndex,
    pub equipped_position: EquipPosition,
    pub result: RequestUnequipItemStatus,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum RestartType {
    Respawn,
    Disconnect,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B1)]
pub struct ParameterChangePacket {
    pub variable_id: u16,
    pub value: u32,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B2)]
pub struct RestartPacket {
    pub restart_type: RestartType,
}

// TODO: check that this can be only 1 and 0, if not ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum RestartResponseStatus {
    Nothing,
    Ok,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00B3)]
pub struct RestartResponsePacket {
    pub result: RestartResponseStatus,
}

// TODO: check that this can be only 1 and 0, if not Named, ByteConvertable
// should be implemented manually
#[derive(Debug, Clone, ByteConvertable, PartialEq, Eq)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum DisconnectResponseStatus {
    Ok,
    Wait10Seconds,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x018B)]
pub struct DisconnectResponsePacket {
    pub result: DisconnectResponseStatus,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0438)]
pub struct UseSkillAtIdPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0AF4)]
pub struct UseSkillOnGroundPacket {
    pub skill_level: SkillLevel,
    pub skill_id: SkillId,
    pub target_position: TilePosition,
    #[new_default]
    pub unused: u8,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B10)]
pub struct StartUseSkillPacket {
    pub skill_id: SkillId,
    pub skill_level: SkillLevel,
    pub target_id: EntityId,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B11)]
pub struct EndUseSkillPacket {
    pub skill_id: SkillId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x07FB)]
pub struct UseSkillSuccessPacket {
    pub source_entity: EntityId,
    pub destination_entity: EntityId,
    pub position: TilePosition,
    pub skill_id: SkillId,
    pub element: u32,
    pub delay_time: u32,
    pub disposable: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0110)]
pub struct ToUseSkillSuccessPacket {
    pub skill_id: SkillId,
    pub btype: i32,
    pub item_id: ItemId,
    pub flag: u8,
    pub cause: u8,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u32)]
pub enum UnitId {
    #[numeric_value(0x7E)]
    Safetywall,
    Firewall,
    WarpWaiting,
    WarpActive,
    Benedictio,
    Sanctuary,
    Magnus,
    Pneuma,
    Dummyskill,
    FirepillarWaiting,
    FirepillarActive,
    HiddenTrap,
    Trap,
    HiddenWarpNpc,
    UsedTraps,
    Icewall,
    Quagmire,
    Blastmine,
    Skidtrap,
    Anklesnare,
    Venomdust,
    Landmine,
    Shockwave,
    Sandman,
    Flasher,
    Freezingtrap,
    Claymoretrap,
    Talkiebox,
    Volcano,
    Deluge,
    Violentgale,
    Landprotector,
    Lullaby,
    Richmankim,
    Eternalchaos,
    Drumbattlefield,
    Ringnibelungen,
    Rokisweil,
    Intoabyss,
    Siegfried,
    Dissonance,
    Whistle,
    Assassincross,
    Poembragi,
    Appleidun,
    Uglydance,
    Humming,
    Dontforgetme,
    Fortunekiss,
    Serviceforyou,
    Graffiti,
    Demonstration,
    Callfamily,
    Gospel,
    Basilica,
    Moonlit,
    Fogwall,
    Spiderweb,
    Gravitation,
    Hermode,
    Kaensin,
    Suiton,
    Tatamigaeshi,
    Kaen,
    GrounddriftWind,
    GrounddriftDark,
    GrounddriftPoison,
    GrounddriftWater,
    GrounddriftFire,
    Deathwave,
    Waterattack,
    Windattack,
    Earthquake,
    Evilland,
    DarkRunner,
    DarkTransfer,
    Epiclesis,
    Earthstrain,
    Manhole,
    Dimensiondoor,
    Chaospanic,
    Maelstrom,
    Bloodylust,
    Feintbomb,
    Magentatrap,
    Cobalttrap,
    Maizetrap,
    Verduretrap,
    Firingtrap,
    Iceboundtrap,
    Electricshocker,
    Clusterbomb,
    Reverberation,
    SevereRainstorm,
    Firewalk,
    Electricwalk,
    Netherworld,
    PsychicWave,
    CloudKill,
    Poisonsmoke,
    Neutralbarrier,
    Stealthfield,
    Warmer,
    ThornsTrap,
    Wallofthorn,
    DemonicFire,
    FireExpansionSmokePowder,
    FireExpansionTearGas,
    HellsPlant,
    VacuumExtreme,
    Banding,
    FireMantle,
    WaterBarrier,
    Zephyr,
    PowerOfGaia,
    FireInsignia,
    WaterInsignia,
    WindInsignia,
    EarthInsignia,
    PoisonMist,
    LavaSlide,
    VolcanicAsh,
    ZenkaiWater,
    ZenkaiLand,
    ZenkaiFire,
    ZenkaiWind,
    Makibishi,
    Venomfog,
    Icemine,
    Flamecross,
    Hellburning,
    MagmaEruption,
    KingsGrace,
    GlitteringGreed,
    BTrap,
    FireRain,
    Catnippowder,
    Nyanggrass,
    Creatingstar,
    Dummy0,
    RainOfCrystal,
    MysteryIllusion,
    #[numeric_value(269)]
    StrantumTremor,
    ViolentQuake,
    AllBloom,
    TornadoStorm,
    FloralFlareRoad,
    AstralStrike,
    CrossRain,
    PneumaticusProcella,
    AbyssSquare,
    AcidifiedZoneWater,
    AcidifiedZoneGround,
    AcidifiedZoneWind,
    AcidifiedZoneFire,
    LightningLand,
    VenomSwamp,
    Conflagration,
    CaneOfEvilEye,
    TwinklingGalaxy,
    StarCannon,
    GrenadesDropping,
    #[numeric_value(290)]
    Fuumashouaku,
    MissionBombard,
    TotemOfTutelary,
    HyunRoksBreeze,
    Shinkirou, // mirage
    JackFrostNova,
    GroundGravitation,
    #[numeric_value(298)]
    Kunaiwaikyoku,
    #[numeric_value(20852)]
    Deepblindtrap,
    Solidtrap,
    Swifttrap,
    Flametrap,
    #[numeric_value(0xC1)]
    GdLeadership,
    #[numeric_value(0xC2)]
    GdGlorywounds,
    #[numeric_value(0xC3)]
    GdSoulcold,
    #[numeric_value(0xC4)]
    GdHawkeyes,
    #[numeric_value(0x190)]
    Max,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09CA)]
pub struct NotifySkillUnitPacket {
    pub lenght: u16,
    pub entity_id: EntityId,
    pub creator_id: EntityId,
    pub position: TilePosition,
    pub unit_id: UnitId,
    pub range: u8,
    pub visible: u8,
    pub skill_level: u8,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0117)]
pub struct NotifyGroundSkillPacket {
    pub skill_id: SkillId,
    pub entity_id: EntityId,
    pub level: SkillLevel,
    pub position: TilePosition,
    pub start_time: ClientTick,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0120)]
pub struct SkillUnitDisappearPacket {
    pub entity_id: EntityId,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Friend {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0202)]
pub struct AddFriendPacket {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0203)]
pub struct RemoveFriendPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x020A)]
pub struct NotifyFriendRemovedPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0201)]
#[variable_length]
pub struct FriendListPacket {
    #[repeating_remaining]
    pub friends: Vec<Friend>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum OnlineState {
    Online,
    Offline,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0206)]
pub struct FriendOnlineStatusPacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub state: OnlineState,
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0207)]
pub struct FriendRequestPacket {
    pub requestee: Friend,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u32)]
pub enum FriendRequestResponse {
    Reject,
    Accept,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0208)]
pub struct FriendRequestResponsePacket {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub response: FriendRequestResponse,
}

#[derive(Debug, Clone, PartialEq, Eq, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum FriendRequestResult {
    Accepted,
    Rejected,
    OwnFriendListFull,
    OtherFriendListFull,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0209)]
pub struct FriendRequestResultPacket {
    pub result: FriendRequestResult,
    pub friend: Friend,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x02C6)]
pub struct PartyInvitePacket {
    pub party_id: PartyId,
    #[length(24)]
    pub party_name: String,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ReputationEntry {
    pub reputation_type: u64,
    pub points: i64,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B8D)]
#[variable_length]
pub struct ReputationPacket {
    pub success: u8,
    #[repeating_remaining]
    pub entries: Vec<ReputationEntry>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Aliance {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Antagonist {
    #[length(24)]
    pub name: String,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x098A)]
#[variable_length]
pub struct ClanInfoPacket {
    pub clan_id: u32,
    #[length(24)]
    pub clan_name: String,
    #[length(24)]
    pub clan_master: String,
    #[length(16)]
    pub clan_map: String,
    #[new_derive]
    pub aliance_count: u8,
    #[new_derive]
    pub antagonist_count: u8,
    #[repeating(aliance_count)]
    pub aliances: Vec<Aliance>,
    #[repeating(antagonist_count)]
    pub antagonists: Vec<Antagonist>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0988)]
pub struct ClanOnlineCountPacket {
    pub online_members: u16,
    pub maximum_members: u16,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0192)]
pub struct ChangeMapCellPacket {
    pub position: TilePosition,
    pub cell_type: u16,
    #[length(16)]
    pub map_name: String,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct MarketItemInformation {
    pub name_id: u32,
    pub item_type: u8,
    pub price: Price,
    pub quantity: u32,
    pub weight: u16,
    pub location: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B7A)]
#[variable_length]
pub struct OpenMarketPacket {
    #[repeating_remaining]
    pub items: Vec<MarketItemInformation>,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ShopItemInformation {
    pub item_id: ItemId,
    pub price: Price,
    pub discount_price: Price,
    pub item_type: u8,
    pub view_sprite: u16,
    pub location: u32,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B77)]
#[variable_length]
pub struct ShopItemListPacket {
    #[repeating_remaining]
    pub items: Vec<ShopItemInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00C4)]
pub struct BuyOrSellPacket {
    pub shop_id: ShopId,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum BuyOrSellOption {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00C5)]
pub struct SelectBuyOrSellPacket {
    pub shop_id: ShopId,
    pub option: BuyOrSellOption,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum BuyItemResult {
    #[numeric_value(0)]
    Successful,
    #[numeric_value(1)]
    NotEoughZeny,
    #[numeric_value(2)]
    WeightLimitExceeded,
    #[numeric_value(3)]
    TooManyItems,
    #[numeric_value(9)]
    TooManyOfThisItem,
    #[numeric_value(10)]
    PropsOpenAir,
    #[numeric_value(11)]
    ExchangeFailed,
    #[numeric_value(12)]
    ExchangeWellDone,
    #[numeric_value(13)]
    ItemSoldOut,
    #[numeric_value(14)]
    NotEnoughGoods,
}

#[derive(Debug, Clone, ByteConvertable, FixedByteSize)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct BuyItemInformation {
    pub amount: u16,
    pub item_id: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00C8)]
#[variable_length]
pub struct BuyItemsPacket {
    #[repeating_remaining]
    pub items: Vec<BuyItemInformation>,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct BuyShopItemInformation {
    pub item_id: ItemId,
    pub amount: u32,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09D6)]
#[variable_length]
pub struct BuyShopItemsPacket {
    pub items: Vec<BuyShopItemInformation>,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[numeric_type(u16)]
pub enum BuyShopItemsResult {
    #[numeric_value(0)]
    Success,
    #[numeric_value(0xFFFF)]
    Error,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct BoughtShopItemInformation {
    pub item_id: ItemId,
    pub amount: u16,
    pub price: Price,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x0B4E)]
#[variable_length]
pub struct BuyShopItemsResultPacket {
    pub result: BuyShopItemsResult,
    #[repeating_remaining]
    pub purchased_items: Vec<BoughtShopItemInformation>,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x09D4)]
pub struct CloseShopPacket {}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SellItemInformation {
    pub inventory_index: InventoryIndex,
    pub price: Price,
    pub overcharge_price: Price,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00C7)]
#[variable_length]
pub struct SellListPacket {
    #[repeating_remaining]
    pub items: Vec<SellItemInformation>,
}

#[derive(Debug, Clone, FixedByteSize, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SoldItemInformation {
    pub inventory_index: InventoryIndex,
    pub amount: u16,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00C9)]
#[variable_length]
pub struct SellItemsPacket {
    #[repeating_remaining]
    pub items: Vec<SoldItemInformation>,
}

#[derive(Debug, Clone, Copy, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub enum SellItemsResult {
    Success,
    Error,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
#[header(0x00CB)]
pub struct SellItemsResultPacket {
    pub result: SellItemsResult,
}
