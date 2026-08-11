use korangar::Client;
use korangar::input::InputEvent;
use korangar::loaders::{ClientInfoPathExt, Service};
use korangar::state::{ClientStatePathExt, client_state};
use korangar_networking::NetworkEvent;
use korangar_testing::{TestManager, inject_input, modify_state, wait_for_network_event_or_failure_with};
use ragnarok_packets::{CharacterId, CharacterServerInformation, ServerAddress};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let service = Service {
        display_name: Default::default(),
        description: Default::default(),
        balloon: Default::default(),
        address: "127.0.0.1".to_owned(),
        port: 6900,
        // TODO: Might need to be adjusted to connect.
        version: Default::default(),
        language_type: Default::default(),
        registration_web: Default::default(),
        game_master_yellow_ids: Default::default(),
        game_master_accounts: Default::default(),
        loading_images: Default::default(),
        packet_version: Default::default(),
    };
    let service_id = service.service_id();
    let username = "testing_m".to_owned();
    // let username = "testing".to_owned();
    let password = "password".to_owned();

    let character_server_information = CharacterServerInformation {
        server_ip: ServerAddress([127, 0, 0, 1]),
        server_port: 6121,
        server_name: "rAthena".to_owned(),
        user_count: Default::default(),
        server_type: Default::default(),
        display_new: Default::default(),
        unknown: [0; 128],
    };

    let test_manager = TestManager::new(vec![
        // Log in to login server.
        modify_state(client_state().client_info().services(), vec![service]),
        inject_input(InputEvent::LogIn {
            service_id,
            username,
            password,
        }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::LoginServerConnected { .. }),
            |network_event| {
                matches!(
                    network_event,
                    // `LoginServerDisconnected` means that the test server might not be running.
                    NetworkEvent::LoginServerDisconnected { .. } | NetworkEvent::LoginServerConnectionFailed { .. }
                )
            },
        ),
        // Log in to character server.
        inject_input(InputEvent::SelectServer {
            character_server_information,
        }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterServerConnected { .. }),
            |network_event| {
                matches!(
                    network_event,
                    // `CharacterServerDisconnected` means that the test server might not be running.
                    NetworkEvent::CharacterServerConnectionFailed { .. } | NetworkEvent::CharacterServerDisconnected { .. }
                )
            },
        ),
        // Create test characters.
        inject_input(InputEvent::CreateCharacter {
            slot: 0,
            name: "testing1".to_owned(),
        }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterCreated { .. }),
            |network_event| matches!(network_event, NetworkEvent::CharacterCreationFailed { .. }),
        ),
        inject_input(InputEvent::CreateCharacter {
            slot: 1,
            name: "testing2".to_owned(),
        }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterCreated { .. }),
            |network_event| matches!(network_event, NetworkEvent::CharacterCreationFailed { .. }),
        ),
        // Swap character slots.
        inject_input(InputEvent::SwitchCharacterSlot {
            origin_slot: 0,
            destination_slot: 1,
        }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterSlotSwitched { .. }),
            |network_event| matches!(network_event, NetworkEvent::CharacterSlotSwitchFailed),
        ),
        // Delete character.
        inject_input(InputEvent::DeleteCharacter {
            character_id: CharacterId(150001),
        }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterDeleted),
            |network_event| matches!(network_event, NetworkEvent::CharacterDeletionFailed { .. }),
        ),
        // Connect to map server.
        inject_input(InputEvent::SelectCharacter { slot: 1 }),
        wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterSelected { .. }),
            |network_event| {
                matches!(
                    network_event,
                    // `MapServerDisconnected` means that the test server might not be running.
                    NetworkEvent::CharacterSelectionFailed { .. } | NetworkEvent::MapServerDisconnected { .. }
                )
            },
        ),
    ]);

    let mut client = Client::init(false, test_manager).unwrap();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run_app(&mut client);
}
