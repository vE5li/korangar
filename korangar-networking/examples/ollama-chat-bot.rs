use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread::sleep;
use std::time::Duration;

use korangar_debug::logging::Colorize;
use korangar_networking::{DisconnectReason, NetworkEvent, NetworkingSystem, SupportedPacketVersion};
use reqwest::StatusCode;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(serde::Serialize)]
struct Request {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Response {
    model: String,
    created_at: String,
    message: Message,
    done: bool,
    total_duration: u64,
    load_duration: u64,
    prompt_eval_duration: u64,
    eval_count: u64,
    eval_duration: u64,
}

struct MessageHistory {
    hash_map: HashMap<String, Vec<Message>>,
}

impl MessageHistory {
    fn get_message_history_with(&mut self, user: String) -> &mut Vec<Message> {
        self.hash_map.entry(user).or_insert_with(|| {
            vec![
                Message {
                    role: "user".to_owned(),
                    content: "You are in a video game, are only allowed to reply in very broken english, your responses must be less that \
                              40 chars. Your name is Joe."
                        .to_owned(),
                },
                // Llama will not be able to understand the history of the conversation if "user"
                // and "assistant" roles don't alternate, so we insert this dummy response.
                Message {
                    role: "assistant".to_owned(),
                    content: "Okey.".to_owned(),
                },
            ]
        })
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Chat bot settings.
    const PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;
    const SOCKET_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(49, 12, 109, 207)), 6900);
    const OLLAMA_ENDPOINT: &str = "http://127.0.0.1:11434/api/chat";
    const OLLAMA_MODEL: &str = "llama2:13b";
    const USERNAME: &str = "username";
    const PASSWORD: &str = "password";
    const CHARACTER_NAME: &str = "character name";

    // Create the networking system and HTTP client.
    let (mut networking_system, mut network_event_buffer) = NetworkingSystem::spawn();
    let client = reqwest::Client::new();

    // Persistent data.
    let mut saved_login_data = None;
    let mut message_history = MessageHistory { hash_map: HashMap::new() };

    // Kick of the bot by connecting to the login server.
    networking_system.connect_to_login_server(PACKET_VERSION, SOCKET_ADDR, USERNAME.to_owned(), PASSWORD.to_owned());

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
                    panic!("Failed to connect to login server: {}", message);
                }
                NetworkEvent::LoginServerDisconnected {
                    reason: DisconnectReason::ConnectionError,
                } => {
                    panic!("Login server connection error");
                }
                NetworkEvent::CharacterServerConnected { .. } => {
                    println!("[{}] Successfully connected to character server", "Setup".green());

                    networking_system.request_character_list().expect("Character server disconnected");
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    panic!("Failed to connect to character server: {}", message);
                }
                NetworkEvent::CharacterServerDisconnected {
                    reason: DisconnectReason::ConnectionError,
                } => {
                    panic!("Character server connection error");
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => {
                    panic!("Failed to select character: {}", message);
                }
                NetworkEvent::MapServerDisconnected {
                    reason: DisconnectReason::ConnectionError,
                } => {
                    panic!("Map server connection error");
                }
                NetworkEvent::CharacterList { characters } => {
                    let character_slot = characters
                        .iter()
                        .find(|character| character.name == CHARACTER_NAME)
                        .unwrap_or_else(|| panic!("Character with name \"{}\" not found for this user", CHARACTER_NAME))
                        .character_number as usize;

                    println!("[{}] Using character in slot: {}", "Setup".green(), character_slot.green());

                    networking_system
                        .select_character(character_slot)
                        .expect("Character server disconnected");
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    let login_login_data = saved_login_data.as_ref().unwrap();

                    networking_system.disconnect_from_character_server();
                    networking_system.connect_to_map_server(PACKET_VERSION, login_login_data, login_data);

                    networking_system.map_loaded().expect("Map server disconnected");
                }
                NetworkEvent::ChatMessage { text, .. } => {
                    if text.starts_with(CHARACTER_NAME) {
                        continue;
                    }

                    let Some((user, message)) = text.split_once(" : ") else {
                        continue;
                    };

                    println!("[{}] Received Message by user: {}", "Chatbot".cyan(), user.yellow());
                    println!("[{}] Message content: {}", "Chatbot".cyan(), message.yellow());
                    println!("[{}] Generating response..", "LLaMA".magenta());

                    let previous_messages = message_history.get_message_history_with(user.to_owned());

                    previous_messages.push(Message {
                        role: "user".to_owned(),
                        content: message.to_owned(),
                    });

                    let result = client
                        .post(OLLAMA_ENDPOINT)
                        .json(&Request {
                            model: OLLAMA_MODEL.to_owned(),
                            messages: previous_messages.clone(),
                            stream: false,
                        })
                        .send()
                        .await
                        .expect("failed to send request to ollama");

                    if result.status() == StatusCode::OK {
                        let response: Response = result.json().await.unwrap();
                        let response = &response.message;

                        println!("[{}] Generated response: {}", "LLaMA".magenta(), response.content.yellow());
                        println!("[{}] Sending response..", "Chatbot".cyan());

                        previous_messages.push(Message {
                            role: response.role.to_owned(),
                            content: response.content.to_owned(),
                        });

                        networking_system
                            .send_chat_message(CHARACTER_NAME, &response.content)
                            .expect("Map server disconnected");
                    }
                }
                _ => {}
            }
        }

        // After processing events, sleep for a bit.
        sleep(Duration::from_millis(200));
    }
}
