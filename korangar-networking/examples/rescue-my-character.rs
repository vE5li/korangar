use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::process::ExitCode;
use std::thread::sleep;
use std::time::Duration;

use clap::Parser;
use korangar_debug::logging::Colorize;
use korangar_networking::{DisconnectReason, NetworkEvent, NetworkingSystem, SupportedPacketVersion};
use ragnarok_packets::TilePosition;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Username of the account with the character to rescue.
    #[arg(short, long)]
    username: String,

    /// Password of the account with the character to rescue.
    #[arg(short, long)]
    password: String,

    /// Name of the character to rescue.
    #[arg(short, long)]
    character: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    const PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;
    const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(49, 12, 109, 207)), 6900);
    const SAFE_MAP: &str = "geffen";
    const SAFE_POSITION: TilePosition = TilePosition { x: 119, y: 59 };

    let arguments = Arguments::parse();

    // Create the networking system.
    let (mut networking_system, mut network_event_buffer) = NetworkingSystem::spawn();

    // Persistent data.
    let mut saved_login_data = None;

    // Kick of the login flow by connecting to the login server.
    networking_system.connect_to_login_server(PACKET_VERSION, SERVER_ADDR, arguments.username, arguments.password);

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
                    networking_system.connect_to_character_server(PACKET_VERSION, &login_data, character_servers[0].clone());

                    saved_login_data = Some(login_data);
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    println!("[{}] Failed to connect to login server: {}", "Error".red(), message);
                    return ExitCode::FAILURE;
                }
                NetworkEvent::LoginServerDisconnected {
                    reason: DisconnectReason::ConnectionError,
                } => {
                    println!("[{}] Login server connection error", "Error".red());
                    return ExitCode::FAILURE;
                }
                NetworkEvent::CharacterServerConnected { .. } => {
                    println!("[{}] Successfully connected to character server", "Setup".green());

                    networking_system.request_character_list().expect("Character server disconnected");
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    println!("[{}] Failed to connect to character server: {}", "Error".red(), message);
                    return ExitCode::FAILURE;
                }
                NetworkEvent::CharacterServerDisconnected {
                    reason: DisconnectReason::ConnectionError,
                } => {
                    println!("[{}] Character server connection error", "Error".red());
                    return ExitCode::FAILURE;
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => {
                    println!("[{}] Failed to select character: {}", "Error".red(), message);
                    return ExitCode::FAILURE;
                }
                NetworkEvent::MapServerDisconnected {
                    reason: DisconnectReason::ConnectionError,
                } => {
                    println!("[{}] Map server connection error", "Error".red());
                    return ExitCode::FAILURE;
                }
                NetworkEvent::CharacterList { characters } => {
                    let Some(character_slot) = characters.iter().find(|character| character.name == arguments.character) else {
                        println!(
                            "[{}] Character with name \"{}\" not found for this user",
                            "Error".red(),
                            arguments.character.magenta()
                        );
                        return ExitCode::FAILURE;
                    };

                    println!(
                        "[{}] Using character in slot: {}",
                        "Setup".green(),
                        character_slot.character_number.green()
                    );

                    networking_system
                        .select_character(character_slot.character_number as usize)
                        .expect("Character server disconnected");
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    let login_login_data = saved_login_data.as_ref().unwrap();

                    networking_system.disconnect_from_character_server();
                    networking_system.connect_to_map_server(PACKET_VERSION, login_login_data, login_data);

                    networking_system.map_loaded().expect("Map server disconnected");

                    networking_system
                        .warp_to_map(SAFE_MAP.to_owned(), SAFE_POSITION)
                        .expect("Map server disconnected");
                }
                NetworkEvent::ChangeMap { .. } => {
                    println!("[{}] Successfully rescued character", "Success".green());
                    return ExitCode::SUCCESS;
                }
                _ => {}
            }
        }

        // After processing events, sleep for a bit.
        sleep(Duration::from_millis(200));
    }
}
