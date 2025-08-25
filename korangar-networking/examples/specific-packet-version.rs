#![cfg_attr(feature = "interface", feature(negative_impls))]
#![cfg_attr(feature = "interface", feature(impl_trait_in_assoc_type))]

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::thread::{JoinHandle, spawn};
use std::time::Instant;

use korangar_debug::logging::Colorize;
use korangar_networking::event::NetworkEventList;
use korangar_networking::packet_version::{
    CharPacketHandlerRegister, LoginPacketHandlerRegister, MapPacketHandlerRegister, MapPacketHandlerState,
};
use korangar_networking::{
    CharPacketFactory, CharacterServerLoginData, LoginPacketFactory, LoginServerLoginData, MapPacketFactory, NetworkEvent, NetworkingSystem,
};
use ragnarok_bytes::{ByteReader, ByteWriter, FromBytes, ToBytes};
use ragnarok_packets::{
    AccountId, CharacterId, CharacterInformation, CharacterServer, CharacterServerInformation, CharacterServerLoginPacket,
    CharacterServerPacket, ClientPacket, ClientTick, Direction, LoginServer, LoginServerLoginPacket, LoginServerPacket, MapServer,
    MapServerPacket, Packet, PacketExt, ServerAddress, ServerPacket, Sex, WorldPosition,
};

/// Example of how korangar can support a specific packet version, via an
/// external crate.

/// Old packets version
#[derive(Debug, Clone, Packet, ServerPacket, LoginServer)]
#[header(0x0069)]
#[variable_length]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
struct LoginServerLoginSuccessPacketOld {
    pub auth_code: i32,
    pub aid: u32,
    pub user_level: u32,
    pub last_login_ip: u32,
    pub sex: Sex,
    pub server_list: Vec<CharacterServerInformation>,
}

#[derive(Debug, Clone, Packet, ServerPacket, CharacterServer)]
#[header(0x006B)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct CharacterServerLoginSuccessPacketOld {
    pub total_slot_num: u8,
    pub premium_start_slot: u8,
    pub code: u8,
    pub time1: u8,
    pub time2: u8,
    pub char_info: Vec<CharacterInformation>,
}

#[derive(Debug, Clone, Packet, ClientPacket, MapServer)]
#[header(0x0072)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MapServerLoginPacketOld {
    pub account_id: AccountId,
    pub character_id: CharacterId,
    pub login_id1: u32,
    pub client_tick: ClientTick,
    pub sex: Sex,
}

#[derive(Debug, Clone, Packet, ServerPacket, MapServer)]
#[header(0x0073)]
#[cfg_attr(feature = "interface", derive(rust_state::RustState, korangar_interface::element::StateElement))]
pub struct MapServerEnterOld {
    pub client_tick: ClientTick,
    pub position: WorldPosition,
    #[new_default]
    pub ignored: [u8; 2],
}

#[derive(Copy, Clone)]
pub struct CustomLoginPacketFactory;
#[derive(Copy, Clone)]
pub struct CustomCharPacketFactory;
#[derive(Copy, Clone)]
pub struct CustomMapPacketFactory;

#[derive(Copy, Clone)]
pub struct CustomLoginPacketReceiver;
#[derive(Copy, Clone)]
pub struct CustomCharPacketReceiver;
#[derive(Copy, Clone)]
pub struct CustomMapPacketReceiver;

/// Registering packet handlers for old packets sent by server
impl LoginPacketHandlerRegister for CustomLoginPacketReceiver {
    fn register_version_specific_handler<Output, Meta, Callback>(
        &self,
        packet_handler: &mut ragnarok_packets::handler::PacketHandler<Output, Meta, Callback>,
    ) where
        Output: From<NetworkEventList> + Default,
        Meta: Default,
        Callback: ragnarok_packets::handler::PacketCallback,
    {
        packet_handler
            .register(|custom_login_response: LoginServerLoginSuccessPacketOld| {
                println!("Login response received: account_id={}", custom_login_response.aid,);
                NetworkEventList::from(NetworkEvent::LoginServerConnected {
                    character_servers: custom_login_response.server_list,
                    login_data: LoginServerLoginData {
                        account_id: AccountId(custom_login_response.aid),
                        login_id1: custom_login_response.auth_code as u32,
                        login_id2: custom_login_response.user_level,
                        sex: custom_login_response.sex,
                    },
                })
            })
            .unwrap();
    }
}

impl CharPacketHandlerRegister for CustomCharPacketReceiver {
    fn register_version_specific_handler<Output, Meta, Callback>(
        &self,
        packet_handler: &mut ragnarok_packets::handler::PacketHandler<Output, Meta, Callback>,
    ) where
        Output: From<NetworkEventList> + Default,
        Meta: Default,
        Callback: ragnarok_packets::handler::PacketCallback,
    {
        packet_handler
            .register(|packet: CharacterServerLoginSuccessPacketOld| {
                NetworkEventList::from(vec![
                    NetworkEvent::CharacterServerConnected {
                        normal_slot_count: packet.total_slot_num as usize,
                    },
                    NetworkEvent::CharacterList {
                        characters: packet.char_info,
                    },
                ])
            })
            .unwrap();
    }
}

impl MapPacketHandlerRegister for CustomMapPacketReceiver {
    fn register_version_specific_handler<Output, Meta, Callback>(
        &self,
        packet_handler: &mut ragnarok_packets::handler::PacketHandler<Output, Meta, Callback>,
        _state: &MapPacketHandlerState,
    ) where
        Output: From<NetworkEventList> + Default,
        Meta: Default,
        Callback: ragnarok_packets::handler::PacketCallback,
    {
        packet_handler
            .register(|packet: MapServerEnterOld| {
                NetworkEventList::from(NetworkEvent::UpdateClientTick {
                    client_tick: packet.client_tick,
                    received_at: Instant::now(),
                })
            })
            .unwrap();
    }
}

/// Define packets to be sent for this specific version
impl LoginPacketFactory for CustomLoginPacketFactory {
    fn login_server_login(&self, username: impl Into<String>, password: impl Into<String>) -> impl LoginServerPacket {
        LoginServerLoginPacket::new(username.into(), password.into())
    }
}
impl CharPacketFactory for CustomCharPacketFactory {
    fn char_server_login(&self, login_data: &LoginServerLoginData) -> impl CharacterServerPacket {
        CharacterServerLoginPacket::new(
            login_data.account_id,
            login_data.login_id1,
            login_data.login_id2,
            login_data.sex,
        )
    }
}
impl MapPacketFactory for CustomMapPacketFactory {
    fn map_server_login(
        &self,
        account_id: AccountId,
        character_id: CharacterId,
        login_id1: u32,
        client_tick: ClientTick,
        sex: Sex,
    ) -> impl MapServerPacket {
        MapServerLoginPacketOld {
            account_id,
            character_id,
            login_id1,
            client_tick,
            sex,
        }
    }
}

fn main() {
    let login_server = Server {
        name: "login".to_string(),
        local_port: 8610,
        specific_proxy: LoginServer {},
        is_alive: Arc::new(AtomicBool::new(true)),
    };
    let char_server = Server {
        name: "char".to_string(),
        local_port: 8611,
        specific_proxy: CharServer {},
        is_alive: Arc::new(AtomicBool::new(true)),
    };
    let map_server = Server {
        name: "map".to_string(),
        local_port: 8612,
        specific_proxy: MapServer {},
        is_alive: Arc::new(AtomicBool::new(true)),
    };
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    handles.push(login_server.start());
    handles.push(char_server.start());
    handles.push(map_server.start());

    let (mut networking_system, mut network_event_buffer) = NetworkingSystem::spawn(
        CustomLoginPacketFactory,
        CustomCharPacketFactory,
        CustomMapPacketFactory,
        CustomLoginPacketReceiver,
        CustomCharPacketReceiver,
        CustomMapPacketReceiver,
    );

    networking_system.connect_to_login_server(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), login_server.local_port),
        "username",
        "password",
    );
    let mut saved_login_data = None;

    loop {
        networking_system.get_events(&mut network_event_buffer);

        for event in network_event_buffer.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    println!("[{}] Successfully connected to login server", "Setup".green());

                    networking_system.disconnect_from_login_server();
                    networking_system.connect_to_character_server(&login_data, character_servers[0].clone());

                    saved_login_data = Some(login_data);
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    println!(
                        "[{}] Successfully connected to character server, slots {}",
                        "Setup".green(),
                        normal_slot_count
                    );
                }
                NetworkEvent::CharacterList { characters } => {
                    println!("Received character list: {}", characters.len());
                    let data = CharacterServerLoginData {
                        server_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                        server_port: map_server.local_port,
                        character_id: CharacterId(150000),
                    };
                    networking_system.connect_to_map_server(saved_login_data.as_ref().unwrap(), data);
                }
                NetworkEvent::UpdateClientTick { client_tick, .. } => {
                    println!(
                        "[{}] Successfully connected to map server, tick {:?}",
                        "Setup".green(),
                        client_tick
                    );
                }
                _ => {}
            }
        }
    }
}

// Simulate server

#[derive(Clone)]
pub struct Server<T: PacketHandler> {
    pub name: String,
    pub local_port: u16,
    pub specific_proxy: T,
    pub is_alive: Arc<AtomicBool>,
}

pub trait PacketHandler {
    fn handle_packet(&self, tcp_stream: Arc<RwLock<TcpStream>>, packet: &[u8]);
}

impl<T: 'static + PacketHandler + Clone + Send + Sync> Server<T> {
    pub fn start(&self) -> JoinHandle<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", self.local_port)).unwrap();
        let immutable_self_ref = Arc::new(self.clone());
        let server_ref = immutable_self_ref.clone();
        spawn(move || {
            println!("Start server {}, on port {}", server_ref.name, server_ref.local_port);
            for tcp_stream in listener.incoming() {
                if !server_ref.is_alive.load(Ordering::Relaxed) {
                    break;
                }
                let server_ref = immutable_self_ref.clone();
                // Receive new connection, starting new thread
                spawn(move || {
                    server_ref.listen(tcp_stream.unwrap());
                });
            }
            println!("shutdown server {}", server_ref.name);
        })
    }

    pub fn shutdown(&self) {
        self.is_alive.store(false, SeqCst);
        TcpStream::connect(format!("127.0.0.1:{}", self.local_port))
            .map(|mut stream| stream.flush())
            .ok();
    }

    fn listen(&self, mut incoming_stream: TcpStream) {
        let mut buffer = [0; 2048];
        let tcp_stream_arc = Arc::new(RwLock::new(incoming_stream.try_clone().unwrap()));
        loop {
            if !self.is_alive.load(Ordering::Relaxed) {
                let _ = incoming_stream.shutdown(Shutdown::Both);
                break;
            }
            match incoming_stream.read(&mut buffer) {
                Ok(bytes_read) => {
                    self.specific_proxy.handle_packet(tcp_stream_arc.clone(), &buffer[..bytes_read]);
                }
                Err(_) => {}
            }
        }
    }
}

#[derive(Clone)]
struct LoginServer {}
#[derive(Clone)]
struct CharServer {}
#[derive(Clone)]
struct MapServer {}

impl PacketHandler for LoginServer {
    fn handle_packet(&self, tcp_stream: Arc<RwLock<TcpStream>>, packet: &[u8]) {
        if packet.is_empty() {
            return;
        }
        println!("LoginServer received packet: {:02X?}", packet);

        let mut byte_reader = ByteReader::without_metadata(packet);
        match LoginServerLoginPacket::packet_from_bytes(&mut byte_reader) {
            Ok(_login_packet) => {
                let response = LoginServerLoginSuccessPacketOld {
                    auth_code: 1234,
                    aid: 2000000,
                    user_level: 90,
                    last_login_ip: 0,
                    sex: Sex::Male,
                    server_list: vec![CharacterServerInformation {
                        server_ip: ServerAddress([127, 0, 0, 1]),
                        server_port: 8611,
                        server_name: "Korangar".to_string(),
                        user_count: 0,
                        server_type: 0,
                        display_new: 0,
                        unknown: [0; 128],
                    }],
                };

                let mut byte_writer = ByteWriter::default();
                if let Ok(_) = response.packet_to_bytes(&mut byte_writer) {
                    if let Ok(mut stream) = tcp_stream.write() {
                        let _ = stream.write_all(byte_writer.as_slice()).unwrap();
                    }
                }
            }
            Err(e) => println!("Failed to parse login packet: {:?}", e),
        }
    }
}

impl PacketHandler for CharServer {
    fn handle_packet(&self, tcp_stream: Arc<RwLock<TcpStream>>, packet: &[u8]) {
        if packet.is_empty() {
            return;
        }
        println!("CharServer received packet: {:02X?}", packet);

        let mut byte_reader = ByteReader::without_metadata(packet);
        match CharacterServerLoginPacket::packet_from_bytes(&mut byte_reader) {
            Ok(_char_packet) => {
                let response = CharacterServerLoginSuccessPacketOld {
                    total_slot_num: 12,
                    premium_start_slot: 0,
                    code: 12,
                    time1: 1,
                    time2: 2,
                    char_info: vec![CharacterInformation {
                        character_id: CharacterId(150000),
                        experience: 0,
                        money: 0,
                        job_experience: 0,
                        job_level: 0,
                        body_state: 0,
                        health_state: 0,
                        effect_state: 0,
                        virtue: 0,
                        honor: 0,
                        job_points: 0,
                        health_points: 0,
                        maximum_health_points: 0,
                        spell_points: 0,
                        maximum_spell_points: 0,
                        movement_speed: 0,
                        job: 0,
                        head: 0,
                        body: 0,
                        weapon: 0,
                        base_level: 0,
                        sp_point: 0,
                        accessory: 0,
                        shield: 0,
                        accessory2: 0,
                        accessory3: 0,
                        head_palette: 0,
                        body_palette: 0,
                        name: "".to_string(),
                        strength: 0,
                        agility: 0,
                        vit: 0,
                        intelligence: 0,
                        dexterity: 0,
                        luck: 0,
                        character_number: 0,
                        hair_color: 0,
                        b_is_changed_char: 0,
                        map_name: "".to_string(),
                        deletion_reverse_date: 0,
                        robe_palette: 0,
                        character_slot_change_count: 0,
                        character_name_change_count: 0,
                        sex: Sex::Male,
                    }],
                };
                let mut byte_writer = ByteWriter::default();
                AccountId(200000).to_bytes(&mut byte_writer).unwrap();
                if let Ok(_) = response.packet_to_bytes(&mut byte_writer) {
                    if let Ok(mut stream) = tcp_stream.write() {
                        let _ = stream.write_all(byte_writer.as_slice());
                    }
                }
            }
            Err(e) => println!("Failed to parse char packet: {:?}", e),
        }
    }
}

impl PacketHandler for MapServer {
    fn handle_packet(&self, tcp_stream: Arc<RwLock<TcpStream>>, packet: &[u8]) {
        println!("MapServer received packet: {:02X?}", packet);

        let mut byte_reader = ByteReader::without_metadata(packet);
        match MapServerLoginPacketOld::packet_from_bytes(&mut byte_reader) {
            Ok(_map_packet) => {
                // Send response using proper response packet
                let response = MapServerEnterOld {
                    client_tick: ClientTick(100),
                    position: WorldPosition {
                        x: 156,
                        y: 112,
                        direction: Direction::N,
                    },
                    ignored: [0; 2],
                };

                let mut byte_writer = ByteWriter::default();
                if let Ok(_) = response.packet_to_bytes(&mut byte_writer) {
                    if let Ok(mut stream) = tcp_stream.write() {
                        let _ = stream.write_all(byte_writer.as_slice());
                    }
                }
            }
            Err(e) => println!("Failed to parse map packet: {:?}", e),
        }
    }
}
