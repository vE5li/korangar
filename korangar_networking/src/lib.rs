mod entity;
mod event;
mod hotkey;
mod items;
mod message;
mod server;

use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::time::Duration;

use event::{
    CharacterServerDisconnectedEvent, DisconnectedEvent, LoginServerDisconnectedEvent, MapServerDisconnectedEvent, NetworkEventList,
    NoNetworkEvents,
};
use ragnarok_bytes::{ByteReader, FromBytes};
use ragnarok_packets::handler::{DuplicateHandlerError, HandlerResult, NoPacketCallback, PacketCallback, PacketHandler};
use ragnarok_packets::*;
use server::{ServerConnectCommand, ServerConnection};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;

pub use self::entity::EntityData;
pub use self::event::{DisconnectReason, NetworkEvent};
pub use self::hotkey::HotkeyState;
pub use self::items::{InventoryItem, InventoryItemDetails, ItemQuantity, NoMetadata, SellItem, ShopItem};
pub use self::message::MessageColor;
pub use self::server::{
    CharacterServerLoginData, LoginServerLoginData, NotConnectedError, UnifiedCharacterSelectionFailedReason, UnifiedLoginFailedReason,
};
use crate::server::NetworkTaskError;

/// Buffer for networking events. This struct exists to reduce heap allocations
/// and is purely an optimization.
pub struct NetworkEventBuffer(Vec<NetworkEvent>);

impl NetworkEventBuffer {
    pub fn drain(&mut self) -> std::vec::Drain<'_, NetworkEvent> {
        self.0.drain(..)
    }
}

pub struct NetworkingSystem<Callback> {
    command_sender: UnboundedSender<ServerConnectCommand>,
    login_server_connection: ServerConnection,
    character_server_connection: ServerConnection,
    map_server_connection: ServerConnection,
    packet_callback: Callback,
}

impl NetworkingSystem<NoPacketCallback> {
    pub fn spawn() -> (Self, NetworkEventBuffer) {
        let command_sender = Self::spawn_networking_thread(NoPacketCallback);

        Self::inner_new(command_sender, NoPacketCallback)
    }
}

impl<Callback> NetworkingSystem<Callback>
where
    Callback: PacketCallback + Send,
{
    fn inner_new(command_sender: UnboundedSender<ServerConnectCommand>, packet_callback: Callback) -> (Self, NetworkEventBuffer) {
        let networking_system = Self {
            command_sender,
            login_server_connection: ServerConnection::Disconnected,
            character_server_connection: ServerConnection::Disconnected,
            map_server_connection: ServerConnection::Disconnected,
            packet_callback,
        };
        let event_buffer = NetworkEventBuffer(Vec::new());

        (networking_system, event_buffer)
    }

    pub fn spawn_with_callback(packet_callback: Callback) -> (Self, NetworkEventBuffer) {
        let command_sender = Self::spawn_networking_thread(packet_callback.clone());

        Self::inner_new(command_sender, packet_callback)
    }

    fn spawn_networking_thread(packet_callback: Callback) -> UnboundedSender<ServerConnectCommand> {
        let (command_sender, mut command_receiver) = tokio::sync::mpsc::unbounded_channel::<ServerConnectCommand>();

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

            let _guard = runtime.enter();
            let local_set = tokio::task::LocalSet::new();

            let mut login_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;
            let mut character_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;
            let mut map_server_task_handle: Option<JoinHandle<Result<(), NetworkTaskError>>> = None;

            local_set.block_on(&runtime, async {
                while let Some(command) = command_receiver.recv().await {
                    match command {
                        ServerConnectCommand::Login {
                            address,
                            action_receiver,
                            event_sender,
                        } => {
                            if let Some(handle) = login_server_task_handle.take() {
                                // TODO: Maybe add a timeout here? Maybe handle Result?
                                let _ = handle.await.unwrap();
                            }

                            let packet_handler = Self::create_login_server_packet_handler(packet_callback.clone()).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                LoginServerKeepalivePacket::new,
                                Duration::from_secs(58),
                                false,
                            ));

                            login_server_task_handle = Some(handle);
                        }
                        ServerConnectCommand::Character {
                            address,
                            action_receiver,
                            event_sender,
                        } => {
                            if let Some(handle) = character_server_task_handle.take() {
                                // TODO: Maybe add a timeout here? Maybe handle Result?
                                let _ = handle.await.unwrap();
                            }

                            let packet_handler = Self::create_character_server_packet_handler(packet_callback.clone()).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                CharacterServerKeepalivePacket::new,
                                Duration::from_secs(10),
                                true,
                            ));

                            character_server_task_handle = Some(handle);
                        }
                        ServerConnectCommand::Map {
                            address,
                            action_receiver,
                            event_sender,
                        } => {
                            if let Some(handle) = map_server_task_handle.take() {
                                // TODO: Maybe add a timeout here? Maybe handle Result?
                                let _ = handle.await.unwrap();
                            }

                            let packet_handler = Self::create_map_server_packet_handler(packet_callback.clone()).unwrap();
                            let handle = local_set.spawn_local(Self::handle_server_connection(
                                address,
                                action_receiver,
                                event_sender,
                                packet_handler,
                                // Always passing 100 seems to work fine for now, but it might cause
                                // issues when connecting to something other than rAthena.
                                || RequestServerTickPacket::new(ClientTick(100)),
                                Duration::from_secs(4),
                                false,
                            ));

                            map_server_task_handle = Some(handle);
                        }
                    }
                }
            });
        });

        command_sender
    }

    fn handle_connection<Event>(connection: &mut ServerConnection, event_buffer: &mut NetworkEventBuffer)
    where
        Event: DisconnectedEvent,
    {
        match connection.take() {
            ServerConnection::Connected {
                action_sender,
                mut event_receiver,
            } => loop {
                match event_receiver.try_recv() {
                    Ok(login_event) => {
                        event_buffer.0.push(login_event);
                    }
                    Err(TryRecvError::Empty) => {
                        *connection = ServerConnection::Connected {
                            action_sender,
                            event_receiver,
                        };
                        break;
                    }
                    Err(..) => {
                        event_buffer.0.push(Event::create_event(DisconnectReason::ConnectionError));
                        *connection = ServerConnection::Disconnected;
                        break;
                    }
                }
            },
            ServerConnection::ClosingManually => {
                event_buffer.0.push(Event::create_event(DisconnectReason::ClosedByClient));
                *connection = ServerConnection::Disconnected;
            }
            _ => (),
        };
    }

    pub fn get_events(&mut self, events: &mut NetworkEventBuffer) {
        Self::handle_connection::<LoginServerDisconnectedEvent>(&mut self.login_server_connection, events);
        Self::handle_connection::<CharacterServerDisconnectedEvent>(&mut self.character_server_connection, events);
        Self::handle_connection::<MapServerDisconnectedEvent>(&mut self.map_server_connection, events);
    }

    async fn handle_server_connection<PingPacket>(
        address: SocketAddr,
        mut action_receiver: UnboundedReceiver<Vec<u8>>,
        event_sender: UnboundedSender<NetworkEvent>,
        mut packet_handler: PacketHandler<NetworkEventList, (), Callback>,
        ping_factory: impl Fn() -> PingPacket,
        ping_frequency: Duration,
        // After logging in to the character server, it sends the account id without any packet.
        // Since our packet handler has no way of working with this, we need to add some special
        // logic.
        mut read_account_id: bool,
    ) -> Result<(), NetworkTaskError>
    where
        PingPacket: Packet + ClientPacket,
        Callback: PacketCallback,
    {
        let mut stream = TcpStream::connect(address).await.map_err(|_| NetworkTaskError::FailedToConnect)?;
        let mut interval = tokio::time::interval(ping_frequency);
        let mut buffer = [0u8; 8192];
        let mut cut_off_buffer_base = 0;
        let mut events = Vec::new();

        loop {
            tokio::select! {
                // Send a packet to the server.
                action = action_receiver.recv() => {
                    let Some(action) = action else {
                        // Channel was closed by the main thread.
                        break Ok(());
                    };

                    stream.write_all(&action).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                }
                // Receive some packets from the server.
                received_bytes = stream.read(&mut buffer[cut_off_buffer_base..]) => {
                    let Ok(received_bytes) = received_bytes else {
                        // Channel was closed by the main thread.
                        break Err(NetworkTaskError::ConnectionClosed);
                    };

                    if received_bytes == 0 {
                        // Receiving Ok(0) means the stream was closed by the server, most
                        // likely because the client sent an incorrect packet.
                        break Err(NetworkTaskError::ConnectionClosed);
                    }

                    let data = &buffer[..cut_off_buffer_base + received_bytes];
                    let mut byte_reader = ByteReader::without_metadata(data);


                    if read_account_id {
                        let account_id = AccountId::from_bytes(&mut byte_reader).unwrap();
                        events.push(NetworkEvent::AccountId(account_id));
                        read_account_id = false;
                    }

                    while !byte_reader.is_empty() {
                        match packet_handler.process_one(&mut byte_reader) {
                            HandlerResult::Ok(packet_events) => events.extend(packet_events.0.into_iter()),
                            HandlerResult::PacketCutOff => {
                                let packet_start = byte_reader.get_offset();
                                let packet_end = cut_off_buffer_base + received_bytes;

                                if packet_start == 0 {
                                    // If the packet_start is 0, that means the packet is allegidly bigger than the MTU of a TCP packet.
                                    // We limit the size of a packet to the MTU, to avoid getting stuck on packets that are parsed incorrectly.
                                    // TODO: Call the packet callback?
                                    cut_off_buffer_base = 0;
                                    break;
                                }

                                buffer.copy_within(packet_start..packet_end, 0);
                                cut_off_buffer_base = packet_end - packet_start;

                                break;
                            },
                            // The packet callback can take care of handling these properly.
                            HandlerResult::UnhandledPacket => {
                                cut_off_buffer_base = 0;
                                break
                            },
                            HandlerResult::InternalError(..) => {
                                cut_off_buffer_base = 0;
                                break
                            },
                        }
                    }

                    for event in events.drain(..) {
                        event_sender.send(event).map_err(|_| NetworkTaskError::ConnectionClosed)?;
                    }
                }
                // Send a keep-alive packet to the server.
                _ = interval.tick() => {
                    let packet_bytes = ping_factory().packet_to_bytes().unwrap();
                    stream.write_all(&packet_bytes).await.map_err(|_| NetworkTaskError::ConnectionClosed)?;
                }
            }
        }
    }

    pub fn connect_to_login_server(&mut self, address: SocketAddr, username: impl Into<String>, password: impl Into<String>) {
        if !matches!(self.login_server_connection, ServerConnection::Disconnected) {
            return;
        }

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        self.command_sender
            .send(ServerConnectCommand::Login {
                address,
                action_receiver,
                event_sender,
            })
            .expect("network thread dropped");

        let login_packet = LoginServerLoginPacket::new(username.into(), password.into());

        self.packet_callback.outgoing_packet(&login_packet);

        action_sender
            .send(login_packet.packet_to_bytes().unwrap())
            .expect("action receiver instantly dropped");

        self.login_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
        };
    }

    pub fn connect_to_character_server(&mut self, login_data: &LoginServerLoginData, server: CharacterServerInformation) {
        if !matches!(self.character_server_connection, ServerConnection::Disconnected) {
            return;
        }

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let address = SocketAddr::new(IpAddr::V4(server.server_ip.into()), server.server_port);

        self.command_sender
            .send(ServerConnectCommand::Character {
                address,
                action_receiver,
                event_sender,
            })
            .expect("network thread dropped");

        let login_packet = CharacterServerLoginPacket::new(
            login_data.account_id,
            login_data.login_id1,
            login_data.login_id2,
            login_data.sex,
        );

        self.packet_callback.outgoing_packet(&login_packet);

        action_sender
            .send(login_packet.packet_to_bytes().unwrap())
            .expect("action receiver instantly dropped");

        self.character_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
        };
    }

    pub fn connect_to_map_server(
        &mut self,
        login_server_login_data: &LoginServerLoginData,
        character_server_login_data: CharacterServerLoginData,
    ) {
        if !matches!(self.map_server_connection, ServerConnection::Disconnected) {
            return;
        }

        let (action_sender, action_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let address = SocketAddr::new(character_server_login_data.server_ip, character_server_login_data.server_port);

        self.command_sender
            .send(ServerConnectCommand::Map {
                address,
                action_receiver,
                event_sender,
            })
            .expect("network thread dropped");

        let login_packet = MapServerLoginPacket::new(
            login_server_login_data.account_id,
            character_server_login_data.character_id,
            login_server_login_data.login_id1,
            // Always passing 100 seems to work fine for now, but it might cause
            // issues when connecting to something other than rAthena.
            ClientTick(100),
            login_server_login_data.sex,
        );

        self.packet_callback.outgoing_packet(&login_packet);

        action_sender
            .send(login_packet.packet_to_bytes().unwrap())
            .expect("action receiver instantly dropped");

        self.map_server_connection = ServerConnection::Connected {
            action_sender,
            event_receiver,
        };
    }

    pub fn disconnect_from_login_server(&mut self) {
        self.login_server_connection = ServerConnection::ClosingManually;
    }

    pub fn disconnect_from_character_server(&mut self) {
        self.character_server_connection = ServerConnection::ClosingManually;
    }

    pub fn disconnect_from_map_server(&mut self) {
        self.map_server_connection = ServerConnection::ClosingManually;
    }

    pub fn send_login_server_packet(&mut self, packet: &impl LoginServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.login_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(packet);

                // FIX: Don't unwrap.
                action_sender.send(packet.packet_to_bytes().unwrap()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    pub fn send_character_server_packet(&mut self, packet: &impl CharacterServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.character_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(packet);

                // FIX: Don't unwrap.
                action_sender.send(packet.packet_to_bytes().unwrap()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    pub fn send_map_server_packet(&mut self, packet: &impl MapServerPacket) -> Result<(), NotConnectedError> {
        match &mut self.map_server_connection {
            ServerConnection::Connected { action_sender, .. } => {
                self.packet_callback.outgoing_packet(packet);

                // FIX: Don't unwrap.
                action_sender.send(packet.packet_to_bytes().unwrap()).map_err(|_| NotConnectedError)
            }
            _ => Err(NotConnectedError),
        }
    }

    fn create_login_server_packet_handler(
        packet_callback: Callback,
    ) -> Result<PacketHandler<NetworkEventList, (), Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, (), Callback>::with_callback(packet_callback);

        packet_handler.register(|packet: LoginServerLoginSuccessPacket| NetworkEvent::LoginServerConnected {
            character_servers: packet.character_server_information,
            login_data: LoginServerLoginData {
                account_id: packet.account_id,
                login_id1: packet.login_id1,
                login_id2: packet.login_id2,
                sex: packet.sex,
            },
        })?;
        packet_handler.register(|packet: LoginFailedPacket| {
            let (reason, message) = match packet.reason {
                LoginFailedReason::ServerClosed => (UnifiedLoginFailedReason::ServerClosed, "Server closed"),
                LoginFailedReason::AlreadyLoggedIn => (
                    UnifiedLoginFailedReason::AlreadyLoggedIn,
                    "Someone has already logged in with this id",
                ),
                LoginFailedReason::AlreadyOnline => (UnifiedLoginFailedReason::AlreadyOnline, "Already online"),
            };

            NetworkEvent::LoginServerConnectionFailed { reason, message }
        })?;
        packet_handler.register(|packet: LoginFailedPacket2| {
            let (reason, message) = match packet.reason {
                LoginFailedReason2::UnregisteredId => (UnifiedLoginFailedReason::UnregisteredId, "Unregistered id"),
                LoginFailedReason2::IncorrectPassword => (UnifiedLoginFailedReason::IncorrectPassword, "Incorrect password"),
                LoginFailedReason2::IdExpired => (UnifiedLoginFailedReason::IdExpired, "Id has expired"),
                LoginFailedReason2::RejectedFromServer => (UnifiedLoginFailedReason::RejectedFromServer, "Rejected from server"),
                LoginFailedReason2::BlockedByGMTeam => (UnifiedLoginFailedReason::BlockedByGMTeam, "Blocked by gm team"),
                LoginFailedReason2::GameOutdated => (UnifiedLoginFailedReason::GameOutdated, "Game outdated"),
                LoginFailedReason2::LoginProhibitedUntil => (UnifiedLoginFailedReason::LoginProhibitedUntil, "Login prohibited until"),
                LoginFailedReason2::ServerFull => (UnifiedLoginFailedReason::ServerFull, "Server is full"),
                LoginFailedReason2::CompanyAccountLimitReached => (
                    UnifiedLoginFailedReason::CompanyAccountLimitReached,
                    "Company account limit reached",
                ),
            };

            NetworkEvent::LoginServerConnectionFailed { reason, message }
        })?;

        Ok(packet_handler)
    }

    fn create_character_server_packet_handler(
        packet_callback: Callback,
    ) -> Result<PacketHandler<NetworkEventList, (), Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, (), Callback>::with_callback(packet_callback);

        packet_handler.register(|packet: LoginFailedPacket| {
            let reason = packet.reason;
            let message = match reason {
                LoginFailedReason::ServerClosed => "Server closed",
                LoginFailedReason::AlreadyLoggedIn => "Someone has already logged in with this id",
                LoginFailedReason::AlreadyOnline => "Already online",
            };

            NetworkEvent::CharacterServerConnectionFailed { reason, message }
        })?;
        packet_handler.register(
            |packet: CharacterServerLoginSuccessPacket| NetworkEvent::CharacterServerConnected {
                normal_slot_count: packet.normal_slot_count as usize,
            },
        )?;
        packet_handler.register(|packet: RequestCharacterListSuccessPacket| NetworkEvent::CharacterList {
            characters: packet.character_information,
        })?;
        packet_handler.register_noop::<Packet006b>()?;
        packet_handler.register_noop::<Packet0b18>()?;
        packet_handler.register(|packet: CharacterSelectionSuccessPacket| {
            let login_data = CharacterServerLoginData {
                server_ip: IpAddr::V4(packet.map_server_ip.into()),
                server_port: packet.map_server_port,
                character_id: packet.character_id,
            };
            let map_name = packet.map_name.strip_suffix(".gat").unwrap().to_owned();

            NetworkEvent::CharacterSelected { login_data, map_name }
        })?;
        packet_handler.register(|packet: CharacterSelectionFailedPacket| {
            let (reason, message) = match packet.reason {
                CharacterSelectionFailedReason::RejectedFromServer => (
                    UnifiedCharacterSelectionFailedReason::RejectedFromServer,
                    "Rejected from server",
                ),
            };

            NetworkEvent::CharacterSelectionFailed { reason, message }
        })?;
        packet_handler.register(|_: MapServerUnavailablePacket| {
            let reason = UnifiedCharacterSelectionFailedReason::MapServerUnavailable;
            let message = "Map server currently unavailable";

            NetworkEvent::CharacterSelectionFailed { reason, message }
        })?;
        packet_handler.register(|packet: CreateCharacterSuccessPacket| NetworkEvent::CharacterCreated {
            character_information: packet.character_information,
        })?;
        packet_handler.register(|packet: CharacterCreationFailedPacket| {
            let reason = packet.reason;
            let message = match reason {
                CharacterCreationFailedReason::CharacterNameAlreadyUsed => "Character name is already used",
                CharacterCreationFailedReason::NotOldEnough => "You are not old enough to create a character",
                CharacterCreationFailedReason::NotAllowedToUseSlot => "You are not allowed to use this character slot",
                CharacterCreationFailedReason::CharacterCerationFailed => "Character creation failed",
            };

            NetworkEvent::CharacterCreationFailed { reason, message }
        })?;
        packet_handler.register(|_: CharacterDeletionSuccessPacket| NetworkEvent::CharacterDeleted)?;
        packet_handler.register(|packet: CharacterDeletionFailedPacket| {
            let reason = packet.reason;
            let message = match reason {
                CharacterDeletionFailedReason::NotAllowed => "You are not allowed to delete this character",
                CharacterDeletionFailedReason::CharacterNotFound => "Character was not found",
                CharacterDeletionFailedReason::NotEligible => "Character is not eligible for deletion",
            };
            NetworkEvent::CharacterDeletionFailed { reason, message }
        })?;
        packet_handler.register(|packet: SwitchCharacterSlotResponsePacket| match packet.status {
            SwitchCharacterSlotResponseStatus::Success => NetworkEvent::CharacterSlotSwitched,
            SwitchCharacterSlotResponseStatus::Error => NetworkEvent::CharacterSlotSwitchFailed,
        })?;

        Ok(packet_handler)
    }

    fn create_map_server_packet_handler(
        packet_callback: Callback,
    ) -> Result<PacketHandler<NetworkEventList, (), Callback>, DuplicateHandlerError> {
        let mut packet_handler = PacketHandler::<NetworkEventList, (), Callback>::with_callback(packet_callback);

        // This is a bit of a workaround for the way that the inventory is
        // sent. There is a single packet to start the inventory list,
        // followed by an arbitary number of item packets, and in the
        // end a sinle packet to mark the list as complete.
        //
        // This variable provides some transient storage shared by all the inventory
        // handlers.
        let inventory_items: Rc<RefCell<Option<Vec<InventoryItem<NoMetadata>>>>> = Rc::new(RefCell::new(None));

        packet_handler.register(|_: MapServerPingPacket| NoNetworkEvents)?;
        packet_handler.register(|packet: BroadcastMessagePacket| NetworkEvent::ChatMessage {
            text: packet.message,
            color: MessageColor::Broadcast,
        })?;
        packet_handler.register(|packet: Broadcast2MessagePacket| {
            // Drop the alpha channel because it might be 0.
            let color = MessageColor::Rgb {
                red: packet.font_color.red,
                green: packet.font_color.green,
                blue: packet.font_color.blue,
            };
            NetworkEvent::ChatMessage {
                text: packet.message,
                color,
            }
        })?;
        packet_handler.register(|packet: OverheadMessagePacket| {
            // FIX: This should be a different event.
            NetworkEvent::ChatMessage {
                text: packet.message,
                color: MessageColor::Broadcast,
            }
        })?;
        packet_handler.register(|packet: ServerMessagePacket| NetworkEvent::ChatMessage {
            text: packet.message,
            color: MessageColor::Server,
        })?;
        packet_handler.register(|packet: EntityMessagePacket| {
            // Drop the alpha channel because it might be 0.
            let color = MessageColor::Rgb {
                red: packet.color.red,
                green: packet.color.green,
                blue: packet.color.blue,
            };
            NetworkEvent::ChatMessage {
                text: packet.message,
                color,
            }
        })?;
        packet_handler.register_noop::<DisplayEmotionPacket>()?;
        packet_handler.register(|packet: EntityMovePacket| {
            let (origin, destination) = packet.from_to.to_origin_destination();
            NetworkEvent::EntityMove(packet.entity_id, origin, destination, packet.timestamp)
        })?;
        packet_handler.register_noop::<EntityStopMovePacket>()?;
        packet_handler.register(|packet: PlayerMovePacket| {
            let (origin, destination) = packet.from_to.to_origin_destination();
            NetworkEvent::PlayerMove(origin, destination, packet.timestamp)
        })?;
        packet_handler.register(|packet: ChangeMapPacket| NetworkEvent::ChangeMap(packet.map_name.replace(".gat", ""), packet.position))?;
        packet_handler.register(|packet: ResurrectionPacket| NetworkEvent::ResurrectPlayer {
            entity_id: packet.entity_id,
        })?;
        packet_handler.register(|packet: EntityAppearedPacket| NetworkEvent::AddEntity(packet.into()))?;
        packet_handler.register(|packet: EntityAppeared2Packet| NetworkEvent::AddEntity(packet.into()))?;
        packet_handler.register(|packet: MovingEntityAppearedPacket| NetworkEvent::AddEntity(packet.into()))?;
        packet_handler.register(|packet: EntityDisappearedPacket| NetworkEvent::RemoveEntity {
            entity_id: packet.entity_id,
            reason: packet.reason,
        })?;
        packet_handler.register(|packet: UpdateStatusPacket| NetworkEvent::UpdateStatus(packet.status_type))?;
        packet_handler.register(|packet: UpdateStatusPacket1| NetworkEvent::UpdateStatus(packet.status_type))?;
        packet_handler.register(|packet: UpdateStatusPacket2| NetworkEvent::UpdateStatus(packet.status_type))?;
        packet_handler.register(|packet: UpdateStatusPacket3| NetworkEvent::UpdateStatus(packet.status_type))?;
        packet_handler.register_noop::<UpdateAttackRangePacket>()?;
        packet_handler.register_noop::<NewMailStatusPacket>()?;
        packet_handler.register_noop::<AchievementUpdatePacket>()?;
        packet_handler.register_noop::<AchievementListPacket>()?;
        packet_handler.register_noop::<CriticalWeightUpdatePacket>()?;
        packet_handler.register(|packet: SpriteChangePacket| match packet.sprite_type {
            SpriteChangeType::Base => Some(NetworkEvent::ChangeJob {
                account_id: packet.account_id,
                job_id: packet.value,
            }),
            SpriteChangeType::Hair => Some(NetworkEvent::ChangeHair {
                account_id: packet.account_id,
                hair_id: packet.value,
            }),
            _ => None,
        })?;
        packet_handler.register({
            let inventory_items = inventory_items.clone();

            move |_: InventoyStartPacket| {
                *inventory_items.borrow_mut() = Some(Vec::new());
                NoNetworkEvents
            }
        })?;
        packet_handler.register({
            let inventory_items = inventory_items.clone();

            move |packet: RegularItemListPacket| {
                inventory_items.borrow_mut().as_mut().expect("Unexpected inventory packet").extend(
                    packet.item_information.into_iter().map(|item_information| {
                        let RegularItemInformation {
                            index,
                            item_id,
                            item_type,
                            amount,
                            equipped_position,
                            slot,
                            hire_expiration_date,
                            flags,
                        } = item_information;

                        InventoryItem {
                            index,
                            metadata: NoMetadata,
                            item_id,
                            item_type,
                            slot,
                            hire_expiration_date,
                            details: InventoryItemDetails::Regular {
                                amount,
                                equipped_position,
                                flags,
                            },
                        }
                    }),
                );
                NoNetworkEvents
            }
        })?;
        packet_handler.register({
            let inventory_items = inventory_items.clone();

            move |packet: EquippableItemListPacket| {
                inventory_items.borrow_mut().as_mut().expect("Unexpected inventory packet").extend(
                    packet.item_information.into_iter().map(|item| {
                        let EquippableItemInformation {
                            index,
                            item_id,
                            item_type,
                            equip_position,
                            equipped_position,
                            slot,
                            hire_expiration_date,
                            bind_on_equip_type,
                            w_item_sprite_number,
                            option_count,
                            option_data,
                            refinement_level,
                            enchantment_level,
                            flags,
                        } = item;

                        InventoryItem {
                            index,
                            metadata: NoMetadata,
                            item_id,
                            item_type,
                            slot,
                            hire_expiration_date,
                            details: InventoryItemDetails::Equippable {
                                equip_position,
                                equipped_position,
                                bind_on_equip_type,
                                w_item_sprite_number,
                                option_count,
                                option_data,
                                refinement_level,
                                enchantment_level,
                                flags,
                            },
                        }
                    }),
                );
                NoNetworkEvents
            }
        })?;
        packet_handler.register({
            let inventory_items = inventory_items.clone();

            move |_: InventoyEndPacket| {
                let items = inventory_items.borrow_mut().take().expect("Unexpected inventory end packet");
                NetworkEvent::SetInventory { items }
            }
        })?;
        packet_handler.register_noop::<EquippableSwitchItemListPacket>()?;
        packet_handler.register_noop::<MapTypePacket>()?;
        packet_handler.register(|packet: UpdateSkillTreePacket| NetworkEvent::SkillTree(packet.skill_information))?;
        packet_handler.register(|packet: UpdateHotkeysPacket| NetworkEvent::SetHotkeyData {
            tab: packet.tab,
            hotkeys: packet
                .hotkeys
                .into_iter()
                .map(|hotkey_data| match hotkey_data == HotkeyData::UNBOUND {
                    true => HotkeyState::Unbound,
                    false => HotkeyState::Bound(hotkey_data),
                })
                .collect(),
        })?;
        packet_handler.register_noop::<InitialStatusPacket>()?;
        packet_handler.register_noop::<UpdatePartyInvitationStatePacket>()?;
        packet_handler.register_noop::<UpdateShowEquipPacket>()?;
        packet_handler.register_noop::<UpdateConfigurationPacket>()?;
        packet_handler.register_noop::<NavigateToMonsterPacket>()?;
        packet_handler.register_noop::<MarkMinimapPositionPacket>()?;
        packet_handler.register(|_: NextButtonPacket| NetworkEvent::AddNextButton)?;
        packet_handler.register(|_: CloseButtonPacket| NetworkEvent::AddCloseButton)?;
        packet_handler.register(|packet: DialogMenuPacket| {
            let choices = packet
                .message
                .split(':')
                .map(String::from)
                .filter(|text| !text.is_empty())
                .collect();

            NetworkEvent::AddChoiceButtons(choices)
        })?;
        packet_handler.register_noop::<DisplaySpecialEffectPacket>()?;
        packet_handler.register_noop::<DisplaySkillCooldownPacket>()?;
        packet_handler.register_noop::<DisplaySkillEffectAndDamagePacket>()?;
        packet_handler.register(|packet: DisplaySkillEffectNoDamagePacket| {
            NetworkEvent::HealEffect(packet.destination_entity_id, packet.heal_amount as usize)
        })?;
        packet_handler.register_noop::<DisplayPlayerHealEffect>()?;
        packet_handler.register_noop::<StatusChangePacket>()?;
        packet_handler.register_noop::<QuestNotificationPacket1>()?;
        packet_handler.register_noop::<HuntingQuestNotificationPacket>()?;
        packet_handler.register_noop::<HuntingQuestUpdateObjectivePacket>()?;
        packet_handler.register_noop::<QuestRemovedPacket>()?;
        packet_handler.register_noop::<QuestListPacket>()?;
        packet_handler.register(|packet: VisualEffectPacket| {
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

            NetworkEvent::VisualEffect(path, packet.entity_id)
        })?;
        packet_handler.register_noop::<DisplayGainedExperiencePacket>()?;
        packet_handler.register_noop::<DisplayImagePacket>()?;
        packet_handler.register_noop::<StateChangePacket>()?;

        packet_handler.register(|packet: QuestEffectPacket| match packet.effect {
            QuestEffect::None => NetworkEvent::RemoveQuestEffect(packet.entity_id),
            _ => NetworkEvent::AddQuestEffect(packet),
        })?;
        packet_handler.register(|packet: ItemPickupPacket| {
            let ItemPickupPacket {
                index,
                count,
                item_id,
                is_identified,
                is_broken,
                cards,
                equip_position,
                item_type,
                result,
                hire_expiration_date,
                bind_on_equip_type,
                option_data,
                favorite,
                look,
                refinement_level,
                enchantment_level,
            } = packet;

            if result != ItemPickupResult::Success {
                todo!();
            }

            // TODO: Not sure where to store these, since the *InventoryItem packets are not
            // sending these either. We will certainly use them at some point though.
            let _ = (favorite, look);

            let details = match equip_position.is_empty() {
                true => InventoryItemDetails::Regular {
                    amount: count,
                    equipped_position: equip_position,
                    flags: {
                        let mut flags = RegularItemFlags::empty();
                        flags.set(RegularItemFlags::IDENTIFIED, is_identified != 0);
                        flags
                    },
                },
                false => InventoryItemDetails::Equippable {
                    equip_position,
                    equipped_position: EquipPosition::empty(),
                    bind_on_equip_type,
                    w_item_sprite_number: 0,
                    option_count: option_data.len() as u8,
                    option_data,
                    refinement_level,
                    enchantment_level,
                    flags: {
                        let mut flags = EquippableItemFlags::empty();
                        flags.set(EquippableItemFlags::IDENTIFIED, is_identified != 0);
                        flags.set(EquippableItemFlags::IS_BROKEN, is_broken != 0);
                        flags
                    },
                },
            };

            let item = InventoryItem {
                metadata: NoMetadata,
                index,
                item_id,
                item_type,
                slot: cards,
                hire_expiration_date,
                details,
            };

            NetworkEvent::IventoryItemAdded { item }
        })?;
        packet_handler.register(|packet: RemoveItemFromInventoryPacket| NetworkEvent::InventoryItemRemoved {
            reason: packet.remove_reason,
            index: packet.index,
            amount: packet.amount,
        })?;
        packet_handler.register(|packet: ServerTickPacket| NetworkEvent::UpdateClientTick(packet.client_tick))?;
        packet_handler.register(|packet: RequestPlayerDetailsSuccessPacket| {
            NetworkEvent::UpdateEntityDetails(EntityId(packet.character_id.0), packet.name)
        })?;
        packet_handler
            .register(|packet: RequestEntityDetailsSuccessPacket| NetworkEvent::UpdateEntityDetails(packet.entity_id, packet.name))?;
        packet_handler.register(|packet: UpdateEntityHealthPointsPacket| {
            NetworkEvent::UpdateEntityHealth(
                packet.entity_id,
                packet.health_points as usize,
                packet.maximum_health_points as usize,
            )
        })?;
        packet_handler.register_noop::<RequestPlayerAttackFailedPacket>()?;
        packet_handler.register(|packet: DamagePacket1| match packet.damage_type {
            DamageType::Damage => Some(NetworkEvent::DamageEffect {
                entity_id: packet.destination_entity_id,
                damage_amount: packet.damage_amount as usize,
            }),
            DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
                entity_id: packet.destination_entity_id,
            }),
            _ => None,
        })?;
        packet_handler.register(|packet: DamagePacket3| match packet.damage_type {
            DamageType::Damage => Some(NetworkEvent::DamageEffect {
                entity_id: packet.destination_entity_id,
                damage_amount: packet.damage_amount as usize,
            }),
            DamageType::StandUp => Some(NetworkEvent::PlayerStandUp {
                entity_id: packet.destination_entity_id,
            }),
            _ => None,
        })?;
        packet_handler.register(|packet: NpcDialogPacket| NetworkEvent::OpenDialog(packet.text, packet.npc_id))?;
        packet_handler.register(|packet: RequestEquipItemStatusPacket| match packet.result {
            RequestEquipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
                index: packet.inventory_index,
                equipped_position: packet.equipped_position,
            }),
            _ => None,
        })?;
        packet_handler.register(|packet: RequestUnequipItemStatusPacket| match packet.result {
            RequestUnequipItemStatus::Success => Some(NetworkEvent::UpdateEquippedPosition {
                index: packet.inventory_index,
                equipped_position: EquipPosition::NONE,
            }),
            _ => None,
        })?;
        packet_handler.register_noop::<Packet8302>()?;
        packet_handler.register_noop::<Packet0b18>()?;
        packet_handler.register(|packet: MapServerLoginSuccessPacket| {
            (
                NetworkEvent::UpdateClientTick(packet.client_tick),
                NetworkEvent::SetPlayerPosition(packet.position),
            )
        })?;
        packet_handler.register(|packet: RestartResponsePacket| match packet.result {
            RestartResponseStatus::Ok => NetworkEvent::LoggedOut,
            RestartResponseStatus::Nothing => NetworkEvent::ChatMessage {
                text: "Failed to log out.".to_string(),
                color: MessageColor::Error,
            },
        })?;
        packet_handler.register(|packet: DisconnectResponsePacket| match packet.result {
            DisconnectResponseStatus::Ok => NetworkEvent::LoggedOut,
            DisconnectResponseStatus::Wait10Seconds => NetworkEvent::ChatMessage {
                text: "Please wait 10 seconds before trying to log out.".to_string(),
                color: MessageColor::Error,
            },
        })?;
        packet_handler.register_noop::<UseSkillSuccessPacket>()?;
        packet_handler.register_noop::<ToUseSkillSuccessPacket>()?;
        packet_handler
            .register(|packet: NotifySkillUnitPacket| NetworkEvent::AddSkillUnit(packet.entity_id, packet.unit_id, packet.position))?;
        packet_handler.register(|packet: SkillUnitDisappearPacket| NetworkEvent::RemoveSkillUnit(packet.entity_id))?;
        packet_handler.register_noop::<NotifyGroundSkillPacket>()?;
        packet_handler.register(|packet: FriendListPacket| NetworkEvent::SetFriendList { friends: packet.friends })?;
        packet_handler.register_noop::<FriendOnlineStatusPacket>()?;
        packet_handler.register(|packet: FriendRequestPacket| NetworkEvent::FriendRequest {
            requestee: packet.requestee,
        })?;
        packet_handler.register(|packet: FriendRequestResultPacket| {
            let text = match packet.result {
                FriendRequestResult::Accepted => format!("You have become friends with {}.", packet.friend.name),
                FriendRequestResult::Rejected => format!("{} does not want to be friends with you.", packet.friend.name),
                FriendRequestResult::OwnFriendListFull => "Your Friend List is full.".to_owned(),
                FriendRequestResult::OtherFriendListFull => format!("{}'s Friend List is full.", packet.friend.name),
            };

            let mut events = vec![NetworkEvent::ChatMessage {
                text,
                color: MessageColor::Information,
            }];

            if matches!(packet.result, FriendRequestResult::Accepted) {
                events.push(NetworkEvent::FriendAdded { friend: packet.friend });
            }

            events
        })?;
        packet_handler.register(|packet: NotifyFriendRemovedPacket| NetworkEvent::FriendRemoved {
            account_id: packet.account_id,
            character_id: packet.character_id,
        })?;
        packet_handler.register_noop::<PartyInvitePacket>()?;
        packet_handler.register_noop::<StatusChangeSequencePacket>()?;
        packet_handler.register_noop::<ReputationPacket>()?;
        packet_handler.register_noop::<ClanInfoPacket>()?;
        packet_handler.register_noop::<ClanOnlineCountPacket>()?;
        packet_handler.register_noop::<ChangeMapCellPacket>()?;
        packet_handler.register_noop::<OpenMarketPacket>()?;
        packet_handler.register(|packet: BuyOrSellPacket| NetworkEvent::AskBuyOrSell { shop_id: packet.shop_id })?;
        packet_handler.register(|packet: ShopItemListPacket| {
            let items = packet
                .items
                .into_iter()
                .map(|item| ShopItem {
                    metadata: NoMetadata,
                    item_id: item.item_id,
                    item_type: item.item_type,
                    price: item.price,
                    quantity: items::ItemQuantity::Infinite,
                    weight: 0,
                    location: item.location,
                })
                .collect();

            NetworkEvent::OpenShop { items }
        })?;
        packet_handler.register(|packet: BuyShopItemsResultPacket| NetworkEvent::BuyingCompleted { result: packet.result })?;
        packet_handler.register_noop::<ParameterChangePacket>()?;
        packet_handler.register(|packet: SellListPacket| NetworkEvent::SellItemList { items: packet.items })?;
        packet_handler.register(|packet: SellItemsResultPacket| NetworkEvent::SellingCompleted { result: packet.result })?;

        Ok(packet_handler)
    }

    pub fn request_character_list(&mut self) -> Result<(), NotConnectedError> {
        self.send_character_server_packet(&RequestCharacterListPacket::default())
    }

    pub fn select_character(&mut self, character_slot: usize) -> Result<(), NotConnectedError> {
        self.send_character_server_packet(&SelectCharacterPacket::new(character_slot as u8))
    }

    pub fn map_loaded(&mut self) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&MapLoadedPacket::default())
    }

    pub fn respawn(&mut self) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RestartPacket::new(RestartType::Respawn))
    }

    pub fn log_out(&mut self) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RestartPacket::new(RestartType::Disconnect))
    }

    pub fn player_move(&mut self, position: WorldPosition) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RequestPlayerMovePacket::new(position))
    }

    pub fn warp_to_map(&mut self, map_name: String, position: TilePosition) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RequestWarpToMapPacket::new(map_name, position))
    }

    pub fn entity_details(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RequestDetailsPacket::new(entity_id))
    }

    pub fn player_attack(&mut self, entity_id: EntityId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RequestActionPacket::new(entity_id, Action::Attack))
    }

    pub fn send_chat_message(&mut self, player_name: &str, message: &str) -> Result<(), NotConnectedError> {
        let complete_message = format!("{} : {}", player_name, message);

        self.send_map_server_packet(&GlobalMessagePacket::new(complete_message))
    }

    pub fn start_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&StartDialogPacket::new(npc_id))
    }

    pub fn next_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&NextDialogPacket::new(npc_id))
    }

    pub fn close_dialog(&mut self, npc_id: EntityId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&CloseDialogPacket::new(npc_id))
    }

    pub fn choose_dialog_option(&mut self, npc_id: EntityId, option: i8) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&ChooseDialogOptionPacket::new(npc_id, option))
    }

    pub fn request_item_equip(&mut self, item_index: InventoryIndex, equip_position: EquipPosition) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RequestEquipItemPacket::new(item_index, equip_position))
    }

    pub fn request_item_unequip(&mut self, item_index: InventoryIndex) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RequestUnequipItemPacket::new(item_index))
    }

    pub fn cast_skill(&mut self, skill_id: SkillId, skill_level: SkillLevel, entity_id: EntityId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&UseSkillAtIdPacket::new(skill_level, skill_id, entity_id))
    }

    pub fn cast_ground_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        target_position: TilePosition,
    ) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&UseSkillOnGroundPacket::new(skill_level, skill_id, target_position))
    }

    pub fn cast_channeling_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        entity_id: EntityId,
    ) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&StartUseSkillPacket::new(skill_id, skill_level, entity_id))
    }

    pub fn stop_channeling_skill(&mut self, skill_id: SkillId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&EndUseSkillPacket::new(skill_id))
    }

    pub fn add_friend(&mut self, name: String) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&AddFriendPacket::new(name))
    }

    pub fn remove_friend(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&RemoveFriendPacket::new(account_id, character_id))
    }

    pub fn reject_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&FriendRequestResponsePacket::new(
            account_id,
            character_id,
            FriendRequestResponse::Reject,
        ))
    }

    pub fn accept_friend_request(&mut self, account_id: AccountId, character_id: CharacterId) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&FriendRequestResponsePacket::new(
            account_id,
            character_id,
            FriendRequestResponse::Accept,
        ))
    }

    pub fn create_character(&mut self, slot: usize, name: String) -> Result<(), NotConnectedError> {
        let hair_color = 0;
        let hair_style = 0;
        let start_job = 0;
        let sex = Sex::Male;

        self.send_character_server_packet(&CreateCharacterPacket::new(
            name, slot as u8, hair_color, hair_style, start_job, sex,
        ))
    }

    pub fn delete_character(&mut self, character_id: CharacterId) -> Result<(), NotConnectedError> {
        let email = "a@a.com".to_string();

        self.send_character_server_packet(&DeleteCharacterPacket::new(character_id, email))
    }

    pub fn switch_character_slot(&mut self, origin_slot: usize, destination_slot: usize) -> Result<(), NotConnectedError> {
        self.send_character_server_packet(&SwitchCharacterSlotPacket::new(origin_slot as u16, destination_slot as u16))
    }

    pub fn set_hotkey_data(&mut self, tab: HotbarTab, index: HotbarSlot, hotkey_data: HotkeyData) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&SetHotkeyData2Packet::new(tab, index, hotkey_data))
    }

    pub fn select_buy_or_sell(&mut self, shop_id: ShopId, buy_or_sell: BuyOrSellOption) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&SelectBuyOrSellPacket::new(shop_id, buy_or_sell))
    }

    pub fn purchase_items(&mut self, items: Vec<ShopItem<u32>>) -> Result<(), NotConnectedError> {
        let item_information = items
            .into_iter()
            .map(|item| BuyShopItemInformation {
                item_id: item.item_id,
                amount: item.metadata,
            })
            .collect();

        self.send_map_server_packet(&BuyShopItemsPacket::new(item_information))
    }

    pub fn close_shop(&mut self) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&CloseShopPacket::new())
    }

    pub fn sell_items(&mut self, items: Vec<SoldItemInformation>) -> Result<(), NotConnectedError> {
        self.send_map_server_packet(&SellItemsPacket { items })
    }
}

#[cfg(test)]
mod packet_handlers {
    use ragnarok_packets::handler::NoPacketCallback;

    use crate::NetworkingSystem;

    #[test]
    fn login_server() {
        let result = NetworkingSystem::create_login_server_packet_handler(NoPacketCallback);
        assert!(result.is_ok());
    }

    #[test]
    fn character_server() {
        let result = NetworkingSystem::create_character_server_packet_handler(NoPacketCallback);
        assert!(result.is_ok());
    }

    #[test]
    fn map_server() {
        let result = NetworkingSystem::create_map_server_packet_handler(NoPacketCallback);
        assert!(result.is_ok());
    }
}
