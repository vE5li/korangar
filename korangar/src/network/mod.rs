mod login;

use std::cell::UnsafeCell;
use std::io::prelude::*;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::Duration;

use cgmath::Vector2;
use chrono::Local;
use derive_new::new;
use korangar_interface::elements::{PrototypeElement, WeakElementCell};
use korangar_interface::state::{
    PlainTrackedState, TrackedState, TrackedStateClone, TrackedStateExt, TrackedStateTake, TrackedStateVec, ValueState,
};
use korangar_procedural::profile;
use ragnarok_bytes::{ByteStream, ConversionError, ConversionResult, FromBytes};
use ragnarok_networking::*;

pub use self::login::LoginSettings;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Color;
use crate::interface::application::InterfaceSettings;
#[cfg(feature = "debug")]
use crate::interface::elements::PacketEntry;
#[cfg(feature = "debug")]
use crate::interface::windows::PacketWindow;
use crate::interface::windows::{CharacterSelectionWindow, FriendsWindow};
use crate::loaders::{ClientInfo, ServiceId};

#[cfg(feature = "debug")]
type PacketMetadata = Vec<PacketEntry>;
#[cfg(not(feature = "debug"))]
type PacketMetadata = ();

/// Extension trait for for [`ByteStream`] for working with network packets.
#[cfg(feature = "debug")]
pub trait ByteStreamNetworkExt {
    /// Push an [`IncomingPacket`] to the metadata.
    fn incoming_packet<T>(&mut self, packet: &T)
    where
        T: IncomingPacket + PrototypeElement<InterfaceSettings> + Clone + 'static;
}

#[cfg(feature = "debug")]
impl<'a> ByteStreamNetworkExt for ByteStream<'a, Vec<PacketEntry>> {
    fn incoming_packet<T>(&mut self, packet: &T)
    where
        T: IncomingPacket + PrototypeElement<InterfaceSettings> + Clone + 'static,
    {
        self.get_metadata_mut::<T, PacketMetadata>()
            .expect("wrong metadata")
            .push(PacketEntry::new_incoming(packet, std::any::type_name::<T>(), T::IS_PING));
    }
}

/// Extension trait for reading incoming packets and recording them into the
/// metadata of the [`ByteStream`] (only when the `debug` feature is active).
pub trait IncomingPacketRecord: IncomingPacket {
    /// Like [`IncomingPacket::payload_from_bytes`](ragnarok_networking::IncomingPacket::payload_from_bytes), but it records the packet into the metadata of the [`ByteStream`].
    fn payload_from_bytes_recorded(byte_stream: &mut ByteStream<PacketMetadata>) -> ConversionResult<Self>;

    /// Like [`IncomingPacketExt::packet_from_bytes`](ragnarok_networking::IncomingPacketExt::packet_from_bytes), but it records the packet into the metadata of the [`ByteStream`].
    fn packet_from_bytes_recorded(byte_stream: &mut ByteStream<PacketMetadata>) -> ConversionResult<Self>;
}

impl<T> IncomingPacketRecord for T
where
    T: IncomingPacket + PrototypeElement<InterfaceSettings> + 'static,
{
    fn payload_from_bytes_recorded(byte_stream: &mut ByteStream<PacketMetadata>) -> ConversionResult<Self> {
        let packet = Self::payload_from_bytes(byte_stream)?;

        #[cfg(feature = "debug")]
        byte_stream.incoming_packet(&packet);

        Ok(packet)
    }

    fn packet_from_bytes_recorded(byte_stream: &mut ByteStream<PacketMetadata>) -> ConversionResult<Self> {
        let packet = Self::packet_from_bytes(byte_stream)?;

        #[cfg(feature = "debug")]
        byte_stream.incoming_packet(&packet);

        Ok(packet)
    }
}

/// An event triggered by the map server.
pub enum NetworkEvent {
    /// Add an entity to the list of entities that the client is aware of.
    AddEntity(EntityData),
    /// Remove an entity from the list of entities that the client is aware of
    /// by its id.
    RemoveEntity(EntityId),
    /// The player is pathing to a new position.
    PlayerMove(Vector2<usize>, Vector2<usize>, ClientTick),
    /// An Entity nearby is pathing to a new position.
    EntityMove(EntityId, Vector2<usize>, Vector2<usize>, ClientTick),
    /// Player was moved to a new position on a different map or the current map
    ChangeMap(String, Vector2<usize>),
    /// Update the client side [`tick
    /// counter`](crate::system::GameTimer::base_client_tick) to keep server and
    /// client synchronized.
    UpdateClientTick(ClientTick),
    /// New chat message for the client.
    ChatMessage(ChatMessage),
    /// Update entity details. Mostly received when the client sends
    /// [RequestDetailsPacket] after the player hovered an entity.
    UpdateEntityDetails(EntityId, String),
    UpdateEntityHealth(EntityId, usize, usize),
    DamageEffect(EntityId, usize),
    HealEffect(EntityId, usize),
    UpdateStatus(StatusType),
    OpenDialog(String, EntityId),
    AddNextButton,
    AddCloseButton,
    AddChoiceButtons(Vec<String>),
    AddQuestEffect(QuestEffectPacket),
    RemoveQuestEffect(EntityId),
    Inventory(Vec<(ItemIndex, ItemId, EquipPosition, EquipPosition)>),
    AddIventoryItem(ItemIndex, ItemId, EquipPosition, EquipPosition),
    SkillTree(Vec<SkillInformation>),
    UpdateEquippedPosition {
        index: ItemIndex,
        equipped_position: EquipPosition,
    },
    ChangeJob(AccountId, u32),
    SetPlayerPosition(Vector2<usize>),
    Disconnect,
    FriendRequest(Friend),
    VisualEffect(&'static str, EntityId),
    AddSkillUnit(EntityId, UnitId, Vector2<usize>),
    RemoveSkillUnit(EntityId),
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
        let prefix = Local::now().format("^66BB44%H:%M:%S: ^000000").to_string();
        let offset = prefix.len();

        text.insert_str(0, &prefix);
        Self { text, color, offset }
    }

    pub fn stamped_text(&self, stamp: bool) -> &str {
        let start = self.offset * !stamp as usize;
        &self.text[start..]
    }
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
    pub sex: Sex,
}

impl EntityData {
    pub fn from_character(account_id: AccountId, character_information: CharacterInformation, position: Vector2<usize>) -> Self {
        Self {
            entity_id: EntityId(account_id.0),
            movement_speed: character_information.movement_speed as u16,
            job: character_information.job as u16,
            position,
            destination: None,
            health_points: character_information.health_points as i32,
            maximum_health_points: character_information.maximum_health_points as i32,
            head_direction: 0, // TODO: get correct rotation
            sex: character_information.sex,
        }
    }
}

impl From<EntityAppearedPacket> for EntityData {
    fn from(packet: EntityAppearedPacket) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job: packet.job,
            position: Vector2::new(packet.position.x, packet.position.y),
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
}

impl From<EntityAppeared2Packet> for EntityData {
    fn from(packet: EntityAppeared2Packet) -> Self {
        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job: packet.job,
            position: Vector2::new(packet.position.x, packet.position.y),
            destination: None,
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
}

impl From<MovingEntityAppearedPacket> for EntityData {
    fn from(packet: MovingEntityAppearedPacket) -> Self {
        let (origin, destination) = (
            Vector2::new(packet.position.x1, packet.position.y1),
            Vector2::new(packet.position.x2, packet.position.y2),
        );

        Self {
            entity_id: packet.entity_id,
            movement_speed: packet.movement_speed,
            job: packet.job,
            position: origin,
            destination: Some(destination),
            health_points: packet.health_points,
            maximum_health_points: packet.maximum_health_points,
            head_direction: packet.head_direction as usize,
            sex: packet.sex,
        }
    }
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

#[derive(new, Clone)]
struct LoginData {
    pub account_id: AccountId,
    pub login_id1: u32,
    pub login_id2: u32,
    pub sex: Sex,
}

// TODO: Use struct like this
// enum GameState {
//     LoggingIn {},
//     SelectingCharacter {
//         login_data: LoginData,
//         characters: TrackedState<Vec<CharacterInformation>>,
//         move_request: TrackedState<Option<usize>>,
//         slot_count: usize,
//     },
//     Playing {
//         friend_list: TrackedState<Vec<(Friend,
// UnsafeCell<Option<WeakElementCell<Application>>>)>>,         player_name:
// String,     },
// }

pub struct NetworkingSystem {
    login_stream: Option<TcpStream>,
    character_stream: Option<TcpStream>,
    map_stream: Option<TcpStream>,
    // TODO: Make this a heapless Vec or something
    map_stream_buffer: Vec<u8>,
    login_keep_alive_timer: NetworkTimer,
    character_keep_alive_timer: NetworkTimer,
    map_keep_alive_timer: NetworkTimer,

    // TODO: Move to GameState
    login_data: Option<LoginData>,
    characters: PlainTrackedState<Vec<CharacterInformation>>,
    move_request: PlainTrackedState<Option<usize>>,
    friend_list: PlainTrackedState<Vec<(Friend, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>)>>,
    slot_count: usize,
    player_name: String,
    #[cfg(feature = "debug")]
    update_packets: PlainTrackedState<bool>,
    #[cfg(feature = "debug")]
    packet_history: PlainTrackedState<RingBuffer<(PacketEntry, UnsafeCell<Option<WeakElementCell<InterfaceSettings>>>), 256>>,
}

impl NetworkingSystem {
    pub fn new() -> Self {
        let login_stream = None;
        let character_stream = None;
        let map_stream = None;
        let map_stream_buffer = Vec::new();
        let login_data = None;
        let characters = PlainTrackedState::default();
        let move_request = PlainTrackedState::default();
        let friend_list = PlainTrackedState::default();
        let slot_count = 0;
        let login_keep_alive_timer = NetworkTimer::new(Duration::from_secs(58));
        let character_keep_alive_timer = NetworkTimer::new(Duration::from_secs(10));
        let map_keep_alive_timer = NetworkTimer::new(Duration::from_secs(4));
        let player_name = String::new();
        #[cfg(feature = "debug")]
        let update_packets = PlainTrackedState::new(true);
        #[cfg(feature = "debug")]
        let packet_history = PlainTrackedState::default();

        Self {
            login_stream,
            character_stream,
            slot_count,
            login_data,
            map_stream,
            map_stream_buffer,
            characters,
            move_request,
            friend_list,
            login_keep_alive_timer,
            character_keep_alive_timer,
            map_keep_alive_timer,
            player_name,
            #[cfg(feature = "debug")]
            update_packets,
            #[cfg(feature = "debug")]
            packet_history,
        }
    }

    pub fn log_in(
        &mut self,
        client_info: &ClientInfo,
        service_id: ServiceId,
        username: String,
        password: String,
    ) -> Result<Vec<CharacterServerInformation>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("log in");

        let service = client_info
            .services
            .iter()
            .find(|service| service.service_id() == service_id)
            .unwrap();
        let service_address = format!("{}:{}", service.address, service.port);

        let login_stream = TcpStream::connect(service_address).map_err(|_| "failed to connect to login server".to_owned())?;
        login_stream.set_read_timeout(Duration::from_secs(1).into()).unwrap();
        self.login_stream = Some(login_stream);

        self.send_packet_to_login_server(LoginServerLoginPacket::new(username.clone(), password.clone()));

        let response = self.get_data_from_login_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let header = u16::from_bytes(&mut byte_stream).unwrap();
        let login_server_login_success_packet = match header {
            LoginFailedPacket::HEADER => {
                let packet = LoginFailedPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
                    LoginFailedReason::ServerClosed => return Err("server closed".to_string()),
                    LoginFailedReason::AlreadyLoggedIn => return Err("someone has already logged in with this id".to_string()),
                    LoginFailedReason::AlreadyOnline => return Err("already online".to_string()),
                }
            }
            LoginFailedPacket2::HEADER => {
                let packet = LoginFailedPacket2::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
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
            LoginServerLoginSuccessPacket::HEADER => LoginServerLoginSuccessPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap(),
            _ => panic!(),
        };

        self.login_data = Some(LoginData::new(
            login_server_login_success_packet.account_id,
            login_server_login_success_packet.login_id1,
            login_server_login_success_packet.login_id2,
            login_server_login_success_packet.sex,
        ));

        if login_server_login_success_packet.character_server_information.is_empty() {
            return Err("no character server available".to_string());
        }

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(login_server_login_success_packet.character_server_information)
    }

    pub fn select_server(&mut self, character_server_information: CharacterServerInformation) -> Result<(), String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("select server");

        let server_ip = IpAddr::V4(character_server_information.server_ip.into());
        let socket_address = SocketAddr::new(server_ip, character_server_information.server_port);
        self.character_stream = TcpStream::connect_timeout(&socket_address, Duration::from_secs(1))
            .map_err(|_| "Failed to connect to character server. Please try again")?
            .into();

        let login_data = self.login_data.clone().unwrap();

        let character_server_login_packet = CharacterServerLoginPacket::new(
            login_data.account_id,
            login_data.login_id1,
            login_data.login_id2,
            login_data.sex,
        );

        let character_stream = self.character_stream.as_mut().ok_or("no character server connection")?;
        character_stream
            .write_all(&character_server_login_packet.packet_to_bytes().unwrap())
            .map_err(|_| "failed to send packet to character server")?;

        let response = self.get_data_from_character_server();

        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);
        let account_id = AccountId::from_bytes(&mut byte_stream).unwrap();

        assert_eq!(account_id, login_data.account_id);

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        let response = self.get_data_from_character_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let header = u16::from_bytes(&mut byte_stream).unwrap();
        let character_server_login_success_packet = match header {
            LoginFailedPacket::HEADER => {
                let packet = LoginFailedPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
                    LoginFailedReason::ServerClosed => return Err("server closed".to_string()),
                    LoginFailedReason::AlreadyLoggedIn => return Err("someone has already logged in with this id".to_string()),
                    LoginFailedReason::AlreadyOnline => return Err("already online".to_string()),
                }
            }
            CharacterServerLoginSuccessPacket::HEADER => {
                CharacterServerLoginSuccessPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap()
            }
            _ => panic!(),
        };

        self.send_packet_to_character_server(RequestCharacterListPacket::default());

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        let response = self.get_data_from_character_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let request_character_list_success_packet =
            RequestCharacterListSuccessPacket::packet_from_bytes_recorded(&mut byte_stream).unwrap();
        self.characters.set(request_character_list_success_packet.character_information);

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        self.slot_count = character_server_login_success_packet.normal_slot_count as usize;

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(())
    }

    pub fn character_selection_window(&self) -> CharacterSelectionWindow {
        CharacterSelectionWindow::new(self.characters.new_remote(), self.move_request.new_remote(), self.slot_count)
    }

    pub fn friends_window(&self) -> FriendsWindow {
        FriendsWindow::new(self.friend_list.new_remote())
    }

    pub fn log_out(&mut self) -> Result<(), String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("log out");

        self.send_packet_to_map_server(RestartPacket::new(RestartType::Disconnect));

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(())
    }

    #[cfg(feature = "debug")]
    fn update_packet_history(&mut self, mut packets: Vec<PacketEntry>) {
        if self.update_packets.cloned() {
            self.packet_history.mutate(|buffer| {
                packets.drain(..).for_each(|packet| buffer.push((packet, UnsafeCell::new(None))));
            });
        }
    }

    #[cfg(feature = "debug")]
    fn new_outgoing<T>(&mut self, packet: &T)
    where
        T: OutgoingPacket + korangar_interface::elements::PrototypeElement<InterfaceSettings> + 'static,
    {
        if self.update_packets.cloned() {
            self.packet_history.mutate(|buffer| {
                buffer.push((
                    PacketEntry::new_outgoing(packet, std::any::type_name::<T>(), T::IS_PING),
                    UnsafeCell::new(None),
                ));
            });
        }
    }

    fn send_packet_to_login_server<T>(&mut self, packet: T)
    where
        T: OutgoingPacket + korangar_interface::elements::PrototypeElement<InterfaceSettings> + 'static,
    {
        #[cfg(feature = "debug")]
        self.new_outgoing(&packet);

        let packet_bytes = packet.packet_to_bytes().unwrap();
        let login_stream = self.login_stream.as_mut().expect("no login server connection");

        login_stream
            .write_all(&packet_bytes)
            .expect("failed to send packet to login server");
    }

    fn send_packet_to_character_server<T>(&mut self, packet: T)
    where
        T: OutgoingPacket + korangar_interface::elements::PrototypeElement<InterfaceSettings> + 'static,
    {
        #[cfg(feature = "debug")]
        self.new_outgoing(&packet);

        let packet_bytes = packet.packet_to_bytes().unwrap();
        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        character_stream
            .write_all(&packet_bytes)
            .expect("failed to send packet to character server");
    }

    fn send_packet_to_map_server<T>(&mut self, packet: T)
    where
        T: OutgoingPacket + korangar_interface::elements::PrototypeElement<InterfaceSettings> + 'static,
    {
        #[cfg(feature = "debug")]
        self.new_outgoing(&packet);

        let packet_bytes = packet.packet_to_bytes().unwrap();
        let map_stream = self.map_stream.as_mut().expect("no map server connection");
        map_stream.write_all(&packet_bytes).expect("failed to send packet to map server");
    }

    fn get_data_from_login_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let login_stream = self.login_stream.as_mut().expect("no login server connection");
        let response_length = login_stream.read(&mut buffer).expect("failed to get response from login server");
        buffer[..response_length].to_vec()
    }

    fn get_data_from_character_server(&mut self) -> Vec<u8> {
        let mut buffer = [0; 4096];
        let character_stream = self.character_stream.as_mut().expect("no character server connection");
        let response_length = character_stream
            .read(&mut buffer)
            .expect("failed to get response from character server");
        buffer[..response_length].to_vec()
    }

    fn try_get_data_from_map_server(&mut self) -> Option<Vec<u8>> {
        let mut buffer = [0; 8096];

        let stream_buffer_length = self.map_stream_buffer.len();
        let map_stream = self.map_stream.as_mut()?;
        let response_length = map_stream.read(&mut buffer[stream_buffer_length..]).ok()?;

        // We copy the buffered data *after* the read call, to save so unnecessary
        // computation.
        buffer[..stream_buffer_length].copy_from_slice(&self.map_stream_buffer);

        self.map_stream_buffer.clear();

        let total_length = stream_buffer_length + response_length;
        Some(buffer[..total_length].to_vec())
    }

    pub fn keep_alive(&mut self, delta_time: f64, client_tick: ClientTick) {
        if self.login_keep_alive_timer.update(delta_time) && self.login_stream.is_some() {
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
        #[cfg(feature = "debug")]
        let timer = Timer::new("create character");

        #[cfg(feature = "debug")]
        print_debug!(
            "character with name {}{}{} in slot {}{}{}",
            MAGENTA,
            name,
            NONE,
            MAGENTA,
            slot,
            NONE
        );

        let hair_color = 0;
        let hair_style = 0;
        let start_job = 0;
        let sex = Sex::Male;

        self.send_packet_to_character_server(CreateCharacterPacket::new(
            name, slot as u8, hair_color, hair_style, start_job, sex,
        ));

        let response = self.get_data_from_character_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let header = u16::from_bytes(&mut byte_stream).unwrap();
        let create_character_success_packet = match header {
            CharacterCreationFailedPacket::HEADER => {
                let packet = CharacterCreationFailedPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
                    CharacterCreationFailedReason::CharacterNameAlreadyUsed => return Err("character name is already used".to_string()),
                    CharacterCreationFailedReason::NotOldEnough => return Err("you are not old enough to create a character".to_string()),
                    CharacterCreationFailedReason::NotAllowedToUseSlot => {
                        return Err("you are not allowed to use that character slot".to_string());
                    }
                    CharacterCreationFailedReason::CharacterCerationFailed => return Err("character creation failed".to_string()),
                }
            }
            CreateCharacterSuccessPacket::HEADER => CreateCharacterSuccessPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap(),
            _ => panic!(),
        };

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        self.characters.push(create_character_success_packet.character_information);

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(())
    }

    pub fn delete_character(&mut self, character_id: CharacterId) -> Result<(), String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("delete character");

        let email = "a@a.com".to_string();

        #[cfg(feature = "debug")]
        print_debug!(
            "character with id {}{}{} and email {}{}{}",
            MAGENTA,
            character_id.0,
            NONE,
            MAGENTA,
            email,
            NONE
        );

        self.send_packet_to_character_server(DeleteCharacterPacket::new(character_id, email));

        let response = self.get_data_from_character_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let header = u16::from_bytes(&mut byte_stream).unwrap();
        match header {
            CharacterDeletionFailedPacket::HEADER => {
                let packet = CharacterDeletionFailedPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
                    CharacterDeletionFailedReason::NotAllowed => return Err("you are not allowed to delete this character".to_string()),
                    CharacterDeletionFailedReason::CharacterNotFound => return Err("character was not found".to_string()),
                    CharacterDeletionFailedReason::NotEligible => return Err("character is not eligible for deletion".to_string()),
                }
            }
            CharacterDeletionSuccessPacket::HEADER => {
                let _ = CharacterDeletionSuccessPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
            }
            _ => panic!(),
        }

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        self.characters.retain(|character| character.character_id != character_id);

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(())
    }

    pub fn select_character(&mut self, slot: usize) -> Result<(AccountId, CharacterInformation, String), String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("select character");

        #[cfg(feature = "debug")]
        print_debug!("character in slot {}{}{}", MAGENTA, slot, NONE,);

        self.send_packet_to_character_server(SelectCharacterPacket::new(slot as u8));

        let response = self.get_data_from_character_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let header = u16::from_bytes(&mut byte_stream).unwrap();
        let character_selection_success_packet = match header {
            CharacterSelectionFailedPacket::HEADER => {
                let packet = CharacterSelectionFailedPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
                    CharacterSelectionFailedReason::RejectedFromServer => return Err("rejected from server".to_string()),
                }
            }
            LoginFailedPacket::HEADER => {
                let packet = LoginFailedPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                match packet.reason {
                    LoginFailedReason::ServerClosed => return Err("Server closed".to_string()),
                    LoginFailedReason::AlreadyLoggedIn => return Err("Someone has already logged in with this ID".to_string()),
                    LoginFailedReason::AlreadyOnline => return Err("Already online".to_string()),
                }
            }
            MapServerUnavailablePacket::HEADER => {
                let _ = MapServerUnavailablePacket::payload_from_bytes_recorded(&mut byte_stream).unwrap();
                return Err("Map server currently unavailable".to_string());
            }
            CharacterSelectionSuccessPacket::HEADER => {
                CharacterSelectionSuccessPacket::payload_from_bytes_recorded(&mut byte_stream).unwrap()
            }
            _ => panic!(),
        };

        let server_ip = IpAddr::V4(character_selection_success_packet.map_server_ip.into());
        let server_port = character_selection_success_packet.map_server_port;

        #[cfg(feature = "debug")]
        print_debug!(
            "connecting to map server at {}{}{} on port {}{}{}",
            MAGENTA,
            server_ip,
            NONE,
            MAGENTA,
            character_selection_success_packet.map_server_port,
            NONE
        );

        let socket_address = SocketAddr::new(server_ip, server_port);
        let map_stream = TcpStream::connect_timeout(&socket_address, Duration::from_secs(1))
            .map_err(|_| "Failed to connect to map server. Please try again")?;

        map_stream.set_nonblocking(true).unwrap();
        self.map_stream = Some(map_stream);

        let login_data = self.login_data.as_ref().unwrap();
        let account_id = login_data.account_id;

        self.send_packet_to_map_server(MapServerLoginPacket::new(
            account_id,
            character_selection_success_packet.character_id,
            login_data.login_id1,
            ClientTick(100), // TODO: what is the logic here?
            login_data.sex,
        ));

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        let character_information = self
            .characters
            .get()
            .iter()
            .find(|character| character.character_number as usize == slot)
            .cloned()
            .unwrap();

        self.player_name = character_information.name.clone();

        #[cfg(feature = "debug")]
        timer.stop();

        Ok((
            account_id,
            character_information,
            character_selection_success_packet.map_name.replace(".gat", ""),
        ))
    }

    pub fn disconnect_from_map_server(&mut self) {
        // Dropping the TcpStream will also close the connection.
        self.map_stream = None;
    }

    pub fn request_switch_character_slot(&mut self, origin_slot: usize) {
        self.move_request.set(Some(origin_slot));
    }

    pub fn cancel_switch_character_slot(&mut self) {
        self.move_request.take();
    }

    pub fn switch_character_slot(&mut self, destination_slot: usize) -> Result<(), String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new("switch character slot");

        let origin_slot = self.move_request.take().unwrap();

        #[cfg(feature = "debug")]
        print_debug!(
            "from slot {}{}{} to slot {}{}{}",
            MAGENTA,
            origin_slot,
            NONE,
            MAGENTA,
            destination_slot,
            NONE
        );

        self.send_packet_to_character_server(SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16));

        let response = self.get_data_from_character_server();
        let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&response);

        let switch_character_slot_response_packet =
            SwitchCharacterSlotResponsePacket::packet_from_bytes_recorded(&mut byte_stream).unwrap();

        match switch_character_slot_response_packet.status {
            SwitchCharacterSlotResponseStatus::Success => {
                let _character_server_login_success_packet =
                    CharacterServerLoginSuccessPacket::packet_from_bytes_recorded(&mut byte_stream).unwrap();
                let _packet_006b = Packet6b00::packet_from_bytes_recorded(&mut byte_stream).unwrap();

                let character_count = self.characters.len();
                self.characters.clear();

                for _index in 0..character_count {
                    let character_information = CharacterInformation::from_bytes(&mut byte_stream).unwrap();
                    self.characters.push(character_information);
                }

                // packet_length and packet 0x09a0 are left unread because we
                // don't need them
            }
            SwitchCharacterSlotResponseStatus::Error => return Err("failed to move character to a different slot".to_string()),
        }

        #[cfg(feature = "debug")]
        self.update_packet_history(byte_stream.into_metadata());

        self.move_request.take();

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(())
    }

    pub fn request_player_move(&mut self, destination: Vector2<usize>) {
        self.send_packet_to_map_server(RequestPlayerMovePacket::new(WorldPosition::new(destination.x, destination.y)));
    }

    pub fn request_warp_to_map(&mut self, map_name: String, position: Vector2<usize>) {
        self.send_packet_to_map_server(RequestWarpToMapPacket::new(map_name, TilePosition {
            x: position.x as u16,
            y: position.y as u16,
        }));
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

    pub fn cast_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) {
        self.send_packet_to_map_server(UseSkillAtIdPacket::new(skill_level, skill_id, entity_id));
    }

    pub fn cast_ground_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, target_position: Vector2<u16>) {
        self.send_packet_to_map_server(UseSkillOnGroundPacket::new(skill_level, skill_id, TilePosition {
            x: target_position.x,
            y: target_position.y,
        }));
    }

    pub fn cast_channeling_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) {
        self.send_packet_to_map_server(StartUseSkillPacket::new(skill_id, skill_level, entity_id));
    }

    pub fn stop_channeling_skill(&mut self, skill_id: SkillId) {
        self.send_packet_to_map_server(EndUseSkillPacket::new(skill_id));
    }

    pub fn add_friend(&mut self, name: String) {
        if name.len() > 24 {
            #[cfg(feature = "debug")]
            print_debug!("[{RED}error{NONE}] friend name {MAGENTA}{name}{NONE} is too long",);

            return;
        }

        self.send_packet_to_map_server(AddFriendPacket::new(name));
    }

    pub fn remove_friend(&mut self, account_id: AccountId, character_id: CharacterId) {
        self.send_packet_to_map_server(RemoveFriendPacket::new(account_id, character_id));
    }

    pub fn reject_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) {
        self.send_packet_to_map_server(FriendRequestResponsePacket::new(
            account_id,
            character_id,
            FriendRequestResponse::Reject,
        ));
    }

    pub fn accept_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) {
        self.send_packet_to_map_server(FriendRequestResponsePacket::new(
            account_id,
            character_id,
            FriendRequestResponse::Accept,
        ));
    }

    #[profile]
    pub fn network_events(&mut self) -> Vec<NetworkEvent> {
        let mut events = Vec::new();

        while let Some(data) = self.try_get_data_from_map_server() {
            let mut byte_stream: ByteStream<PacketMetadata> = ByteStream::without_metadata(&data);

            while !byte_stream.is_empty() {
                let saved_offset = byte_stream.get_offset();

                // Packet is cut-off at the header
                let Ok(header) = u16::from_bytes(&mut byte_stream) else {
                    byte_stream.set_offset(saved_offset);
                    self.map_stream_buffer = byte_stream.remaining_bytes();
                    break;
                };

                match self.handle_packet(&mut byte_stream, header, &mut events) {
                    Ok(true) => {}
                    // Unknown packet
                    Ok(false) => {
                        #[cfg(feature = "debug")]
                        {
                            byte_stream.set_offset(saved_offset);
                            let packet = UnknownPacket::new(byte_stream.remaining_bytes());
                            byte_stream.incoming_packet(&packet);
                        }

                        break;
                    }
                    // Cut-off packet
                    Err(error) if error.is_byte_stream_too_short() => {
                        byte_stream.set_offset(saved_offset);
                        self.map_stream_buffer = byte_stream.remaining_bytes();
                        break;
                    }
                    Err(error) => panic!("{:?}", error),
                }
            }

            #[cfg(feature = "debug")]
            self.update_packet_history(byte_stream.into_metadata());
        }

        events
    }

    #[profile]
    fn handle_packet(
        &mut self,
        byte_stream: &mut ByteStream<PacketMetadata>,
        header: u16,
        events: &mut Vec<NetworkEvent>,
    ) -> ConversionResult<bool> {
        match header {
            BroadcastMessagePacket::HEADER => {
                let packet = BroadcastMessagePacket::payload_from_bytes_recorded(byte_stream)?;
                let color = Color::rgb_u8(220, 200, 30);
                let chat_message = ChatMessage::new(packet.message, color);
                events.push(NetworkEvent::ChatMessage(chat_message));
            }
            Broadcast2MessagePacket::HEADER => {
                let packet = Broadcast2MessagePacket::payload_from_bytes_recorded(byte_stream)?;
                // NOTE: Drop the alpha channel because it might be 0.
                let color = Color::rgb_u8(packet.font_color.red, packet.font_color.green, packet.font_color.blue);
                let chat_message = ChatMessage::new(packet.message, color);
                events.push(NetworkEvent::ChatMessage(chat_message));
            }
            OverheadMessagePacket::HEADER => {
                let packet = OverheadMessagePacket::payload_from_bytes_recorded(byte_stream)?;
                let color = Color::monochrome_u8(230);
                let chat_message = ChatMessage::new(packet.message, color);
                events.push(NetworkEvent::ChatMessage(chat_message));
            }
            ServerMessagePacket::HEADER => {
                let packet = ServerMessagePacket::payload_from_bytes_recorded(byte_stream)?;
                let chat_message = ChatMessage::new(packet.message, Color::monochrome_u8(255));
                events.push(NetworkEvent::ChatMessage(chat_message));
            }
            EntityMessagePacket::HEADER => {
                let packet = EntityMessagePacket::payload_from_bytes_recorded(byte_stream)?;
                // NOTE: Drop the alpha channel because it might be 0.
                let color = Color::rgb_u8(packet.color.red, packet.color.green, packet.color.blue);
                let chat_message = ChatMessage::new(packet.message, color);
                events.push(NetworkEvent::ChatMessage(chat_message));
            }
            DisplayEmotionPacket::HEADER => {
                let _packet = DisplayEmotionPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            EntityMovePacket::HEADER => {
                let packet = EntityMovePacket::payload_from_bytes_recorded(byte_stream)?;
                let (origin, destination) = (
                    Vector2::new(packet.from_to.x1, packet.from_to.y1),
                    Vector2::new(packet.from_to.x2, packet.from_to.y2),
                );
                events.push(NetworkEvent::EntityMove(
                    packet.entity_id,
                    origin,
                    destination,
                    packet.timestamp,
                ));
            }
            EntityStopMovePacket::HEADER => {
                let _packet = EntityStopMovePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            PlayerMovePacket::HEADER => {
                let packet = PlayerMovePacket::payload_from_bytes_recorded(byte_stream)?;
                let (origin, destination) = (
                    Vector2::new(packet.from_to.x1, packet.from_to.y1),
                    Vector2::new(packet.from_to.x2, packet.from_to.y2),
                );
                events.push(NetworkEvent::PlayerMove(origin, destination, packet.timestamp));
            }
            ChangeMapPacket::HEADER => {
                let packet = ChangeMapPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::ChangeMap(
                    packet.map_name.replace(".gat", ""),
                    Vector2::new(packet.position.x as usize, packet.position.y as usize),
                ));
            }
            EntityAppearedPacket::HEADER => {
                let packet = EntityAppearedPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddEntity(packet.into()));
            }
            EntityAppeared2Packet::HEADER => {
                let packet = EntityAppeared2Packet::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddEntity(packet.into()));
            }
            MovingEntityAppearedPacket::HEADER => {
                let packet = MovingEntityAppearedPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddEntity(packet.into()));
            }
            EntityDisappearedPacket::HEADER => {
                let packet = EntityDisappearedPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::RemoveEntity(packet.entity_id));
            }
            UpdateStatusPacket::HEADER => {
                let packet = UpdateStatusPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateStatus(packet.status_type));
            }
            UpdateStatusPacket1::HEADER => {
                let packet = UpdateStatusPacket1::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateStatus(packet.status_type));
            }
            UpdateStatusPacket2::HEADER => {
                let packet = UpdateStatusPacket2::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateStatus(packet.status_type));
            }
            UpdateStatusPacket3::HEADER => {
                let packet = UpdateStatusPacket3::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateStatus(packet.status_type));
            }
            UpdateAttackRangePacket::HEADER => {
                let _packet = UpdateAttackRangePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            NewMailStatusPacket::HEADER => {
                let _packet = NewMailStatusPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            AchievementUpdatePacket::HEADER => {
                let _packet = AchievementUpdatePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            AchievementListPacket::HEADER => {
                let _packet = AchievementListPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            CriticalWeightUpdatePacket::HEADER => {
                let _packet = CriticalWeightUpdatePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            SpriteChangePacket::HEADER => {
                let packet = SpriteChangePacket::payload_from_bytes_recorded(byte_stream)?;
                if packet.sprite_type == 0 {
                    events.push(NetworkEvent::ChangeJob(packet.account_id, packet.value));
                }
            }
            InventoyStartPacket::HEADER => {
                let _packet = InventoyStartPacket::payload_from_bytes_recorded(byte_stream)?;
                let mut item_data = Vec::new();

                // TODO: it might be better for performance and resilience to instead save a
                // state in the networking system instaed of buffering *all*
                // inventory packets if one of them is cut off
                loop {
                    let header = u16::from_bytes(byte_stream)?;

                    match header {
                        InventoyEndPacket::HEADER => {
                            break;
                        }
                        RegularItemListPacket::HEADER => {
                            let packet = RegularItemListPacket::payload_from_bytes_recorded(byte_stream)?;
                            for item_information in packet.item_information {
                                item_data.push((
                                    item_information.index,
                                    item_information.item_id,
                                    EquipPosition::None,
                                    EquipPosition::None,
                                )); // TODO: Don't add that data here, only equippable items need this data.
                            }
                        }
                        EquippableItemListPacket::HEADER => {
                            let packet = EquippableItemListPacket::payload_from_bytes_recorded(byte_stream)?;
                            for item_information in packet.item_information {
                                item_data.push((
                                    item_information.index,
                                    item_information.item_id,
                                    item_information.equip_position,
                                    item_information.equipped_position,
                                ));
                            }
                        }
                        _ => return Err(ConversionError::from_message("expected inventory packet")),
                    }
                }

                let _ = InventoyEndPacket::payload_from_bytes_recorded(byte_stream)?;

                events.push(NetworkEvent::Inventory(item_data));
            }
            EquippableSwitchItemListPacket::HEADER => {
                let _packet = EquippableSwitchItemListPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            MapTypePacket::HEADER => {
                let _packet = MapTypePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            UpdateSkillTreePacket::HEADER => {
                let packet = UpdateSkillTreePacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::SkillTree(packet.skill_information));
            }
            UpdateHotkeysPacket::HEADER => {
                let _packet = UpdateHotkeysPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            InitialStatusPacket::HEADER => {
                let _packet = InitialStatusPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            UpdatePartyInvitationStatePacket::HEADER => {
                let _packet = UpdatePartyInvitationStatePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            UpdateShowEquipPacket::HEADER => {
                let _packet = UpdateShowEquipPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            UpdateConfigurationPacket::HEADER => {
                let _packet = UpdateConfigurationPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            NavigateToMonsterPacket::HEADER => {
                let _packet = NavigateToMonsterPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            MarkMinimapPositionPacket::HEADER => {
                let _packet = MarkMinimapPositionPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            NextButtonPacket::HEADER => {
                let _packet = NextButtonPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddNextButton);
            }
            CloseButtonPacket::HEADER => {
                let _packet = CloseButtonPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddCloseButton);
            }
            DialogMenuPacket::HEADER => {
                let packet = DialogMenuPacket::payload_from_bytes_recorded(byte_stream)?;
                let choices = packet
                    .message
                    .split(':')
                    .map(String::from)
                    .filter(|text| !text.is_empty())
                    .collect();

                events.push(NetworkEvent::AddChoiceButtons(choices));
            }
            DisplaySpecialEffectPacket::HEADER => {
                let _packet = DisplaySpecialEffectPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            DisplaySkillCooldownPacket::HEADER => {
                let _packet = DisplaySkillCooldownPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            DisplaySkillEffectAndDamagePacket::HEADER => {
                let _packet = DisplaySkillEffectAndDamagePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            DisplaySkillEffectNoDamagePacket::HEADER => {
                let packet = DisplaySkillEffectNoDamagePacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::HealEffect(
                    packet.destination_entity_id,
                    packet.heal_amount as usize,
                ));

                //events.push(NetworkEvent::VisualEffect());
            }
            DisplayPlayerHealEffect::HEADER => {
                let _packet = DisplayPlayerHealEffect::payload_from_bytes_recorded(byte_stream)?;
            }
            StatusChangePacket::HEADER => {
                let _packet = StatusChangePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            QuestNotificationPacket1::HEADER => {
                let _packet = QuestNotificationPacket1::payload_from_bytes_recorded(byte_stream)?;
            }
            HuntingQuestNotificationPacket::HEADER => {
                let _packet = HuntingQuestNotificationPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            HuntingQuestUpdateObjectivePacket::HEADER => {
                let _packet = HuntingQuestUpdateObjectivePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            QuestRemovedPacket::HEADER => {
                let _packet = QuestRemovedPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            QuestListPacket::HEADER => {
                let _packet = QuestListPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            VisualEffectPacket::HEADER => {
                let packet = VisualEffectPacket::payload_from_bytes_recorded(byte_stream)?;
                let path = match packet.effect {
                    VisualEffect::BaseLevelUp => "angel.str",
                    VisualEffect::JobLevelUp => "joblvup.str",
                    VisualEffect::RefineFailure => "bs_refinefailed.str",
                    VisualEffect::RefineSuccess => "bs_refinesuccess.str",
                    VisualEffect::GameOver => "help_angel\\help_angel\\help_angel.str",
                    VisualEffect::PharmacySuccess => "p_success.str",
                    VisualEffect::PharmacyFailure => "p_failed.str",
                    VisualEffect::BaseLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
                    VisualEffect::JobLevelUpSuperNovice => "help_angel\\help_angel\\help_angel.str",
                    VisualEffect::BaseLevelUpTaekwon => "help_angel\\help_angel\\help_angel.str",
                };

                events.push(NetworkEvent::VisualEffect(path, packet.entity_id));
            }
            DisplayGainedExperiencePacket::HEADER => {
                let _packet = DisplayGainedExperiencePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            DisplayImagePacket::HEADER => {
                let _packet = DisplayImagePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            StateChangePacket::HEADER => {
                let _packet = StateChangePacket::payload_from_bytes_recorded(byte_stream)?;
            }

            QuestEffectPacket::HEADER => {
                let packet = QuestEffectPacket::payload_from_bytes_recorded(byte_stream)?;
                let event = match packet.effect {
                    QuestEffect::None => NetworkEvent::RemoveQuestEffect(packet.entity_id),
                    _ => NetworkEvent::AddQuestEffect(packet),
                };
                events.push(event);
            }
            ItemPickupPacket::HEADER => {
                let packet = ItemPickupPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddIventoryItem(
                    packet.index,
                    packet.item_id,
                    packet.equip_position,
                    EquipPosition::None,
                ));
            }
            RemoveItemFromInventoryPacket::HEADER => {
                let _packet = RemoveItemFromInventoryPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            ServerTickPacket::HEADER => {
                let packet = ServerTickPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateClientTick(packet.client_tick));
            }
            RequestPlayerDetailsSuccessPacket::HEADER => {
                let packet = RequestPlayerDetailsSuccessPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateEntityDetails(EntityId(packet.character_id.0), packet.name));
            }
            RequestEntityDetailsSuccessPacket::HEADER => {
                let packet = RequestEntityDetailsSuccessPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateEntityDetails(packet.entity_id, packet.name));
            }
            UpdateEntityHealthPointsPacket::HEADER => {
                let packet = UpdateEntityHealthPointsPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateEntityHealth(
                    packet.entity_id,
                    packet.health_points as usize,
                    packet.maximum_health_points as usize,
                ));
            }
            RequestPlayerAttackFailedPacket::HEADER => {
                let _packet = RequestPlayerAttackFailedPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            DamagePacket::HEADER => {
                let packet = DamagePacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::DamageEffect(
                    packet.destination_entity_id,
                    packet.damage_amount as usize,
                ));
            }
            NpcDialogPacket::HEADER => {
                let packet = NpcDialogPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::OpenDialog(packet.text, packet.npc_id));
            }
            RequestEquipItemStatusPacket::HEADER => {
                let packet = RequestEquipItemStatusPacket::payload_from_bytes_recorded(byte_stream)?;
                if let RequestEquipItemStatus::Success = packet.result {
                    events.push(NetworkEvent::UpdateEquippedPosition {
                        index: packet.inventory_index,
                        equipped_position: packet.equipped_position,
                    });
                }
            }
            RequestUnequipItemStatusPacket::HEADER => {
                let packet = RequestUnequipItemStatusPacket::payload_from_bytes_recorded(byte_stream)?;
                if let RequestUnequipItemStatus::Success = packet.result {
                    events.push(NetworkEvent::UpdateEquippedPosition {
                        index: packet.inventory_index,
                        equipped_position: EquipPosition::None,
                    });
                }
            }
            Packet8302::HEADER => {
                let _packet = Packet8302::payload_from_bytes_recorded(byte_stream)?;
            }
            Packet180b::HEADER => {
                let _packet = Packet180b::payload_from_bytes_recorded(byte_stream)?;
            }
            MapServerLoginSuccessPacket::HEADER => {
                let packet = MapServerLoginSuccessPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::UpdateClientTick(packet.client_tick));
                events.push(NetworkEvent::SetPlayerPosition(Vector2::new(
                    packet.position.x,
                    packet.position.y,
                )));
            }
            RestartResponsePacket::HEADER => {
                let packet = RestartResponsePacket::payload_from_bytes_recorded(byte_stream)?;
                match packet.result {
                    RestartResponseStatus::Ok => events.push(NetworkEvent::Disconnect),
                    RestartResponseStatus::Nothing => {
                        let color = Color::rgb_u8(255, 100, 100);
                        let chat_message = ChatMessage::new("Failed to log out.".to_string(), color);
                        events.push(NetworkEvent::ChatMessage(chat_message));
                    }
                }
            }
            DisconnectResponsePacket::HEADER => {
                let packet = DisconnectResponsePacket::payload_from_bytes_recorded(byte_stream)?;
                match packet.result {
                    DisconnectResponseStatus::Ok => events.push(NetworkEvent::Disconnect),
                    DisconnectResponseStatus::Wait10Seconds => {
                        let color = Color::rgb_u8(255, 100, 100);
                        let chat_message = ChatMessage::new("Please wait 10 seconds before trying to log out.".to_string(), color);
                        events.push(NetworkEvent::ChatMessage(chat_message));
                    }
                }
            }
            UseSkillSuccessPacket::HEADER => {
                let _packet = UseSkillSuccessPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            ToUseSkillSuccessPacket::HEADER => {
                let _packet = ToUseSkillSuccessPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            NotifySkillUnitPacket::HEADER => {
                let packet = NotifySkillUnitPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::AddSkillUnit(
                    packet.entity_id,
                    packet.unit_id,
                    Vector2::new(packet.position.x as usize, packet.position.y as usize),
                ));
            }
            SkillUnitDisappearPacket::HEADER => {
                let packet = SkillUnitDisappearPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::RemoveSkillUnit(packet.entity_id));
            }
            NotifyGroundSkillPacket::HEADER => {
                let _packet = NotifyGroundSkillPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            FriendListPacket::HEADER => {
                let packet = FriendListPacket::payload_from_bytes_recorded(byte_stream)?;
                self.friend_list.mutate(|friends| {
                    *friends = packet.friends.into_iter().map(|friend| (friend, UnsafeCell::new(None))).collect();
                });
            }
            FriendOnlineStatusPacket::HEADER => {
                let _packet = FriendOnlineStatusPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            FriendRequestPacket::HEADER => {
                let packet = FriendRequestPacket::payload_from_bytes_recorded(byte_stream)?;
                events.push(NetworkEvent::FriendRequest(packet.friend));
            }
            FriendRequestResultPacket::HEADER => {
                let packet = FriendRequestResultPacket::payload_from_bytes_recorded(byte_stream)?;
                if packet.result == FriendRequestResult::Accepted {
                    self.friend_list.push((packet.friend.clone(), UnsafeCell::new(None)));
                }

                let color = Color::rgb_u8(220, 200, 30);
                let chat_message = ChatMessage::new(packet.into_message(), color);
                events.push(NetworkEvent::ChatMessage(chat_message));
            }
            NotifyFriendRemovedPacket::HEADER => {
                let packet = NotifyFriendRemovedPacket::payload_from_bytes_recorded(byte_stream)?;
                self.friend_list.with_mut(|friends| {
                    friends.retain(|(friend, _)| !(friend.account_id == packet.account_id && friend.character_id == packet.character_id));
                    ValueState::Mutated(())
                });
            }
            PartyInvitePacket::HEADER => {
                let _packet = PartyInvitePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            StatusChangeSequencePacket::HEADER => {
                let _packet = StatusChangeSequencePacket::payload_from_bytes_recorded(byte_stream)?;
            }
            ReputationPacket::HEADER => {
                let _packet = ReputationPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            ClanInfoPacket::HEADER => {
                let _packet = ClanInfoPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            ClanOnlineCountPacket::HEADER => {
                let _packet = ClanOnlineCountPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            ChangeMapCellPacket::HEADER => {
                let _packet = ChangeMapCellPacket::payload_from_bytes_recorded(byte_stream)?;
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    #[cfg(feature = "debug")]
    pub fn clear_packet_history(&mut self) {
        self.packet_history.mutate(|buffer| {
            buffer.clear();
        });
    }

    #[cfg(feature = "debug")]
    pub fn packet_window(&self) -> PacketWindow<256> {
        PacketWindow::new(self.packet_history.new_remote(), self.update_packets.clone())
    }
}
