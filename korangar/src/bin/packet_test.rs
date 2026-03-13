use std::sync::atomic::Ordering;
use std::time::Duration;

use korangar::input::InputEvent;
use korangar::loaders::{ClientInfoPathExt, Service, ServiceId};
use korangar::state::{ClientState, ClientStatePathExt, client_state};
use korangar::{Client, SHUTDOWN_SIGNAL, init_tls_rand, time_phase};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
use korangar_networking::NetworkEvent;
use ragnarok_packets::{CharacterId, CharacterServerInformation, ServerAddress};
use rust_state::{Path, State};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

mod testing {
    use std::time::Duration;

    use korangar::ClientHooks;
    use korangar::input::InputEvent;
    use korangar::state::ClientState;
    use korangar_networking::NetworkEvent;
    use rust_state::{Path, State};

    pub struct Timer {}

    impl Timer {
        fn reset(&mut self) {}

        fn elapsed_time(&self) -> Duration {
            Duration::from_millis(10)
        }
    }

    pub enum StepFlow {
        Continue,
        Success,
        Failure,
    }

    pub trait WorkStep {
        fn inject_input_event(&self, input_events: &mut Vec<InputEvent>) -> StepFlow {
            StepFlow::Continue
        }

        fn inspect_network_event(&self, network_event: &NetworkEvent, timer: &Timer) -> StepFlow {
            StepFlow::Continue
        }

        fn inspect_state(&self, state: &mut State<ClientState>, timer: &Timer) -> StepFlow {
            StepFlow::Continue
        }
    }

    pub fn modify_state<T>(path: impl Path<ClientState, T>, value: T) -> Box<dyn WorkStep>
    where
        T: Clone + 'static,
    {
        struct Inner<T, P> {
            path: P,
            value: T,
        }

        impl<T, P> WorkStep for Inner<T, P>
        where
            T: Clone,
            P: Path<ClientState, T>,
        {
            fn inspect_state(&self, state: &mut State<ClientState>, timer: &Timer) -> StepFlow {
                *state.follow_mut(self.path) = self.value.clone();
                StepFlow::Success
            }
        }

        Box::new(Inner { path, value })
    }

    pub fn inject_input(input_event: InputEvent) -> Box<dyn WorkStep> {
        struct Inner(InputEvent);

        impl WorkStep for Inner {
            fn inject_input_event(&self, input_events: &mut Vec<InputEvent>) -> StepFlow {
                input_events.push(self.0.clone());
                StepFlow::Success
            }
        }

        Box::new(Inner(input_event))
    }

    // fn wait_for_network_event(network_event: NetworkEvent) -> Box<dyn WorkStep> {
    //     todo!()
    // }
    //
    // fn wait_for_network_event_or_failure(network_event: NetworkEvent,
    // failure_network_event: NetworkEvent) -> Box<dyn WorkStep> {     todo!()
    // }

    pub fn wait_for_network_event_with(f: impl Fn(&NetworkEvent) -> bool + 'static) -> Box<dyn WorkStep> {
        struct Inner<F> {
            f: F,
        }

        impl<F> WorkStep for Inner<F>
        where
            F: Fn(&NetworkEvent) -> bool,
        {
            fn inspect_network_event(&self, network_event: &NetworkEvent, timer: &Timer) -> StepFlow {
                if (self.f)(&network_event) {
                    StepFlow::Success
                } else if timer.elapsed_time() > Duration::from_millis(500) {
                    StepFlow::Failure
                } else {
                    StepFlow::Continue
                }
            }
        }

        Box::new(Inner { f })
    }

    pub fn wait_for_network_event_or_failure_with(
        f: impl Fn(&NetworkEvent) -> bool + 'static,
        e: impl Fn(&NetworkEvent) -> bool + 'static,
    ) -> Box<dyn WorkStep> {
        struct Inner<F, E> {
            f: F,
            e: E,
        }

        impl<F, E> WorkStep for Inner<F, E>
        where
            F: Fn(&NetworkEvent) -> bool,
            E: Fn(&NetworkEvent) -> bool,
        {
            fn inspect_network_event(&self, network_event: &NetworkEvent, timer: &Timer) -> StepFlow {
                println!("Comparing network events: {:?}", network_event);

                if (self.f)(&network_event) {
                    StepFlow::Success
                } else if (self.e)(&network_event) || timer.elapsed_time() > Duration::from_millis(500) {
                    StepFlow::Failure
                } else {
                    StepFlow::Continue
                }
            }
        }

        Box::new(Inner { f, e })
    }

    pub struct TestManager {
        steps: Vec<Box<dyn WorkStep>>,
        current_step: usize,
        step_timer: Timer,
        error_message: Option<String>,
    }

    impl TestManager {
        pub fn new(steps: Vec<Box<dyn WorkStep>>) -> Self {
            Self {
                steps,
                current_step: 0,
                step_timer: Timer {},
                error_message: None,
            }
        }

        fn do_for_current_step(&mut self, f: impl FnOnce(&dyn WorkStep, &Timer) -> StepFlow) {
            let Some(current_step) = self.steps.get(self.current_step) else {
                return;
            };

            match f(&**current_step, &self.step_timer) {
                StepFlow::Continue => {}
                StepFlow::Success => {
                    self.current_step += 1;
                    self.step_timer.reset();
                    println!("Advancing to the next step: {}", self.current_step);
                }
                StepFlow::Failure => {
                    self.error_message = Some("tests failed".to_owned());
                    println!(">>>>>>>> Test failed");
                    // TODO: Should terminate another way.
                    self.current_step += 1;
                }
            }
        }
    }

    impl ClientHooks for TestManager {
        fn inject_input_event(&mut self, input_events: &mut Vec<InputEvent>) {
            self.do_for_current_step(|step, _| step.inject_input_event(input_events));
        }

        fn inspect_network_event(&mut self, network_event: &NetworkEvent) {
            self.do_for_current_step(|step, timer| step.inspect_network_event(network_event, timer));
        }

        fn inspect_state(&mut self, state: &mut State<ClientState>) {
            self.do_for_current_step(|step, timer| step.inspect_state(state, timer));
        }
    }
}

fn main() {
    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = threads::Main::start_frame();

    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .start_handler(|_| init_tls_rand())
        .build_global()
        .unwrap();

    init_tls_rand();

    // Check if korangar is in the correct working directory and if not, try to
    // correct it.
    // NOTE: This check might be temporary or feature gated in the future.
    time_phase!("adjust working directory", {
        if !std::fs::metadata("archive").is_ok_and(|metadata| metadata.is_dir()) {
            #[cfg(feature = "debug")]
            print_debug!(
                "[{}] failed to find archive directory, attempting to change working directory {}",
                "warning".yellow(),
                "korangar".magenta()
            );

            if let Err(_error) = std::env::set_current_dir("korangar") {
                #[cfg(feature = "debug")]
                print_debug!("[{}] failed to change working directory: {:?}", "error".red(), _error);
            }
        }
    });

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

    let test_manager = testing::TestManager::new(vec![
        // Log in to login server.
        testing::modify_state(client_state().client_info().services(), vec![service]),
        testing::inject_input(InputEvent::LogIn {
            service_id,
            username,
            password,
        }),
        testing::wait_for_network_event_or_failure_with(
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
        testing::inject_input(InputEvent::SelectServer {
            character_server_information,
        }),
        testing::wait_for_network_event_or_failure_with(
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
        testing::inject_input(InputEvent::CreateCharacter {
            slot: 0,
            name: "testing1".to_owned(),
        }),
        testing::wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterCreated { .. }),
            |network_event| matches!(network_event, NetworkEvent::CharacterCreationFailed { .. }),
        ),
        testing::inject_input(InputEvent::CreateCharacter {
            slot: 1,
            name: "testing2".to_owned(),
        }),
        testing::wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterCreated { .. }),
            |network_event| matches!(network_event, NetworkEvent::CharacterCreationFailed { .. }),
        ),
        // Swap character slots.
        testing::inject_input(InputEvent::SwitchCharacterSlot {
            origin_slot: 0,
            destination_slot: 1,
        }),
        testing::wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterSlotSwitched { .. }),
            |network_event| matches!(network_event, NetworkEvent::CharacterSlotSwitchFailed),
        ),
        // Delete character.
        testing::inject_input(InputEvent::DeleteCharacter {
            character_id: CharacterId(150001),
        }),
        testing::wait_for_network_event_or_failure_with(
            |network_event| matches!(network_event, NetworkEvent::CharacterDeleted),
            |network_event| matches!(network_event, NetworkEvent::CharacterDeletionFailed { .. }),
        ),
        // Connect to map server.
        testing::inject_input(InputEvent::SelectCharacter { slot: 1 }),
        testing::wait_for_network_event_or_failure_with(
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
