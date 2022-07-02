use derive_new::new;
use cgmath::Vector2;
use std::rc::Rc;
use std::cell::RefCell;
use std::net::{ IpAddr, Ipv4Addr, SocketAddr };
use std::time::Duration;
use std::fmt::Debug;
use std::net::TcpStream;
use std::io::prelude::*;

#[cfg(feature = "debug")]
use debug::*;
use crate::interface::windows::CharacterSelectionWindow;
use crate::traits::ByteConvertable;
use crate::types::ByteStream;

pub trait Packet {

    fn header() -> [u8; 2];

    fn to_bytes(&self) -> Vec<u8>;
}

/// An event triggered by the character server
#[derive(Clone, Debug)]
pub enum NetworkEvent {
    /// Add an entity to the list of entities that the client is aware of
    AddEntity(usize, usize, Vector2<usize>, usize),
    /// Remove an entity from the list of entities that the client is aware of by its id
    RemoveEntity(usize),
    /// The player is pathing to a new position
    PlayerMove(Vector2<usize>, Vector2<usize>, u32),
    /// An Entity nearby is pathing to a new position
    EntityMove(usize, Vector2<usize>, Vector2<usize>, u32),
    /// Player was moved to a new position on a different map or the current map
    ChangeMap(String, Vector2<usize>),
    /// Update the client side [tick counter](crate::system::GameTimer::client_tick) to keep server and client synchronized
    UpdataClientTick(u32),
}

#[derive(Copy, Clone, Debug, ByteConvertable)]
pub enum Sex {
    Male,
    Female,
    Both,
    Server,
}

#[derive(Debug, Packet, new)]
#[header(0x64, 0x00)]
struct LoginServerLoginPacket {
    #[new(default)]
    pub version: [u8; 4], // unused ?
    #[length_hint(24)]
    pub name: String, 
    #[length_hint(24)]
    pub password: String, 
    #[new(default)]
    pub client_type: u8, // also unused ?
}

#[allow(dead_code)]
#[derive(Debug, Packet)]
#[header(0xc4, 0x0a)]
struct LoginServerLoginSuccessPacket {
    pub packet_length: u16,
    pub login_id1: u32,
    pub account_id: u32,
    pub login_id2: u32,
    pub ip_address: u32, // deprecated and always 0
    pub name: [u8; 24], // deprecated and always 0
    pub unknown: u16, // always 0
    pub sex: Sex,
    pub auth_token: [u8; 17],
    #[repeating((self.packet_length - 64) / 160)]
    pub character_server_information: Vec<CharacterServerInformation>,
}

#[allow(dead_code)]
#[derive(Debug, Packet)]
#[header(0x2d, 0x08)]
struct CharacterServerLoginSuccessPacket {
    pub unknown: u16, // always 29 on rAthena
    pub normal_slot_count: u8,
    pub vip_slot_count: u8,
    pub billing_slot_count: u8,
    pub poducilble_slot_count: u8,
    pub vaild_slot: u8,
    pub unused: [u8; 20],
}

#[allow(dead_code)]
#[derive(Debug, Packet)]
#[header(0x6b, 0x00)]
struct Packet6b00 {
    pub unused: u16,
    pub maximum_slot_count: u8,
    pub avalible_slot_count: u8,
    pub vip_slot_count: u8,
    pub unknown: [u8; 20],
}

#[allow(dead_code)]
#[derive(Debug, Packet)]
#[header(0x18, 0x0b)]
struct Packet180b {
    pub unknown: u16, // possibly inventory related
}

#[derive(Debug, new)]
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

        Self { x: x as usize, y: y as usize }
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

#[derive(Debug, new)]
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

#[allow(dead_code)]
#[derive(Debug, Packet)]
#[header(0xeb, 0x02)]
struct MapServerLoginSuccessPacket {
    pub client_tick: u32,
    pub position: WorldPosition,
    pub ignored: [u8; 2], // always [5, 5] ?
    pub font: u16,
}

#[derive(Debug, ByteConvertable)]
pub enum LoginFailedReason {
    #[variant_value(1)]
    ServerClosed,
    #[variant_value(2)]
    AlreadyLoggedIn,
    #[variant_value(8)]
    AlreadyOnline,
}

#[derive(Debug, Packet)]
#[header(0x81, 0x00)]
struct LoginFailedPacket {
    pub reason: LoginFailedReason,
}

#[derive(Debug, ByteConvertable)]
pub enum CharacterSelectionFailedReason {
    RejectedFromServer,
}

#[derive(Debug, Packet)]
#[header(0x6c, 0x00)]
struct CharacterSelectionFailedPacket {
    pub reason: CharacterSelectionFailedReason,
}

#[derive(Debug, Packet)]
#[header(0xc5, 0x0a)]
struct CharacterSelectionSuccessPacket {
    pub character_id: u32,
    #[length_hint(16)]
    pub map_name: String,
    pub map_server_ip: Ipv4Addr,
    pub map_server_port: u16,
    pub unknown: [u8; 128],
}

#[derive(Debug, ByteConvertable)]
pub enum CharacterCreationFailedReason {
    CharacterNameAlreadyUsed,
    NotOldEnough,
    #[variant_value(3)]
    NotAllowedToUseSlot,
    #[variant_value(255)]
    CharacterCerationFailed,
}

#[derive(Debug, Packet)]
#[header(0x6e, 0x00)]
struct CharacterCreationFailedPacket {
    pub reason: CharacterCreationFailedReason,
}

#[derive(Debug, Default, Packet)]
#[header(0x00, 0x02)]
struct LoginServerKeepalivePacket {
    pub user_id: [u8; 24],
}

impl ByteConvertable for Ipv4Addr {

    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        assert!(length_hint.is_none());
        Ipv4Addr::new(byte_stream.next(), byte_stream.next(), byte_stream.next(), byte_stream.next())
    }
}

#[derive(Debug, ByteConvertable)]
struct CharacterServerInformation {
    pub server_ip: Ipv4Addr,
    pub server_port: u16,
    pub server_name: [u8; 20],
    pub user_count: u16,
    pub server_type: u16, // ServerType
    pub display_new: u16, // bool16 ?
    pub unknown: [u8; 128],
}

#[derive(Debug, Packet, new)]
#[header(0x65, 0x00)]
struct CharacterServerLoginPacket {
    pub account_id: u32,
    pub login_id1: u32,
    pub login_id2: u32,
    #[new(default)]
    pub unknown: u16,
    pub sex: Sex,
}

#[derive(Debug, Packet, new)]
#[header(0x36, 0x04)]
struct MapServerLoginPacket {
    pub account_id: u32,
    pub character_id: u32,
    pub login_id1: u32,
    pub client_tick: u32,
    pub sex: Sex,
    #[new(default)]
    pub unknown: [u8; 4],
}

#[derive(Debug, Packet)]
#[header(0x83, 0x02)]
struct Packet8302 {
    pub entity_id: u32,
}

#[derive(Debug, Packet, new)]
#[header(0x39, 0x0a)]
struct CreateCharacterPacket {
    #[length_hint(24)]
    pub name: String,
    pub slot: u8,
    pub hair_color: u16, // TODO: HairColor
    pub hair_style: u16, // TODO: HairStyle
    pub start_job: u16, // TODO: Job
    #[new(default)]
    pub unknown: [u8; 2],
    pub sex: Sex,
}

#[derive(Debug, ByteConvertable)]
pub struct CharacterInformation {
    pub character_id: u32,
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
    pub health: i64,
    pub maximum_health: i64,
    pub spell_points: i64,
    pub maximum_spell_points: i64,
    pub speed: i16,
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

#[derive(Debug, Packet)]
#[header(0x6f, 0x0b)]
struct CreateCharacterSuccessPacket {
    pub character_information: CharacterInformation,
}

#[derive(Debug, Default, Packet)]
#[header(0xa1, 0x09)]
struct RequestCharacterListPacket {}

#[derive(Debug, Packet)]
#[header(0x72, 0x0b)]
struct RequestCharacterListSuccessPacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 175)]
    pub character_information: Vec<CharacterInformation>,
}

#[derive(Debug, Packet, new)]
#[header(0x81, 0x08)]
struct RequestPlayerMovePacket {
    pub position: WorldPosition,
}

#[derive(Debug, Packet)]
#[header(0x86, 0x00)]
struct EntityMovePacket {
    pub entity_id: u32,
    pub from_to: WorldPosition2,
    pub timestamp: u32,
}

#[derive(Debug, Packet)]
#[header(0x87, 0x00)]
struct PlayerMovePacket {
    pub timestamp: u32,
    pub from_to: WorldPosition2,
}

#[derive(Debug, Packet, new)]
#[header(0xfb, 0x01)]
struct DeleteCharacterPacket {
    character_id: u32,
    #[length_hint(40)]
    pub email: String,
    #[new(default)]
    pub unknown: [u8; 10],
}

#[derive(Debug, ByteConvertable)]
pub enum CharacterDeletionFailedReason {
    NotAllowed,
    CharacterNotFound,
    NotEligible,
}

#[derive(Debug, Packet)]
#[header(0x70, 0x00)]
struct CharacterDeletionFailedPacket {
    pub reason: CharacterDeletionFailedReason,
}

#[derive(Debug, Packet)]
#[header(0x6f, 0x00)]
struct CharacterDeletionSuccessPacket {}

#[derive(Debug, Packet, new)]
#[header(0x66, 0x00)]
struct SelectCharacterPacket {
    pub selected_slot: u8,
}

#[derive(Debug, Packet)]
#[header(0x8e, 0x00)]
struct ServerMessagePacket {
    pub packet_length: u16,
    #[length_hint(self.packet_length - 4)]
    pub message: String,
}

#[derive(Debug, Packet)]
#[header(0xe7, 0x09)]
struct NewMailStatusPacket {
    pub new_avalible: u8,
}

#[derive(Debug, ByteConvertable)]
struct AchievementData {
    pub acheivement_id: u32,
    pub is_completed: u8,
    pub objectives: [u32; 10],
    pub completion_timestamp: u32,
    pub got_rewarded: u8,
}

#[derive(Debug, Packet)]
#[header(0x24, 0x0a)]
struct AchievementUpdatePacket {
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_tnl: u32, // ?
    pub acheivement_data: AchievementData,
}

#[derive(Debug, Packet)]
#[header(0x23, 0x0a)]
struct AchievementListPacket {
    pub packet_length: u16,
    pub acheivement_count: u32,
    pub total_score: u32,
    pub level: u16,
    pub acheivement_experience: u32,
    pub acheivement_experience_tnl: u32, // ?
    #[repeating(self.acheivement_count)]
    pub acheivement_data: Vec<AchievementData>,
}

#[derive(Debug, Packet)]
#[header(0xde, 0x0a)]
struct CriticalWeightUpdatePacket {
    pub packet_length: u32,
}

#[derive(Debug, Packet)]
#[header(0xd7, 0x01)]
struct SpriteChangePacket {
    pub entity_id: u32,
    pub sprite_type: u8, // is it actually sprite_ ?
    pub value: u32,
    pub value2: u32,
}

#[derive(Debug, Packet)]
#[header(0x08, 0x0b)]
struct InventoyStartPacket {
    pub packet_length: u16,
    pub inventory_type: u8,
    #[length_hint(self.packet_length - 5)]
    pub inventory_name: String,
}

#[derive(Debug, Packet)]
#[header(0x0b, 0x0b)]
struct InventoyEndPacket {
    pub inventory_type: u8,
    pub flag: u8, // maybe char ?
}

#[derive(Copy, Clone, Debug, Default, ByteConvertable)]
struct ItemOptions {
    pub index: u16,
    pub value: u16,
    pub parameter: u8,
}

#[derive(Debug, ByteConvertable)]
struct EquippableItemInformation {
    pub index: u16,
    pub item_id: u32,
    pub item_type: u8,
    pub location: u32, // 11
    pub wear_state: u32,
    pub slot: [u32; 4], // card ?
    pub hire_expiration_date: i32, // 35
    pub bind_on_equip_type: u16,
    pub w_item_sprite_number: u16,
    pub option_count: u8, // 40
    pub option_data: [ItemOptions; 5], // fix count
    pub refinement_level: u8,
    pub enchantment_level: u8,
    pub fags: u8, // bit 1 - is_identified; bit 2 - is_damaged; bit 3 - place_in_etc_tab
}

#[derive(Debug, Packet)]
#[header(0x39, 0x0b)]
struct EquippableItemListPacket {
    pub packet_length: u16,
    pub inventory_type: u8,
    #[repeating((self.packet_length - 5) / 68)]
    pub item_information: Vec<EquippableItemInformation>,
}

#[derive(Debug, ByteConvertable)]
struct EquippableSwitchItemInformation {
    pub index: u16, // is actually index + 2
    pub position: u32,
}

#[derive(Debug, Packet)]
#[header(0x9b, 0x0a)]
struct EquippableSwitchItemListPacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 6)]
    pub item_information: Vec<EquippableSwitchItemInformation>,
}

#[derive(Debug, Packet)]
#[header(0x9b, 0x09)]
struct MapTypePacket {
    pub map_type: u16,
    pub flags: u32,
}

#[derive(Debug, Packet)]
#[header(0xc3, 0x01)]
struct BroadcastMessagePacket {
    pub packet_length: u16,
    pub font_color: u32,
    pub font_type: u16,
    pub font_size: u16,
    pub font_alignment: u16,
    pub font_y: u16,
    #[length_hint(self.packet_length - 16)]
    pub message: String,
}

#[derive(Debug, Packet)]
#[header(0xc1, 0x02)]
struct EntityMessagePacket {
    pub packet_length: u16,
    pub entity_id: u32,
    pub color: u32,
    #[length_hint(self.packet_length - 12)]
    pub message: String,
}

#[derive(Debug, Packet)]
#[header(0xc0, 0x00)]
struct DisplayEmotionPacket {
    pub entity_id: u32,
    pub emotion: u8,
}

#[derive(Debug)]
enum StatusType {
    SP_WEIGHT(u32),
    SP_MAXWEIGHT(u32),
    SP_SPEED(u32),
    SP_BASELEVEL(u32),
    SP_JOBLEVEL(u32),
    SP_KARMA(u32),
    SP_MANNER(u32),
    SP_STATUSPOINT(u32),
    SP_SKILLPOINT(u32),
    SP_HIT(u32),
    SP_FLEE1(u32),
    SP_FLEE2(u32),
    SP_MAXHP(u32),
    SP_MAXSP(u32),
    SP_HP(u32),
    SP_SP(u32),
    SP_ASPD(u32),
    SP_ATK1(u32),
    SP_DEF1(u32),
    SP_MDEF1(u32),
    SP_ATK2(u32),
    SP_DEF2(u32),
    SP_MDEF2(u32),
    SP_CRITICAL(u32),
    SP_MATK1(u32),
    SP_MATK2(u32),
    SP_ZENY(u32),
    SP_BASEEXP(u64),
    SP_JOBEXP(u64),
    SP_NEXTBASEEXP(u64),
    SP_NEXTJOBEXP(u64),
    SP_USTR(u8),
    SP_UAGI(u8),
    SP_UVIT(u8),
    SP_UINT(u8),
    SP_UDEX(u8),
    SP_ULUK(u8),
    SP_STR(u32, u32),
    SP_AGI(u32, u32),
    SP_VIT(u32, u32),
    SP_INT(u32, u32),
    SP_DEX(u32, u32),
    SP_LUK(u32, u32),
    SP_CARTINFO(u16, u32, u32),
    SP_AP(u32),
    SP_TRAITPOINT(u32),
    SP_MAXAP(u32),
    SP_POW(u32, u32),
    SP_STA(u32, u32),
    SP_WIS(u32, u32),
    SP_SPL(u32, u32),
    SP_CON(u32, u32),
    SP_CRT(u32, u32),
    SP_UPOW(u8),
    SP_USTA(u8),
    SP_UWIS(u8),
    SP_USPL(u8),
    SP_UCON(u8),
    SP_UCRT(u8),
    SP_PATK(u32),
    SP_SMATK(u32),
    SP_RES(u32),
    SP_MRES(u32),
    SP_HPLUS(u32),
    SP_CRATE(u32),
}

impl ByteConvertable for StatusType {
    
    fn from_bytes(byte_stream: &mut ByteStream, length_hint: Option<usize>) -> Self {
        let data = byte_stream.slice(length_hint.unwrap());
        let mut byte_stream = ByteStream::new(&data);

        match u16::from_bytes(&mut byte_stream, None) {
            0 => Self::SP_SPEED(u32::from_bytes(&mut byte_stream, None)),
            1 => Self::SP_BASEEXP(u64::from_bytes(&mut byte_stream, None)),
            2 => Self::SP_JOBEXP(u64::from_bytes(&mut byte_stream, None)),
            3 => Self::SP_KARMA(u32::from_bytes(&mut byte_stream, None)),
            4 => Self::SP_MANNER(u32::from_bytes(&mut byte_stream, None)),
            5 => Self::SP_HP(u32::from_bytes(&mut byte_stream, None)),
            6 => Self::SP_MAXHP(u32::from_bytes(&mut byte_stream, None)),
            7 => Self::SP_SP(u32::from_bytes(&mut byte_stream, None)),
            8 => Self::SP_MAXSP(u32::from_bytes(&mut byte_stream, None)),
            9 => Self::SP_STATUSPOINT(u32::from_bytes(&mut byte_stream, None)),
            11 => Self::SP_BASELEVEL(u32::from_bytes(&mut byte_stream, None)),
            12 => Self::SP_SKILLPOINT(u32::from_bytes(&mut byte_stream, None)),
            13 => Self::SP_STR(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            14 => Self::SP_AGI(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            15 => Self::SP_VIT(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            16 => Self::SP_INT(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            17 => Self::SP_DEX(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            18 => Self::SP_LUK(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            20 => Self::SP_ZENY(u32::from_bytes(&mut byte_stream, None)),
            22 => Self::SP_NEXTBASEEXP(u64::from_bytes(&mut byte_stream, None)),
            23 => Self::SP_NEXTJOBEXP(u64::from_bytes(&mut byte_stream, None)),
            24 => Self::SP_WEIGHT(u32::from_bytes(&mut byte_stream, None)),
            25 => Self::SP_MAXWEIGHT(u32::from_bytes(&mut byte_stream, None)),
            32 => Self::SP_USTR(u8::from_bytes(&mut byte_stream, None)),
            33 => Self::SP_UAGI(u8::from_bytes(&mut byte_stream, None)),
            34 => Self::SP_UVIT(u8::from_bytes(&mut byte_stream, None)),
            35 => Self::SP_UINT(u8::from_bytes(&mut byte_stream, None)),
            36 => Self::SP_UDEX(u8::from_bytes(&mut byte_stream, None)),
            37 => Self::SP_ULUK(u8::from_bytes(&mut byte_stream, None)),
            41 => Self::SP_ATK1(u32::from_bytes(&mut byte_stream, None)),
            42 => Self::SP_ATK2(u32::from_bytes(&mut byte_stream, None)),
            43 => Self::SP_MATK1(u32::from_bytes(&mut byte_stream, None)),
            44 => Self::SP_MATK2(u32::from_bytes(&mut byte_stream, None)),
            45 => Self::SP_DEF1(u32::from_bytes(&mut byte_stream, None)),
            46 => Self::SP_DEF2(u32::from_bytes(&mut byte_stream, None)),
            47 => Self::SP_MDEF1(u32::from_bytes(&mut byte_stream, None)),
            48 => Self::SP_MDEF2(u32::from_bytes(&mut byte_stream, None)),
            49 => Self::SP_HIT(u32::from_bytes(&mut byte_stream, None)),
            50 => Self::SP_FLEE1(u32::from_bytes(&mut byte_stream, None)),
            51 => Self::SP_FLEE2(u32::from_bytes(&mut byte_stream, None)),
            52 => Self::SP_CRITICAL(u32::from_bytes(&mut byte_stream, None)),
            53 => Self::SP_ASPD(u32::from_bytes(&mut byte_stream, None)),
            55 => Self::SP_JOBLEVEL(u32::from_bytes(&mut byte_stream, None)),
            99 => Self::SP_CARTINFO(u16::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
	    219 => Self::SP_POW(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)), 
            220 => Self::SP_STA(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            221 => Self::SP_WIS(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            222 => Self::SP_SPL(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            223 => Self::SP_CON(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            224 => Self::SP_CRT(u32::from_bytes(&mut byte_stream, None), u32::from_bytes(&mut byte_stream, None)),
            225 => Self::SP_PATK(u32::from_bytes(&mut byte_stream, None)),
            226 => Self::SP_SMATK(u32::from_bytes(&mut byte_stream, None)),
            227 => Self::SP_RES(u32::from_bytes(&mut byte_stream, None)),
            228 => Self::SP_MRES(u32::from_bytes(&mut byte_stream, None)),
            229 => Self::SP_HPLUS(u32::from_bytes(&mut byte_stream, None)),
            230 => Self::SP_CRATE(u32::from_bytes(&mut byte_stream, None)),
            231 => Self::SP_TRAITPOINT(u32::from_bytes(&mut byte_stream, None)),
            232 => Self::SP_AP(u32::from_bytes(&mut byte_stream, None)),
            233 => Self::SP_MAXAP(u32::from_bytes(&mut byte_stream, None)),
            247 => Self::SP_UPOW(u8::from_bytes(&mut byte_stream, None)),
            248 => Self::SP_USTA(u8::from_bytes(&mut byte_stream, None)),
            249 => Self::SP_UWIS(u8::from_bytes(&mut byte_stream, None)),
            250 => Self::SP_USPL(u8::from_bytes(&mut byte_stream, None)),
            251 => Self::SP_UCON(u8::from_bytes(&mut byte_stream, None)),
            252 => Self::SP_UCRT(u8::from_bytes(&mut byte_stream, None)),
            invalid => panic!("invalid status code {}", invalid),
        }
    }
}

#[derive(Debug, Packet)]
#[header(0xb0, 0x00)]
struct UpdateStatusPacket {
    #[length_hint(6)]
    pub status_type: StatusType,
}

#[derive(Debug, Packet)]
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
    pub bonus_attack_speed: u16, // always 0
}

#[derive(Debug, Packet)]
#[header(0x41, 0x01)]
struct UpdateStatusPacket1 {
    #[length_hint(12)]
    pub status_type: StatusType,
}

#[derive(Debug, Packet)]
#[header(0xcb, 0x0a)]
struct UpdateStatusPacket2 {
    #[length_hint(10)]
    pub status_type: StatusType,
}

#[derive(Debug, Packet)]
#[header(0xbe, 0x00)]
struct UpdateStatusPacket3 {
    #[length_hint(3)]
    pub status_type: StatusType,
}

#[derive(Debug, Packet)]
#[header(0x3a, 0x01)]
struct UpdateAttackRangePacket {
    pub attack_range: u16,
}

#[derive(Debug, Packet, new)]
#[header(0xd4, 0x08)]
struct SwitchCharacterSlotPacket {
    pub origin_slot: u16,
    pub destination_slot: u16,
    #[new(value = "1")]
    pub remaining_moves: u16, // 1 instead of default, just in case the sever actually uses this value (rAthena does not)
}

#[derive(Debug, Packet)]
#[header(0x7f, 0x00)]
struct ServerTickPacket {
    pub client_tick: u32,
}

#[derive(Debug, Packet, new)]
#[header(0x60, 0x03)]
struct RequestServerTickPacket {
    pub client_tick: u32,
}

#[derive(Debug, PartialEq, Eq, ByteConvertable)]
#[base_type(u16)]
pub enum SwitchCharacterSlotResponseStatus {
    Success,
    Error,
}

#[derive(Debug, Packet)]
#[header(0x70, 0x0b)]
struct SwitchCharacterSlotResponsePacket {
    pub unknown: u16, // is always 8 ?
    pub status: SwitchCharacterSlotResponseStatus,
    pub remaining_moves: u16,
}

#[derive(Debug, Packet)]
#[header(0x91, 0x00)]
struct ChangeMapPacket {
    #[length_hint(16)]
    pub map_name: String,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, ByteConvertable)]
enum DissapearanceReason {
    OutOfSight,
    Died,
    LoggedOut,
    Teleported,
    TrickDead,
}

#[derive(Debug, Packet)]
#[header(0x80, 0x00)]
struct EntityDisappearedPacket {
    pub entity_id: u32,
    pub reason: DissapearanceReason,
}

#[derive(Debug, Packet)]
#[header(0xfd, 0x09)]
struct MovingEntityAppearedPacket {
    pub packet_length: u16,
    pub object_type: u8,
    pub entity_id: u32,
    pub group_id: u32, // may be reversed - or completely wrong
    pub speed: u16,
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
    pub head_dir: u16,
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

#[derive(Debug, Packet)]
#[header(0xff, 0x09)]
struct EntityAppearedPacket {
    pub packet_length: u16,
    pub object_type: u8,
    pub entity_id: u32,
    pub group_id: u32, // may be reversed - or completely wrong
    pub speed: u16,
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
    pub head_dir: u16,
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

#[derive(Debug, ByteConvertable)]
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

#[derive(Debug, Packet)]
#[header(0x0f, 0x01)]
struct UpdateSkillTreePacket {
    pub packet_length: u16,
    #[repeating((self.packet_length - 4) / 37)]
    pub skill_information: Vec<SkillInformation>,
}

#[derive(Copy, Clone, Debug, Default, ByteConvertable)]
struct HotkeyData {
    pub is_skill: u8,
    pub skill_id: u32,
    pub quantity_or_skill_level: u16,
}

#[derive(Debug, Packet)]
#[header(0x20, 0x0b)]
struct UpdateHotkeysPacket {
    pub rotate: u8,
    pub tab: u16,
    pub hotkeys: [HotkeyData; 38],
}

#[derive(Debug, Packet)]
#[header(0xc9, 0x02)]
struct UpdatePartyInvitationStatePacket {
    pub allowed: u8, // always 0 on rAthena
}

#[derive(Debug, Packet)]
#[header(0xda, 0x02)]
struct UpdateShowEquipPacket {
    pub open_equip_window: u8,
}

#[derive(Debug, Packet)]
#[header(0xd9, 0x02)]
struct UpdateConfigurationPacket {
    pub config_type: u32,
    pub value: u32, // only enabled and disabled ?
}

#[derive(Debug, Packet)]
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

#[derive(Debug, Packet)]
#[header(0xb6, 0x00)]
struct CloseScriptPacket {
    pub entity_id: u32,
}

#[derive(Debug, Packet)]
#[header(0x46, 0x04)]
struct QuestNotificatonPacket {
    pub entity_id: u32,
    pub position_x: u16,
    pub position_y: u16,
    pub effect: u16, // 0 - none; 1 - exclamation mark; 2 - question mark 
    pub color: u16, // 0 - yellow; 1 - orange; 2 - green; 3 - purple
}

#[derive(Debug, Default, Packet)]
#[header(0x7d, 0x00)]
struct MapLoadedPacket {}

#[derive(Debug, Default, Packet)]
#[header(0x87, 0x01)]
struct CharacterServerKeepalivePacket {
    pub account_id: u32,
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
    pub account_id: u32,
    pub login_id1: u32,
    pub sex: Sex,
}

pub struct NetworkingSystem {
    login_stream: TcpStream,
    character_stream: Option<TcpStream>,
    map_stream: Option<TcpStream>,
    login_data: Option<LoginData>,
    characters: Rc<RefCell<Vec<CharacterInformation>>>,
    move_request: Rc<RefCell<Option<usize>>>,
    changed: Rc<RefCell<bool>>,
    login_keep_alive_timer: NetworkTimer,
    character_keep_alive_timer: NetworkTimer,
    map_keep_alive_timer: NetworkTimer,
}

impl NetworkingSystem {

    pub fn new() -> Self {

        //let login_stream = TcpStream::connect("127.0.0.1:6900").expect("failed to connect to login server");
        let login_stream = TcpStream::connect("167.235.227.244:6900").expect("failed to connect to login server");
        
        let character_stream = None;
        let map_stream = None;
        let login_data = None;
        let characters = Rc::new(RefCell::new(Vec::new()));
        let move_request = Rc::new(RefCell::new(None));
        let changed = Rc::new(RefCell::new(false));
        let login_keep_alive_timer = NetworkTimer::new(Duration::from_secs(58));
        let character_keep_alive_timer = NetworkTimer::new(Duration::from_secs(10));
        let map_keep_alive_timer = NetworkTimer::new(Duration::from_secs(4));

        login_stream.set_read_timeout(Duration::from_secs(20).into()).unwrap();

        Self {
            login_stream,
            character_stream,
            move_request,
            login_data,
            changed,
            map_stream,
            characters,
            login_keep_alive_timer,
            character_keep_alive_timer,
            map_keep_alive_timer,
        }
    }

    pub fn login(&mut self) -> Result<CharacterSelectionWindow, String> {

        #[cfg(feature = "debug_network")]
        let timer = Timer::new("login");

        self.send_packet_to_login_server(LoginServerLoginPacket::new("test_user".to_string(), "password".to_string()));

        let response = self.get_data_from_login_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(login_failed_packet) = LoginFailedPacket::try_from_bytes(&mut byte_stream) {
            match login_failed_packet.reason {
                LoginFailedReason::ServerClosed => return Err("server closed".to_string()),
                LoginFailedReason::AlreadyLoggedIn => return Err("someone has already logged in with this id".to_string()),
                LoginFailedReason::AlreadyOnline => return Err("already online".to_string()),
            }
        }

        let login_server_login_success_packet = LoginServerLoginSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();
        self.login_data = LoginData::new(login_server_login_success_packet.account_id, login_server_login_success_packet.login_id1, login_server_login_success_packet.sex).into();

        let character_server_information = login_server_login_success_packet.character_server_information
            .into_iter()
            .next()
            .expect("no character server available");

        let server_ip = IpAddr::V4(character_server_information.server_ip);
        let socket_address = SocketAddr::new(server_ip, character_server_information.server_port);
        self.character_stream = TcpStream::connect(socket_address)
            .expect("failed to connect to character server")
            .into();

        let character_server_login_packet = CharacterServerLoginPacket::new(
            login_server_login_success_packet.account_id,
            login_server_login_success_packet.login_id1,
            login_server_login_success_packet.login_id2,
            login_server_login_success_packet.sex,
        );

        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        character_stream.write(&character_server_login_packet.to_bytes()).expect("failed to send packet to character server");

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        let account_id = u32::from_bytes(&mut byte_stream, None);
        assert!(account_id == login_server_login_success_packet.account_id);

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

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        let request_character_list_success_packet = RequestCharacterListSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();
        *self.characters.borrow_mut() = request_character_list_success_packet.character_information;

        #[cfg(feature = "debug_network")]
        timer.stop();

        Ok(CharacterSelectionWindow::new(Rc::clone(&self.characters), Rc::clone(&self.move_request), Rc::clone(&self.changed), character_server_login_success_packet.normal_slot_count as usize))
    }

    fn send_packet_to_login_server(&mut self, packet: impl Packet + Debug) {

        #[cfg(feature = "debug_network")]
        print_debug!("{}outgoing packet{}: {:?}", RED, NONE, packet);

        let packet_bytes = packet.to_bytes();
        self.login_stream.write(&packet_bytes).expect("failed to send packet to login server");
    }

    fn send_packet_to_character_server(&mut self, packet: impl Packet + Debug) {

        #[cfg(feature = "debug_network")]
        print_debug!("{}outgoing packet{}: {:?}", RED, NONE, packet);

        let packet_bytes = packet.to_bytes();
        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        character_stream.write(&packet_bytes).expect("failed to send packet to character server");
    }

    fn send_packet_to_map_server(&mut self, packet: impl Packet + Debug) {

        #[cfg(feature = "debug_network")]
        print_debug!("{}outgoing packet{}: {:?}", RED, NONE, packet);

        let packet_bytes = packet.to_bytes();
        let map_stream = self.map_stream.as_mut().expect("no map server connection");
        map_stream.write(&packet_bytes).expect("failed to send packet to map server");
    }

    fn get_data_from_login_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let response_lenght = self.login_stream.read(&mut buffer).expect("failed to get response from login server");
        buffer[..response_lenght].to_vec()
    }

    fn get_data_from_character_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        let response_lenght = character_stream.read(&mut buffer).expect("failed to get response from character server");
        buffer[..response_lenght].to_vec()
    }

    fn get_data_from_map_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let map_stream = self.map_stream.as_mut().expect("no map server connection");
        let response_lenght = map_stream.read(&mut buffer).expect("failed to get response from map server");
        buffer[..response_lenght].to_vec()
    }

    fn try_get_data_from_map_server(&mut self) -> Option<Vec<u8>> {
        let mut buffer = [0; 4096];
        let map_stream = self.map_stream.as_mut()?;
        map_stream.set_read_timeout(Duration::from_micros(1).into()).unwrap();
        let response_lenght = map_stream.read(&mut buffer).ok()?;
        buffer[..response_lenght].to_vec().into()
    }

    pub fn keep_alive(&mut self, delta_time: f64, client_tick: u32) {

        if self.login_keep_alive_timer.update(delta_time) {
            self.send_packet_to_login_server(LoginServerKeepalivePacket::default());
        }

        if self.character_keep_alive_timer.update(delta_time) && self.character_stream.is_some() {
            self.send_packet_to_character_server(CharacterServerKeepalivePacket::default());
        }

        if self.map_keep_alive_timer.update(delta_time) && self.map_stream.is_some() {
            self.send_packet_to_map_server(RequestServerTickPacket::new(client_tick));
        }
    }

    pub fn crate_character(&mut self, slot: usize, /* TODO: */) -> Result<(), String> {

        let name = [
            "lucas",
            "warlock dude",
            "t3st CH4R",
            "Seemon",
            "Pretty Long Name",
            "xXdarkshadowXx",
            "nvidia fanboy",
            "AMD Enjoyer",
            "Slutty eGirl",
            "NULL",
            "bitwise or",
            "im out of names",
            "someone help me",
            "seriously",
            "Ron Howard",
        ][slot].to_string();

        let hair_color = 0;
        let hair_style = 0;
        let start_job = 0;
        let sex = Sex::Male;

        self.send_packet_to_character_server(CreateCharacterPacket::new(name, slot as u8, hair_color, hair_style, start_job, sex));

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        if let Ok(character_creation_failed_packet) = CharacterCreationFailedPacket::try_from_bytes(&mut byte_stream) {
            match character_creation_failed_packet.reason {
                CharacterCreationFailedReason::CharacterNameAlreadyUsed => return Err("character name is already used".to_string()),
                CharacterCreationFailedReason::NotOldEnough => return Err("you are not old enough to create a character".to_string()),
                CharacterCreationFailedReason::NotAllowedToUseSlot => return Err("you are not allowed to use that character slot".to_string()),
                CharacterCreationFailedReason::CharacterCerationFailed => return Err("character creation failed".to_string()),
            }
        }

        let create_character_success_packet = CreateCharacterSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        self.characters.borrow_mut().push(create_character_success_packet.character_information);
        *self.changed.borrow_mut() = true;
        Ok(())
    }

    pub fn delete_character(&mut self, character_id: usize) -> Result<(), String> {

        let email = "a@a.com".to_string();

        self.send_packet_to_character_server(DeleteCharacterPacket::new(character_id as u32, email));

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

        self.characters.borrow_mut().retain(|character| character.character_id as usize != character_id);
        *self.changed.borrow_mut() = true;
        Ok(())
    }

    pub fn select_character(&mut self, slot: usize) -> Result<(String, Vector2<usize>, usize, usize, usize, u32), String> {

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
                LoginFailedReason::ServerClosed => return Err("server closed".to_string()),
                LoginFailedReason::AlreadyLoggedIn => return Err("someone has already logged in with this id".to_string()),
                LoginFailedReason::AlreadyOnline => return Err("already online".to_string()),
            }
        }

        let select_character_success_packet = CharacterSelectionSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        let server_ip = IpAddr::V4(select_character_success_packet.map_server_ip);
        let socket_address = SocketAddr::new(server_ip, select_character_success_packet.map_server_port);
        self.map_stream = TcpStream::connect(socket_address)
            .expect("failed to connect to map server")
            .into();

        let login_data = self.login_data.as_ref().unwrap();
        self.send_packet_to_map_server(MapServerLoginPacket::new(login_data.account_id, select_character_success_packet.character_id, login_data.login_id1, 100, login_data.sex));

        let response = self.get_data_from_map_server();
        let mut byte_stream = ByteStream::new(&response);

        let _packet8302 = Packet8302::try_from_bytes(&mut byte_stream).unwrap();

        let response = self.get_data_from_map_server();
        let mut byte_stream = ByteStream::new(&response);

        let _packet_180b = Packet180b::try_from_bytes(&mut byte_stream).unwrap();
        let map_server_login_success_packet = MapServerLoginSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();

        while let Ok(server_message_packet) = ServerMessagePacket::try_from_bytes(&mut byte_stream) {
            println!("message from server: {}", server_message_packet.message);
        }

        let change_map_packet = ChangeMapPacket::try_from_bytes(&mut byte_stream).unwrap();

        Ok((
            change_map_packet.map_name.replace(".gat", ""),
            Vector2::new(change_map_packet.x as usize, change_map_packet.y as usize),
            select_character_success_packet.character_id as usize,
            45, // how do we get this ?
            200, // set 200 to 0 as soon as stats are properly updated
            map_server_login_success_packet.client_tick
        ))
    }

    pub fn request_switch_character_slot(&mut self, origin_slot: usize) {
        *self.move_request.borrow_mut() = Some(origin_slot);
        *self.changed.borrow_mut() = true;
    }

    pub fn cancel_switch_character_slot(&mut self) {
        *self.move_request.borrow_mut() = None;
        *self.changed.borrow_mut() = true;
    }

    pub fn switch_character_slot(&mut self, destination_slot: usize) -> Result<(), String> {

        let origin_slot = self.move_request
            .borrow_mut()
            .take()
            .unwrap();

        self.send_packet_to_character_server(SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16));

        let response = self.get_data_from_character_server();
        let mut byte_stream = ByteStream::new(&response);

        let switch_character_slot_response_packet = SwitchCharacterSlotResponsePacket::try_from_bytes(&mut byte_stream).unwrap();

        match switch_character_slot_response_packet.status {

            SwitchCharacterSlotResponseStatus::Success => {

                let _character_server_login_success_packet = CharacterServerLoginSuccessPacket::try_from_bytes(&mut byte_stream).unwrap();
                let _packet_006b = Packet6b00::try_from_bytes(&mut byte_stream).unwrap();

                let mut characters = self.characters.borrow_mut();
                let character_count = characters.len();
                characters.clear();

                for _index in 0..character_count {
                    let character_information = CharacterInformation::from_bytes(&mut byte_stream, None);

                    #[cfg(feature = "debug_network")]
                    print_debug!("{}incoming packet{}: {:?}", YELLOW, NONE, character_information);

                    characters.push(character_information);
                }

                // packet_length and packet 0xa0 0x09 are left unread because we don't need them 
            },

            SwitchCharacterSlotResponseStatus::Error => return Err("failed to move character to a different slot".to_string()),
        }

        *self.move_request.borrow_mut() = None;
        *self.changed.borrow_mut() = true;
        Ok(())
    }

    pub fn request_player_move(&mut self, destination: Vector2<usize>) {
        self.send_packet_to_map_server(RequestPlayerMovePacket::new(WorldPosition::new(destination.x, destination.y)));
    }

    pub fn map_loaded(&mut self) {
        self.send_packet_to_map_server(MapLoadedPacket::default());
    }

    pub fn changes_applied(&mut self) {
        *self.changed.borrow_mut() = false;
    }

    pub fn network_events(&mut self) -> Vec<NetworkEvent> {
        let mut events = Vec::new();

        while let Some(data) = self.try_get_data_from_map_server() {
            let mut byte_stream = ByteStream::new(&data);

            while !byte_stream.is_empty() {
                
                if let Ok(packet) = BroadcastMessagePacket::try_from_bytes(&mut byte_stream) {
                    println!("broadcast message: {}", packet.message);

                } else if let Ok(packet) = ServerMessagePacket::try_from_bytes(&mut byte_stream) {
                    println!("server message: {}", packet.message);

                } else if let Ok(packet) = EntityMessagePacket::try_from_bytes(&mut byte_stream) {
                    println!("entity message: {}", packet.message);

                } else if let Ok(_packet) = DisplayEmotionPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(packet) = EntityMovePacket::try_from_bytes(&mut byte_stream) { 
                    let (origin, destination) = packet.from_to.to_vectors();
                    events.push(NetworkEvent::EntityMove(packet.entity_id as usize, origin, destination, packet.timestamp));

                } else if let Ok(packet) = PlayerMovePacket::try_from_bytes(&mut byte_stream) {
                    let (origin, destination) = packet.from_to.to_vectors();
                    events.push(NetworkEvent::PlayerMove(origin, destination, packet.timestamp));

                } else if let Ok(packet) = ChangeMapPacket::try_from_bytes(&mut byte_stream) { 
                    events.push(NetworkEvent::ChangeMap(packet.map_name.replace(".gat", "").to_string(), Vector2::new(packet.x as usize, packet.y as usize)));

                } else if let Ok(packet) = EntityAppearedPacket::try_from_bytes(&mut byte_stream) { 
                    events.push(NetworkEvent::AddEntity(packet.entity_id as usize, packet.job as usize, packet.position.to_vector(), packet.speed as usize));

                } else if let Ok(packet) = MovingEntityAppearedPacket::try_from_bytes(&mut byte_stream) { 
                    let (_origin, destination) = packet.position.to_vectors();
                    events.push(NetworkEvent::AddEntity(packet.entity_id as usize, packet.job as usize, destination, packet.speed as usize));

                } else if let Ok(packet) = EntityDisappearedPacket::try_from_bytes(&mut byte_stream) { 
                    events.push(NetworkEvent::RemoveEntity(packet.entity_id as usize));

                } else if let Ok(_packet) = UpdateStatusPacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = UpdateStatusPacket1::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = UpdateStatusPacket2::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = UpdateStatusPacket3::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = UpdateAttackRangePacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = NewMailStatusPacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = AchievementUpdatePacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = AchievementListPacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = CriticalWeightUpdatePacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = SpriteChangePacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = InventoyStartPacket::try_from_bytes(&mut byte_stream) { 

                    while InventoyEndPacket::try_from_bytes(&mut byte_stream).is_err() {
                        if let Ok(_packet) = EquippableItemListPacket::try_from_bytes(&mut byte_stream) {
                        } else {
                            panic!();
                        }
                    }

                } else if let Ok(_packet) = EquippableSwitchItemListPacket::try_from_bytes(&mut byte_stream) { 

                } else if let Ok(_packet) = MapTypePacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = UpdateSkillTreePacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = UpdateHotkeysPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = InitialStatusPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = UpdatePartyInvitationStatePacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = UpdateShowEquipPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = UpdateConfigurationPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = NavigateToMonsterPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = CloseScriptPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(_packet) = QuestNotificatonPacket::try_from_bytes(&mut byte_stream) {

                } else if let Ok(packet) = ServerTickPacket::try_from_bytes(&mut byte_stream) {
                    events.push(NetworkEvent::UpdataClientTick(packet.client_tick));

                } else {

                    #[cfg(feature = "debug_network")]
                    println!("{}unhandled{}: {:x?}", RED, NONE, byte_stream.remaining());

                    break;
                }
            }
        }

        events
    }
}
