#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(random)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]
#![feature(associated_type_defaults)]
#![feature(macro_metavar_expr)]
#![feature(unsafe_cell_access)]
#![feature(impl_trait_in_assoc_type)]
#![feature(thread_local)]

// Helper macro to time and print the startup time of Korangar
macro_rules! time_phase {
    ($message:expr, { $($statements:tt)* }) => {
        #[cfg(feature = "debug")]
        let _statement_timer = korangar_debug::logging::Timer::new($message);

        $($statements)*

        #[cfg(feature = "debug")]
        _statement_timer.stop();
    }
}

mod graphics;
mod input;
mod state;
#[macro_use]
mod interface;
mod inventory;
mod loaders;
#[cfg(feature = "debug")]
mod networking;
mod renderer;
mod settings;
mod system;
mod world;

use std::io::Cursor;
use std::net::{SocketAddr, ToSocketAddrs};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use cgmath::{Point3, Vector3};
use image::{EncodableLayout, ImageFormat, ImageReader};
use input::{MouseInputMode, MouseModeExt};
use inventory::{HotbarPathExt, InventoryPathExt, SkillTreePathExt};
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_interface::Interface;
use korangar_interface::layout::MouseButton;
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SellItem,
    SupportedPacketVersion,
};
#[cfg(feature = "debug")]
use networking::{PacketHistory, PacketHistoryCallback};
#[cfg(not(feature = "debug"))]
use ragnarok_packets::handler::NoPacketCallback;
use ragnarok_packets::{
    BuyShopItemsResult, CharacterServerInformation, Direction, DisappearanceReason, HotbarSlot, SellItemsResult, SkillId, SkillType,
    TilePosition, UnitId, WorldPosition,
};
use renderer::InterfaceRenderer;
use rust_state::{Context, ManuallyAssertExt};
#[cfg(feature = "debug")]
use rust_state::{VecIndexExt, VecLookupExt};
use settings::{
    AudioSettings, AudioSettingsPathExt, GraphicsSettingsCapabilities, GraphicsSettingsPathExt, InterfaceSettings, InterfaceSettingsPathExt,
};
use state::localization::Localization;
use state::theme::{CursorThemePathExt, IndicatorThemePathExt, InterfaceThemePathExt, WorldThemePathExt};
use state::{ChatMessage, ClientState, ClientStatePathExt, ClientStateRootExt, client_state, this_entity, this_player};
#[cfg(feature = "debug")]
use wgpu::Device;
use wgpu::util::initialize_adapter_from_env_or_default;
use wgpu::wgt::{Dx12SwapchainKind, Dx12UseFrameLatencyWaitableObject};
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, Dx12BackendOptions, Dx12Compiler, ExperimentalFeatures, GlBackendOptions, GlFenceBehavior,
    Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, MemoryBudgetThresholds, MemoryHints, NoopBackendOptions, Queue, Trace,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Icon, Window, WindowId};

use crate::graphics::*;
use crate::input::{InputEvent, InputSystem};
use crate::interface::cursor::{MouseCursor, MouseCursorState};
use crate::interface::resource::{ItemSource, SkillSource};
use crate::interface::windows::*;
use crate::loaders::*;
#[cfg(feature = "debug")]
use crate::renderer::DebugMarkerRenderer;
use crate::renderer::{AlignHorizontal, EffectRenderer, GameInterfaceRenderer};
use crate::settings::{GameSettingsPathExt, GraphicsSettings, IN_GAME_THEMES_PATH, LightingMode, MENU_THEMES_PATH, WORLD_THEMES_PATH};
use crate::state::theme::{InterfaceTheme, InterfaceThemeType, WorldTheme};
use crate::system::GameTimer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::*;

const CLIENT_NAME: &str = "Korangar";
const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
const DEFAULT_MAP: &str = "geffen";
const START_CAMERA_FOCUS_POINT: Point3<f32> = Point3::new(600.0, 0.0, 240.0);
const DEFAULT_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3");
const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "버튼소리.wav";
// TODO: The number of point lights that can cast shadows should be configurable
// through the graphics settings. For now I just chose an arbitrary smaller
// number that should be playable on most devices.
const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;

const INITIAL_SCREEN_SIZE: ScreenSize = ScreenSize {
    width: 1280.0,
    height: 720.0,
};

const INITIAL_SCALING_FACTOR: Scaling = Scaling::new(1.0);
const FALLBACK_PACKET_VERSION: SupportedPacketVersion = SupportedPacketVersion::_20220406;

static ICON_DATA: &[u8] = include_bytes!("../archive/data/icon.png");

/// CTR+C was sent, and the client is supposed to close.
pub static SHUTDOWN_SIGNAL: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

#[cfg(feature = "debug")]
const DEBUG_WINDOWS: &[WindowClass] = &[
    WindowClass::CacheStatistics,
    WindowClass::ClientStateInspector,
    WindowClass::PacketInspector,
    WindowClass::Profiler,
    WindowClass::RenderOptions,
];

// Create the `threads` module.
#[cfg(feature = "debug")]
korangar_debug::create_profiler_threads!(threads, {
    Main,
    Loader,
});

mod character_slots {
    use korangar_interface::element::StateElement;
    use ragnarok_packets::{CharacterId, CharacterInformation};
    use rust_state::{Path, RustState, Selector};

    use crate::state::ClientState;

    #[derive(Default, RustState, StateElement)]
    pub struct CharacterSlots {
        slots: Vec<Option<CharacterInformation>>,
    }

    impl CharacterSlots {
        pub fn set_slot_count(&mut self, slot_count: usize) {
            self.slots.resize(slot_count, None);
        }

        pub fn get_slot_count(&self) -> usize {
            self.slots.len()
        }

        pub fn add_character(&mut self, character_information: CharacterInformation) {
            let Some(slot) = self.slots.get_mut(character_information.character_number as usize) else {
                panic!("attempted to add character to a slot that doesn't exist");
            };

            assert!(slot.is_none(), "attempted to add a character to an occupied slot");

            *slot = Some(character_information);
        }

        pub fn remove_with_id(&mut self, character_id: CharacterId) {
            self.slots.iter_mut().for_each(|slot| {
                if slot
                    .as_ref()
                    .is_some_and(|character_information| character_information.character_id == character_id)
                {
                    *slot = None;
                }
            })
        }

        pub fn with_id(&self, character_id: CharacterId) -> Option<&CharacterInformation> {
            self.slots
                .iter()
                .find(|slot| {
                    slot.as_ref()
                        .is_some_and(|character_information| character_information.character_id == character_id)
                })
                .and_then(|slot| slot.as_ref())
        }

        pub fn set_characters(&mut self, characters: Vec<CharacterInformation>) {
            // Clear the character list.
            self.slots.iter_mut().for_each(|slot| *slot = None);

            characters
                .into_iter()
                .for_each(|character_information| self.add_character(character_information));
        }
    }

    #[derive(Clone, Copy)]
    struct SlotPath<P>
    where
        P: Copy,
    {
        path: P,
        slot: usize,
    }

    impl<P> Path<ClientState, CharacterInformation, false> for SlotPath<P>
    where
        P: Path<ClientState, CharacterSlots>,
    {
        fn follow<'a>(&self, state: &'a ClientState) -> Option<&'a CharacterInformation> {
            // SAFETY
            // Unwrapping is fine here since it's guaranteed to be `Some` from the trait
            // bounds.
            self.path.follow(state).unwrap().slots.get(self.slot).and_then(|slot| slot.as_ref())
        }

        fn follow_mut<'a>(&self, state: &'a mut ClientState) -> Option<&'a mut CharacterInformation> {
            // SAFETY
            // Unwrapping is fine here since it's guaranteed to be `Some` from the trait
            // bounds.
            self.path
                .follow_mut(state)
                .unwrap()
                .slots
                .get_mut(self.slot)
                .and_then(|slot| slot.as_mut())
        }
    }

    impl<P> Selector<ClientState, CharacterInformation, false> for SlotPath<P>
    where
        P: Path<ClientState, CharacterSlots>,
    {
        fn select<'a>(&'a self, state: &'a ClientState) -> Option<&'a CharacterInformation> {
            self.follow(state)
        }
    }

    pub trait CharacterSlotsExt {
        fn in_slot(self, slot: usize) -> impl Path<ClientState, CharacterInformation, false>;
    }

    impl<P> CharacterSlotsExt for P
    where
        P: Path<ClientState, CharacterSlots>,
    {
        fn in_slot(self, slot: usize) -> impl Path<ClientState, CharacterInformation, false> {
            SlotPath { path: self, slot }
        }
    }
}

pub fn init_tls_rand() {
    use std::random::*;
    let mut seed = [0; 32];
    DefaultRandomSource.fill_bytes(&mut seed);
    rand_aes::tls::rand_seed(seed.into());
}

fn main() {
    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = threads::Main::start_frame();

    initialize_shutdown_signal();

    time_phase!("create global thread pool", {
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .start_handler(|_| init_tls_rand())
            .build_global()
            .unwrap();
    });

    time_phase!("seed main random instance", {
        init_tls_rand();
    });

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

    let args: Vec<String> = std::env::args().collect();
    let sync_cache = args.len() > 1 && &args[1] == "sync-cache";

    let Some(mut client) = Client::init(sync_cache) else {
        return;
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run_app(&mut client);
}

fn initialize_shutdown_signal() {
    ctrlc::set_handler(|| {
        println!("CTRL-C received. Shutting down");
        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

struct Client {
    game_file_loader: Arc<GameFileLoader>,
    action_loader: Arc<ActionLoader>,
    #[cfg(feature = "debug")]
    animation_loader: Arc<AnimationLoader>,
    async_loader: Arc<AsyncLoader>,
    effect_loader: Arc<EffectLoader>,
    font_loader: Arc<FontLoader>,
    sprite_loader: Arc<SpriteLoader>,
    texture_loader: Arc<TextureLoader>,
    library: Arc<Library>,

    interface_renderer: InterfaceRenderer,
    bottom_interface_renderer: GameInterfaceRenderer,
    middle_interface_renderer: GameInterfaceRenderer,
    top_interface_renderer: GameInterfaceRenderer,
    effect_renderer: EffectRenderer,
    #[cfg(feature = "debug")]
    debug_marker_renderer: DebugMarkerRenderer,
    #[cfg(feature = "debug")]
    aabb_instructions: Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    circle_instructions: Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    rectangle_instructions: Vec<DebugRectangleInstruction>,
    model_batches: Vec<ModelBatch>,
    model_instructions: Vec<ModelInstruction>,
    entity_instructions: Vec<EntityInstruction>,
    directional_shadow_model_batches: [Vec<ModelBatch>; PARTITION_COUNT],
    directional_shadow_model_instructions: Vec<ModelInstruction>,
    directional_shadow_entity_instructions: [Vec<EntityInstruction>; PARTITION_COUNT],
    point_shadow_model_batches: Vec<ModelBatch>,
    point_shadow_model_instructions: Vec<ModelInstruction>,
    point_shadow_entity_instructions: Vec<EntityInstruction>,
    point_light_with_shadow_instructions: Vec<PointLightWithShadowInstruction>,
    point_light_instructions: Vec<PointLightInstruction>,

    input_system: InputSystem,

    interface: Interface<'static, ClientState>,
    mouse_cursor: MouseCursor,
    show_interface: bool,
    game_timer: GameTimer,

    #[cfg(feature = "debug")]
    debug_camera: DebugCamera,
    start_camera: StartCamera,
    player_camera: PlayerCamera,
    directional_shadow_camera: DirectionalShadowCamera,
    directional_shadow_partitions: Arc<Mutex<[DirectionalShadowPartition; PARTITION_COUNT]>>,
    point_shadow_camera: PointShadowCamera,

    input_event_buffer: Vec<InputEvent>,
    network_event_buffer: NetworkEventBuffer,
    // TODO: Move or remove this.
    saved_login_data: Option<LoginServerLoginData>,
    // TODO: Move or remove this.
    saved_character_server: Option<CharacterServerInformation>,
    // TODO: Move or remove this.
    saved_login_server_address: Option<SocketAddr>,
    // TODO: Move or remove this.
    saved_password: String,
    // TODO: Move or remove this.
    saved_username: String,
    // TODO: Move or remove this.
    saved_packet_version: SupportedPacketVersion,

    particle_holder: ParticleHolder,
    point_light_manager: PointLightManager,
    effect_holder: EffectHolder,
    path_finder: PathFinder,

    point_light_set_buffer: ResourceSetBuffer<LightSourceKey>,
    directional_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: ResourceSetBuffer<ObjectKey>,

    #[cfg(feature = "debug")]
    pathing_texture_set: Arc<TextureSet>,
    #[cfg(feature = "debug")]
    tile_texture_set: Arc<TextureSet>,

    main_menu_click_sound_effect: SoundEffectKey,

    #[cfg(feature = "debug")]
    networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    networking_system: NetworkingSystem<NoPacketCallback>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    active_interface_settings: InterfaceSettings,
    active_graphics_settings: GraphicsSettings,
    graphics_engine: GraphicsEngine,
    queue: Queue,
    #[cfg(feature = "debug")]
    device: Device,
    window: Option<Arc<Window>>,

    map: Option<Box<Map>>,
    client_state: Context<ClientState>,
}

impl Client {
    fn init(sync_cache: bool) -> Option<Self> {
        time_phase!("load graphics settings", {
            let picker_value = Arc::new(AtomicU64::new(0));
            let directional_shadow_partitions = Arc::new(Mutex::new([DirectionalShadowPartition::default(); PARTITION_COUNT]));
            let input_system = InputSystem::new(picker_value.clone());
            let graphics_settings = GraphicsSettings::new();
        });

        time_phase!("create adapter", {
            let instance = Instance::new(&InstanceDescriptor {
                backends: Backends::all().with_env(),
                flags: InstanceFlags::from_build_config().with_env(),
                memory_budget_thresholds: MemoryBudgetThresholds::default(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::Automatic,
                        fence_behavior: GlFenceBehavior::Normal,
                    },
                    dx12: Dx12BackendOptions {
                        shader_compiler: Dx12Compiler::StaticDxc.with_env(),
                        presentation_system: Dx12SwapchainKind::DxgiFromHwnd,
                        latency_waitable_object: Dx12UseFrameLatencyWaitableObject::Wait,
                    },
                    noop: NoopBackendOptions { enable: false },
                },
            });

            let adapter = pollster::block_on(async { initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });

            #[cfg(feature = "debug")]
            {
                let adapter_info = adapter.get_info();
                print_debug!("using adapter {} ({})", adapter_info.name, adapter_info.backend);
                print_debug!("using device {} ({})", adapter_info.device, adapter_info.vendor);
                print_debug!("using driver {} ({})", adapter_info.driver, adapter_info.driver_info);
            }
        });

        time_phase!("create device", {
            let capabilities = Capabilities::from_adapter(&adapter);

            let (device, queue) = pollster::block_on(async {
                adapter
                    .request_device(&DeviceDescriptor {
                        label: None,
                        required_features: capabilities.get_required_features(),
                        required_limits: capabilities.get_required_limits(),
                        experimental_features: ExperimentalFeatures::disabled(),
                        memory_hints: MemoryHints::Performance,
                        trace: Trace::Off,
                    })
                    .await
                    .unwrap()
            });

            #[cfg(feature = "debug")]
            device.on_uncaptured_error(Arc::new(error_handler));

            #[cfg(feature = "debug")]
            print_debug!("received {} and {}", "queue".magenta(), "device".magenta());
        });

        time_phase!("create shader compiler", {
            let shader_compiler = ShaderCompiler::new(device.clone());
        });

        time_phase!("create game file loader", {
            let game_file_loader = Arc::new(GameFileLoader::default());

            game_file_loader.load_archives_from_settings();
            game_file_loader.load_patched_lua_files();
        });

        time_phase!("calculate game file hash", {
            let game_file_hash = game_file_loader.calculate_hash();
            #[cfg(feature = "debug")]
            print_debug!("game file hash: {}", game_file_hash);
        });

        time_phase!("create audio engine", {
            let audio_engine = Arc::new(AudioEngine::new(game_file_loader.clone()));
            audio_engine.set_background_music_volume(0.1);
        });

        time_phase!("create resource managers", {
            std::fs::create_dir_all(MENU_THEMES_PATH).unwrap();
            std::fs::create_dir_all(IN_GAME_THEMES_PATH).unwrap();
            std::fs::create_dir_all(WORLD_THEMES_PATH).unwrap();

            let model_loader = Arc::new(ModelLoader::new(game_file_loader.clone(), capabilities.bindless_support()));
            let texture_loader = Arc::new(TextureLoader::new(
                device.clone(),
                queue.clone(),
                &shader_compiler,
                &capabilities,
                game_file_loader.clone(),
            ));
            let video_loader = Arc::new(VideoLoader::new(game_file_loader.clone(), texture_loader.clone()));
            let font_loader = Arc::new(FontLoader::new(
                &["NotoSans".to_owned(), "NotoSansKR".to_owned()],
                &game_file_loader,
                &texture_loader,
            ));
            let map_loader = Arc::new(MapLoader::new(
                device.clone(),
                queue.clone(),
                game_file_loader.clone(),
                audio_engine.clone(),
                capabilities.bindless_support(),
            ));
            let sprite_loader = Arc::new(SpriteLoader::new(game_file_loader.clone(), texture_loader.clone()));
            let action_loader = Arc::new(ActionLoader::new(game_file_loader.clone(), audio_engine.clone()));
            let effect_loader = Arc::new(EffectLoader::new(game_file_loader.clone()));
            let animation_loader = Arc::new(AnimationLoader::new());

            let library = Arc::new(Library::new(&game_file_loader).unwrap_or_else(|_| {
                // The library not being created correctly means that the lua files were
                // not valid. It's possible that the archive was copied from a
                // different machine with a different architecture, so the one thing
                // we can try is generating it again.

                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to execute lua files; attempting to fix it by re-patching",
                    "error".red()
                );

                game_file_loader.remove_patched_lua_files();
                game_file_loader.load_patched_lua_files();

                Library::new(&game_file_loader).unwrap()
            }));

            if sync_cache {
                sync_cache_archive(&game_file_loader, texture_loader, game_file_hash);
                return None;
            }

            game_file_loader.load_cache_archive(game_file_hash);

            let async_loader = Arc::new(AsyncLoader::new(
                action_loader.clone(),
                animation_loader.clone(),
                map_loader.clone(),
                model_loader.clone(),
                sprite_loader.clone(),
                texture_loader.clone(),
                video_loader.clone(),
                library.clone(),
            ));

            let interface_renderer = InterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                font_loader.clone(),
                &texture_loader,
                graphics_settings.high_quality_interface,
            );
            let bottom_interface_renderer = GameInterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                INITIAL_SCALING_FACTOR,
                font_loader.clone(),
                #[cfg(feature = "debug")]
                &texture_loader,
            );
            let middle_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let top_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let effect_renderer = EffectRenderer::new(INITIAL_SCREEN_SIZE);
            #[cfg(feature = "debug")]
            let debug_marker_renderer = DebugMarkerRenderer::new();

            #[cfg(feature = "debug")]
            let aabb_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let circle_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let rectangle_instructions = Vec::default();
            let model_batches = Vec::default();
            let model_instructions = Vec::default();
            let entity_instructions = Vec::default();
            let directional_shadow_model_batches = Default::default();
            let directional_shadow_model_instructions = Vec::default();
            let directional_shadow_entity_instructions = Default::default();
            let point_shadow_model_batches = Vec::default();
            let point_shadow_model_instructions = Vec::default();
            let point_shadow_entity_instructions = Vec::default();
            let point_light_with_shadow_instructions = Vec::default();
            let point_light_instructions = Vec::default();
        });

        time_phase!("create graphics engine", {
            let graphics_engine = GraphicsEngine::initialize(GraphicsEngineDescriptor {
                capabilities,
                adapter,
                instance,
                device: device.clone(),
                queue: queue.clone(),
                shader_compiler,
                texture_loader: texture_loader.clone(),
                picker_value,
                directional_shadow_partitions: directional_shadow_partitions.clone(),
            });
        });

        time_phase!("initialize interface", {
            let mut interface = Interface::new(font_loader.clone(), INITIAL_SCREEN_SIZE);
            let mouse_cursor = MouseCursor::new(&sprite_loader, &action_loader);
            let show_interface = true;
        });

        time_phase!("initialize timer", {
            let game_timer = GameTimer::new();
        });

        time_phase!("initialize camera", {
            #[cfg(feature = "debug")]
            let debug_camera = DebugCamera::new();
            let mut start_camera = StartCamera::new();
            let player_camera = PlayerCamera::new();
            let mut directional_shadow_camera = DirectionalShadowCamera::new();
            let point_shadow_camera = PointShadowCamera::new();
            start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
        });

        // TODO: Move all of these to the ClientState
        let saved_login_data: Option<LoginServerLoginData> = None;
        let saved_character_server: Option<CharacterServerInformation> = None;
        let saved_login_server_address = None;
        let saved_password = String::new();
        let saved_username = String::new();
        let saved_packet_version = FALLBACK_PACKET_VERSION;

        time_phase!("initialize networking", {
            #[cfg(not(feature = "debug"))]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn();

            #[cfg(feature = "debug")]
            let (packet_history, packet_history_callback) = PacketHistory::new();
            #[cfg(feature = "debug")]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(packet_history_callback);
        });

        time_phase!("create resources", {
            let input_event_buffer = Vec::new();

            let particle_holder = ParticleHolder::default();
            let point_light_manager = PointLightManager::new();
            let effect_holder = EffectHolder::default();
            let path_finder = PathFinder::default();

            let point_light_set_buffer = ResourceSetBuffer::default();
            let directional_shadow_object_set_buffer = ResourceSetBuffer::default();
            let point_shadow_object_set_buffer = ResourceSetBuffer::default();
            let deferred_object_set_buffer = ResourceSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ResourceSetBuffer::default();

            #[cfg(feature = "debug")]
            let pathing_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "pathing", &[
                "pathing_goal.png",
                "pathing_straight.png",
                "pathing_diagonal.png",
            ]);
            #[cfg(feature = "debug")]
            let pathing_texture_set = Arc::new(pathing_texture_set);

            #[cfg(feature = "debug")]
            let tile_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), video_loader.clone(), "tile", &[
                "tile_0.png",
                "tile_1.png",
                "tile_2.png",
                "tile_3.png",
                "tile_4.png",
                "tile_5.png",
                "tile_6.png",
            ]);
            #[cfg(feature = "debug")]
            let tile_texture_set = Arc::new(tile_texture_set);

            let main_menu_click_sound_effect = audio_engine.load(MAIN_MENU_CLICK_SOUND_EFFECT);
        });

        time_phase!("load default map", {
            let map = map_loader
                .load(
                    DEFAULT_MAP.to_string(),
                    &model_loader,
                    texture_loader.clone(),
                    video_loader,
                    &library,
                )
                .expect("failed to load initial map");

            directional_shadow_camera.set_level_bound(map.get_level_bound());

            audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);
            map.set_ambient_sound_sources(&audio_engine);
        });

        time_phase!("create client state", {
            let client_state = Context::new(ClientState::new(
                &game_file_loader,
                graphics_settings.clone(),
                #[cfg(feature = "debug")]
                packet_history,
            ));
        });

        let active_interface_settings = client_state.follow(crate::client_state().interface_settings()).clone();

        interface.open_window(LoginWindow::new(
            ClientState::path().login_window(),
            ClientState::path().login_settings(),
            ClientState::path().client_info(),
        ));

        Some(Self {
            game_file_loader,
            action_loader,
            #[cfg(feature = "debug")]
            animation_loader,
            async_loader,
            effect_loader,
            font_loader,
            sprite_loader,
            texture_loader,
            library,
            interface_renderer,
            bottom_interface_renderer,
            middle_interface_renderer,
            top_interface_renderer,
            effect_renderer,
            #[cfg(feature = "debug")]
            debug_marker_renderer,
            #[cfg(feature = "debug")]
            aabb_instructions,
            #[cfg(feature = "debug")]
            circle_instructions,
            #[cfg(feature = "debug")]
            rectangle_instructions,
            model_batches,
            model_instructions,
            entity_instructions,
            directional_shadow_model_batches,
            directional_shadow_model_instructions,
            directional_shadow_entity_instructions,
            point_shadow_model_batches,
            point_shadow_model_instructions,
            point_shadow_entity_instructions,
            point_light_with_shadow_instructions,
            point_light_instructions,
            input_system,
            interface,
            mouse_cursor,
            show_interface,
            game_timer,
            #[cfg(feature = "debug")]
            debug_camera,
            start_camera,
            player_camera,
            directional_shadow_camera,
            directional_shadow_partitions,
            point_shadow_camera,
            input_event_buffer,
            network_event_buffer,
            saved_login_data,
            saved_character_server,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_packet_version,
            particle_holder,
            point_light_manager,
            effect_holder,
            path_finder,
            point_light_set_buffer,
            directional_shadow_object_set_buffer,
            point_shadow_object_set_buffer,
            deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            bounding_box_object_set_buffer,
            #[cfg(feature = "debug")]
            pathing_texture_set,
            #[cfg(feature = "debug")]
            tile_texture_set,
            main_menu_click_sound_effect,
            networking_system,
            audio_engine,
            active_interface_settings,
            active_graphics_settings: graphics_settings,
            graphics_engine,
            queue,
            #[cfg(feature = "debug")]
            device,
            window: None,

            map: Some(map),
            client_state,
        })
    }

    fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            event_loop.exit();
            return;
        }

        #[cfg(feature = "debug")]
        let _measurement = threads::Main::start_frame();

        #[cfg(feature = "debug")]
        let clear_measurement = Profiler::start_measurement("clear instructions");

        self.interface_renderer.clear();
        self.bottom_interface_renderer.clear();
        self.middle_interface_renderer.clear();
        self.top_interface_renderer.clear();
        self.effect_renderer.clear();
        #[cfg(feature = "debug")]
        self.debug_marker_renderer.clear();

        #[cfg(feature = "debug")]
        self.aabb_instructions.clear();
        #[cfg(feature = "debug")]
        self.circle_instructions.clear();
        #[cfg(feature = "debug")]
        self.rectangle_instructions.clear();
        self.model_batches.clear();
        self.model_instructions.clear();
        self.entity_instructions.clear();
        self.directional_shadow_model_batches.iter_mut().for_each(|batch| batch.clear());
        self.directional_shadow_model_instructions.clear();
        self.directional_shadow_entity_instructions
            .iter_mut()
            .for_each(|instructions| instructions.clear());
        self.point_shadow_model_batches.clear();
        self.point_shadow_model_instructions.clear();
        self.point_shadow_entity_instructions.clear();
        self.point_light_with_shadow_instructions.clear();
        self.point_light_instructions.clear();

        #[cfg(feature = "debug")]
        clear_measurement.stop();

        // We can only apply the graphic changes and reconfigure the surface once the
        // previous image was presented. Moving this function to the end of the
        // function results in surface configuration errors under DX12.
        self.update_settings();

        // TODO: Shouldn't this happen later? After the scaling has been potentially
        // changed by the UI.
        let scaling = *self.client_state.follow(client_state().interface_settings().scaling());
        self.bottom_interface_renderer.update_scaling(scaling);
        self.middle_interface_renderer.update_scaling(scaling);
        self.top_interface_renderer.update_scaling(scaling);

        let frame = self.graphics_engine.wait_for_next_frame();

        #[cfg(feature = "debug")]
        let timer_measurement = Profiler::start_measurement("update timers");

        let delta_time = self.game_timer.update();
        let animation_timer_ms = self.game_timer.get_animation_timer_ms();
        let client_tick = self.game_timer.get_client_tick();

        #[cfg(feature = "debug")]
        timer_measurement.stop();

        // TODO: Rename
        let input_report = self.input_system.update_delta(client_tick);

        self.networking_system.get_events(&mut self.network_event_buffer);

        #[cfg(feature = "debug")]
        let picker_measurement = Profiler::start_measurement("update picker target");

        if let PickerTarget::Entity(entity_id) = input_report.mouse_target
            && let Some(entity) = self
                .client_state
                .follow_mut(client_state().entities())
                .iter_mut()
                .find(|entity| entity.get_entity_id() == entity_id)
            && entity.are_details_unavailable()
            && self.networking_system.entity_details(entity_id).is_ok()
        {
            entity.set_details_requested();
        }

        #[cfg(feature = "debug")]
        picker_measurement.stop();

        #[cfg(feature = "debug")]
        let network_event_measurement = Profiler::start_measurement("process network events");

        for event in self.network_event_buffer.drain() {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    self.saved_login_data = Some(login_data);

                    *self.client_state.follow_mut(client_state().character_servers()) = character_servers;

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.interface
                        .open_window(ServerSelectionWindow::new(client_state().character_servers()));
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_login_server();

                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::LoginServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let socket_address = self.saved_login_server_address.unwrap();
                        self.networking_system.connect_to_login_server(
                            self.saved_packet_version,
                            socket_address,
                            &self.saved_username,
                            &self.saved_password,
                        );
                    }
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_slot_count(normal_slot_count);

                    let _ = self.networking_system.request_character_list();
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_character_server();
                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let login_data = self.saved_login_data.as_ref().unwrap();
                        let server = self.saved_character_server.clone().unwrap();
                        self.networking_system
                            .connect_to_character_server(self.saved_packet_version, login_data, server);
                    } else if !self.networking_system.is_map_server_connected() {
                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(LoginWindow::new(
                            client_state().login_window(),
                            client_state().login_settings(),
                            client_state().client_info(),
                        ));
                    }
                }
                NetworkEvent::MapServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the map server with error");
                    }

                    let login_data = self.saved_login_data.as_ref().unwrap();
                    let server = self.saved_character_server.clone().unwrap();
                    self.networking_system
                        .connect_to_character_server(self.saved_packet_version, login_data, server);

                    self.map = None;

                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    self.client_state.follow_mut(client_state().entities()).clear();
                    self.client_state.follow_mut(client_state().dead_entities()).clear();

                    self.audio_engine.play_background_music_track(None);

                    #[cfg(not(feature = "debug"))]
                    self.interface.close_all_windows();

                    #[cfg(feature = "debug")]
                    self.interface.close_all_windows_except(DEBUG_WINDOWS);

                    self.async_loader
                        .request_map_load(DEFAULT_MAP.to_string(), Some(TilePosition::new(0, 0)));
                }
                NetworkEvent::InitialStats {
                    strength_stat_points_cost,
                    agility_stat_points_cost,
                    vitality_stat_points_cost,
                    intelligence_stat_points_cost,
                    dexterity_stat_points_cost,
                    luck_stat_points_cost,
                } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.strength_stat_points_cost = strength_stat_points_cost;
                        player.agility_stat_points_cost = agility_stat_points_cost;
                        player.vitality_stat_points_cost = vitality_stat_points_cost;
                        player.intelligence_stat_points_cost = intelligence_stat_points_cost;
                        player.dexterity_stat_points_cost = dexterity_stat_points_cost;
                        player.luck_stat_points_cost = luck_stat_points_cost;
                    }
                }
                NetworkEvent::ResurrectPlayer { entity_id } => {
                    // If the resurrected player is us, close the resurrect window.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == entity_id)
                    {
                        self.interface.close_window_with_class(WindowClass::Respawn);
                    }
                }
                NetworkEvent::PlayerStandUp { entity_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_idle(client_tick);
                    }
                }
                NetworkEvent::AccountId { .. } => {}
                NetworkEvent::CharacterList { characters } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .set_characters(characters);

                    if !self.interface.is_window_with_class_open(WindowClass::CharacterSelection) {
                        // TODO: this will do one unnecessary restore_focus. check
                        // if that will be problematic

                        #[cfg(not(feature = "debug"))]
                        self.interface.close_all_windows();

                        #[cfg(feature = "debug")]
                        self.interface.close_all_windows_except(DEBUG_WINDOWS);

                        self.interface.open_window(CharacterSelectionWindow::new(
                            client_state().character_slots(),
                            client_state().switch_request(),
                        ));
                    }
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => self.interface.open_window(ErrorWindow::new(message.to_owned())),
                NetworkEvent::CharacterDeleted => {
                    if let Some(character_id) = self.client_state.follow_mut(client_state().currently_deleting()).take() {
                        self.client_state
                            .follow_mut(client_state().character_slots())
                            .remove_with_id(character_id);
                    }
                }
                NetworkEvent::CharacterDeletionFailed { message, .. } => {
                    *self.client_state.follow_mut(client_state().currently_deleting()) = None;
                    self.interface.open_window(ErrorWindow::new(message.to_owned()))
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    let saved_login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system
                        .connect_to_map_server(self.saved_packet_version, saved_login_data, login_data);
                    // Ask for the client tick right away, so that the player isn't de-synced when
                    // they spawn on the map.
                    let _ = self.networking_system.request_client_tick();

                    let character_information = self
                        .client_state
                        .follow(client_state().character_slots())
                        .with_id(login_data.character_id)
                        .cloned()
                        .unwrap();

                    let mut player = Entity::Player(Player::new(saved_login_data.account_id, &character_information, client_tick));

                    *self.client_state.follow_mut(client_state().player_name()) = character_information.name;

                    let entity_id = player.get_entity_id();
                    let entity_type = player.get_entity_type();
                    let entity_part_files = player.get_entity_part_files(&self.library);

                    if let Some(animation_data) = self
                        .async_loader
                        .request_animation_data_load(entity_id, entity_type, entity_part_files)
                    {
                        player.set_animation_data(animation_data);
                    }

                    self.client_state.follow_mut(client_state().entities()).push(player);

                    self.interface.close_window_with_class(WindowClass::CharacterSelection);
                    self.interface.open_window(CharacterOverviewWindow::new(
                        client_state().player_name(),
                        // TODO: Check that manually asserting is fine. Technically this window should only
                        // be open while the player is selected.
                        this_player().manually_asserted().base_level(),
                        // TODO: Check that manually asserting is fine. Technically this window should only
                        // be open while the player is selected.
                        this_player().manually_asserted().job_level(),
                    ));
                    self.interface
                        .open_window(ChatWindow::new(client_state().chat_window(), client_state().chat_messages()));
                    self.interface.open_window(HotbarWindow::new(client_state().hotbar().skills()));

                    // Put the dialog system in a well-defined state.
                    self.client_state.follow_mut(client_state().dialog_window()).end();

                    self.map = None;

                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    self.client_state
                        .follow_mut(client_state().character_slots())
                        .add_character(character_information);

                    self.interface.close_window_with_class(WindowClass::CharacterCreation);
                }
                NetworkEvent::CharacterCreationFailed { message, .. } => {
                    self.interface.open_window(ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterSlotSwitched => {
                    *self.client_state.follow_mut(client_state().switch_request()) = None;
                }
                NetworkEvent::CharacterSlotSwitchFailed => {
                    self.interface
                        .open_window(ErrorWindow::new("Failed to switch character slots".to_owned()));
                }
                NetworkEvent::AddEntity { entity_data } => {
                    if let Some(map) = &self.map
                        && let Some(npc) = Npc::new(map, &mut self.path_finder, entity_data, client_tick)
                    {
                        let mut npc = Entity::Npc(npc);

                        let entity_id = npc.get_entity_id();
                        let entity_type = npc.get_entity_type();
                        let entity_part_files = npc.get_entity_part_files(&self.library);

                        let entities = self.client_state.follow_mut(client_state().entities());

                        // Check if this entity was fading out and capture its current alpha value
                        // so we can smoothly transition to fading in from the same alpha.
                        let old_entity_alpha = entities
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .and_then(|entity| match entity.get_fade_state() {
                                FadeState::FadingOut { .. } => Some(entity.get_fade_state().calculate_alpha(client_tick)),
                                _ => None,
                            });

                        // Sometimes (like after a job change) the server will tell the client
                        // that a new entity appeared, even though it was already on screen. So
                        // to prevent the entity existing twice, we remove the old one.
                        entities.retain(|entity| entity.get_entity_id() != entity_id);

                        // If the entity was fading out, start fading in from its current alpha value.
                        if let Some(alpha) = old_entity_alpha {
                            npc.set_fade_state(FadeState::from_alpha(alpha, FadeDirection::In, client_tick));
                        }

                        if let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(entity_id, entity_type, entity_part_files)
                        {
                            npc.set_animation_data(animation_data);
                        }

                        #[cfg(feature = "debug")]
                        npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

                        entities.push(npc);
                    }
                }
                NetworkEvent::RemoveEntity { entity_id, reason } => {
                    // If the motive is dead, you need to set the player to dead.
                    if reason == DisappearanceReason::Died {
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            let entity_type = entity.get_entity_type();

                            if entity_type == EntityType::Monster {
                                let mut entity = entity.clone();
                                entity.set_dead(client_tick);
                                entity.stop_movement();

                                // Remove the entity from the list of alive entities.
                                self.client_state
                                    .follow_mut(client_state().entities())
                                    .retain(|entity| entity.get_entity_id() != entity_id);

                                // Add the entity to the list of dead entities.
                                self.client_state.follow_mut(client_state().dead_entities()).push(entity);
                            } else if entity_type == EntityType::Player {
                                entity.set_dead(client_tick);

                                // If the player is us, we need to open the respawn window.
                                if entity_id == self.client_state.follow(client_state().entities())[0].get_entity_id() {
                                    self.interface.open_window(RespawnWindow);
                                }
                            }
                        }
                    } else {
                        // For non-death disappearances, start fading out the entity.
                        if let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == entity_id)
                        {
                            // Preserve alpha when transitioning from any state to fading out.
                            let current_alpha = entity.get_fade_state().calculate_alpha(client_tick);
                            entity.set_fade_state(FadeState::from_alpha(current_alpha, FadeDirection::Out, client_tick));
                        }
                    }

                    // If the entity that was removed had an attack buffered we remove the entity
                    // from the buffer.
                    let buffered_attack_entity = self.client_state.follow_mut(client_state().buffered_attack_entity());
                    if buffered_attack_entity.is_some_and(|buffered_entity_id| buffered_entity_id == entity_id) {
                        *buffered_attack_entity = None;
                    }
                }
                NetworkEvent::EntityMove {
                    entity_id,
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    let entities = self.client_state.follow_mut(client_state().entities());
                    let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity
                        && let Some(map) = &self.map
                    {
                        entity.move_from_to(
                            map,
                            &mut self.path_finder,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        entity.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::PlayerMove {
                    origin,
                    destination,
                    starting_timestamp,
                } => {
                    if let Some(map) = &self.map
                        && let Some(player) = self.client_state.try_follow_mut(this_entity())
                    {
                        player.move_from_to(
                            map,
                            &mut self.path_finder,
                            origin.tile_position(),
                            destination.tile_position(),
                            starting_timestamp,
                        );
                        #[cfg(feature = "debug")]
                        player.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::ChangeMap { map_name, position } => {
                    self.map = None;
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    // Only the player must stay alive between map changes.
                    self.client_state.follow_mut(client_state().entities()).truncate(1);
                    self.client_state.follow_mut(client_state().dead_entities()).clear();

                    // Close any remaining dialogs.
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    self.async_loader.request_map_load(map_name, Some(position));
                }
                NetworkEvent::UpdateClientTick { client_tick, received_at } => {
                    self.game_timer.set_client_tick(client_tick, received_at);
                }
                NetworkEvent::ChatMessage { text, color } => {
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(text, color));
                }
                NetworkEvent::UpdateEntityDetails { entity_id, name } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.set_details(name);
                    }
                }
                NetworkEvent::DamageEffect {
                    source_entity_id,
                    destination_entity_id,
                    damage_amount,
                    attack_duration,
                    is_critical,
                } => {
                    let target_position = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .map(|entity| entity.get_tile_position());

                    // Auto attack logic.
                    if self
                        .client_state
                        .try_follow(this_entity())
                        .is_some_and(|player| player.get_entity_id() == source_entity_id)
                    {
                        let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                        let buffered_attack_entity = self.client_state.follow_mut(client_state().buffered_attack_entity());

                        if let Some(entity_id) = buffered_attack_entity.take() {
                            let _ = self.networking_system.player_attack(entity_id);

                            if auto_attack {
                                *buffered_attack_entity = Some(entity_id);
                            }
                        }
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == source_entity_id)
                    // TODO: Maybe also or_else this_entity?
                    {
                        if let Some(target_position) = target_position {
                            entity.rotate_towards(target_position);
                        }

                        entity.set_attack(attack_duration, is_critical, client_tick);
                    }

                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == destination_entity_id)
                        .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        let particle: Box<dyn Particle + Send + Sync> = match damage_amount {
                            Some(amount) => Box::new(DamageNumber::new(entity.get_position(), amount.to_string(), is_critical)),
                            None => Box::new(Miss::new(entity.get_position())),
                        };

                        self.particle_holder.spawn_particle(particle);
                    }
                }
                NetworkEvent::HealEffect { entity_id, heal_amount } => {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .or_else(|| self.client_state.try_follow(this_entity()))
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(HealNumber::new(entity.get_position(), heal_amount.to_string())));
                    }
                }
                NetworkEvent::UpdateEntityHealth {
                    entity_id,
                    health_points,
                    maximum_health_points,
                } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_health(health_points, maximum_health_points);
                    }
                }
                NetworkEvent::UpdateStat { stat_type } => {
                    if let Some(player) = self.client_state.try_follow_mut(this_player()) {
                        player.update_stat(stat_type);
                    }
                }
                NetworkEvent::OpenDialog { text, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        .initialize(npc_id)
                        .add_text(text);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddNextButton { npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // An NPCs could start the dialog with this packet so we want to make sure it's initialized.
                        .initialize(npc_id)
                        .add_next_button();

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddCloseButton { npc_id } => {
                    // Some NPCs send the `CloseButtonPacket` after the dialog
                    // has been closed. We want to filter these out because otherwise we get a
                    // close button at the start of the next dialog.
                    if self.interface.is_window_with_class_open(WindowClass::Dialog) {
                        self.client_state
                            .follow_mut(client_state().dialog_window())
                            // Technically this call is redundant since the window is already open
                            // but we keep it for consistency.
                            .initialize(npc_id)
                            .add_close_button();
                    }
                }
                NetworkEvent::AddChoiceButtons { choices, npc_id } => {
                    self.client_state
                        .follow_mut(client_state().dialog_window())
                        // Some NPCs start the dialog with this packet so we need to make sure it's initialized.
                        .initialize(npc_id)
                        .add_choice_buttons(choices);

                    self.interface.open_window(DialogWindow::new(client_state().dialog_window()));
                }
                NetworkEvent::AddQuestEffect { quest_effect } => {
                    if let Some(map) = &self.map {
                        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
                    }
                }
                NetworkEvent::RemoveQuestEffect { entity_id } => self.particle_holder.remove_quest_icon(entity_id),
                NetworkEvent::SetInventory { items } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .fill(&self.async_loader, &self.library, items);
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .add_item(&self.async_loader, &self.library, item);

                    // TODO: Update the selling items. If you pick up an item
                    // that you already have the sell window
                    // should allow you to sell the new
                    // amount of items.
                }
                NetworkEvent::InventoryItemRemoved { index, amount, .. } => {
                    self.client_state.follow_mut(client_state().inventory()).remove_item(index, amount);
                }
                NetworkEvent::SkillTree { skill_information } => {
                    self.client_state.follow_mut(client_state().skill_tree()).fill(
                        &self.sprite_loader,
                        &self.action_loader,
                        skill_information,
                        client_tick,
                    );
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_equipped_position(index, equipped_position);
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .unwrap();

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    entity.set_job(job_id as usize);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }
                }
                NetworkEvent::ChangeHair { account_id, hair_id } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .unwrap();

                    entity.set_hair(hair_id as usize);

                    if let Some(animation_data) = self.async_loader.request_animation_data_load(
                        entity.get_entity_id(),
                        entity.get_entity_type(),
                        entity.get_entity_part_files(&self.library),
                    ) {
                        entity.set_animation_data(animation_data);
                    }
                }
                NetworkEvent::LoggedOut => {
                    self.networking_system.disconnect_from_map_server();
                }
                NetworkEvent::FriendRequest { requestee } => {
                    self.interface.open_window(FriendRequestWindow::new(requestee));
                }
                NetworkEvent::FriendRemoved { account_id, character_id } => {
                    self.client_state
                        .follow_mut(client_state().friend_list())
                        .retain(|friend| !(friend.account_id == account_id && friend.character_id == character_id));
                }
                NetworkEvent::FriendAdded { friend } => {
                    self.client_state.follow_mut(client_state().friend_list()).push(friend);
                }
                NetworkEvent::VisualEffect { effect_path, entity_id } => {
                    let effect = self.effect_loader.get_or_load(effect_path, &self.texture_loader).unwrap();
                    let frame_timer = effect.new_frame_timer();

                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
                        Vector3::new(0.0, 9.0, 0.0),
                        // FIX: The point light ID needs to be unique.
                        // The point light manager uses the ID to decide which point light
                        // renders with a shadow. Having duplicate IDs might cause some
                        // visual artifacts, such as flickering, as the point lights switch
                        // between shadows and no shadows.
                        PointLightId::new(entity_id.0),
                        Vector3::new(0.0, 12.0, 0.0),
                        Color::WHITE,
                        50.0,
                        false,
                    )));
                }
                NetworkEvent::AddSkillUnit {
                    entity_id,
                    unit_id,
                    position,
                } => {
                    let Some(map) = &self.map else {
                        continue;
                    };

                    match unit_id {
                        UnitId::Firewall => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let effect = self.effect_loader.get_or_load("firewall.str", &self.texture_loader).unwrap();
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(255, 30, 0),
                                    60.0,
                                    true,
                                )),
                                entity_id,
                            );
                        }
                        UnitId::Pneuma => {
                            let Some(position) = map.get_world_position(position) else {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] entity with id {:?} is out of map bounds", "error".red(), entity_id);
                                continue;
                            };

                            let effect = self.effect_loader.get_or_load("pneuma1.str", &self.texture_loader).unwrap();
                            let frame_timer = effect.new_frame_timer();

                            self.effect_holder.add_unit(
                                Box::new(EffectWithLight::new(
                                    effect,
                                    frame_timer,
                                    EffectCenter::Position(position),
                                    Vector3::new(0.0, 0.0, 0.0),
                                    PointLightId::new(unit_id as u32),
                                    Vector3::new(0.0, 6.0, 0.0),
                                    Color::rgb_u8(83, 220, 108),
                                    40.0,
                                    false,
                                )),
                                entity_id,
                            );
                        }
                        _ => {}
                    }
                }
                NetworkEvent::RemoveSkillUnit { entity_id } => {
                    self.effect_holder.remove_unit(entity_id);
                }
                NetworkEvent::SetFriendList { friend_list } => {
                    *self.client_state.follow_mut(client_state().friend_list()) = friend_list;
                }
                NetworkEvent::SetHotkeyData { tab, hotkeys } => {
                    // FIX: Since we only have one hotbar at the moment, we ignore
                    // everything but 0.
                    if tab.0 != 0 {
                        continue;
                    }

                    for (index, hotkey) in hotkeys.into_iter().take(10).enumerate() {
                        match hotkey {
                            HotkeyState::Bound(hotkey) => {
                                let Some(mut skill) = self
                                    .client_state
                                    .follow(client_state().skill_tree())
                                    .find_skill(SkillId(hotkey.skill_id as u16))
                                else {
                                    self.client_state
                                        .follow_mut(client_state().hotbar())
                                        .clear_slot(&mut self.networking_system, HotbarSlot(index as u16));
                                    continue;
                                };

                                skill.skill_level = hotkey.quantity_or_skill_level;
                                self.client_state
                                    .follow_mut(client_state().hotbar())
                                    .set_slot(HotbarSlot(index as u16), skill);
                            }
                            HotkeyState::Unbound => self
                                .client_state
                                .follow_mut(client_state().hotbar())
                                .unset_slot(HotbarSlot(index as u16)),
                        }
                    }
                }
                NetworkEvent::OpenShop { items } => {
                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    *self.client_state.follow_mut(client_state().shop_items()) = items
                        .into_iter()
                        .map(|item| self.library.load_shop_item_metadata(&self.async_loader, item))
                        .collect();

                    self.interface
                        .open_window(BuyWindow::new(client_state().shop_items(), client_state().buy_cart()));
                    self.interface.open_window(BuyCartWindow::new(client_state().buy_cart()));
                }
                NetworkEvent::AskBuyOrSell { shop_id } => {
                    self.interface.open_window(BuyOrSellWindow::new(shop_id));
                }
                NetworkEvent::BuyingCompleted { result } => match result {
                    BuyShopItemsResult::Success => {
                        let _ = self.networking_system.close_shop();

                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Buy);
                        self.interface.close_window_with_class(WindowClass::BuyCart);
                    }
                    BuyShopItemsResult::Error => {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Failed to buy items".to_owned(), MessageColor::Error));
                    }
                },
                NetworkEvent::SellItemList { items } => {
                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    let inventory_items = self.client_state.follow(client_state().inventory().items());
                    let sell_items = items
                        .into_iter()
                        .map(|item| {
                            let inventory_item = inventory_items
                                .iter()
                                .find(|inventory_item| inventory_item.index == item.inventory_index)
                                .expect("item not in inventory");

                            let name = inventory_item.metadata.name.clone();
                            let texture = inventory_item.metadata.texture.clone();
                            let quantity = match &inventory_item.details {
                                korangar_networking::InventoryItemDetails::Regular { amount, .. } => *amount,
                                korangar_networking::InventoryItemDetails::Equippable { .. } => 1,
                            };

                            SellItem {
                                metadata: (ResourceMetadata { name, texture }, quantity),
                                inventory_index: item.inventory_index,
                                price: item.price,
                                overcharge_price: item.overcharge_price,
                            }
                        })
                        .collect();

                    *self.client_state.follow_mut(client_state().sell_items()) = sell_items;

                    self.interface
                        .open_window(SellWindow::new(client_state().sell_items(), client_state().sell_cart()));
                    self.interface.open_window(SellCartWindow::new(client_state().sell_cart()));
                }
                NetworkEvent::SellingCompleted { result } => match result {
                    SellItemsResult::Success => {
                        // Clear the cart.
                        self.client_state.follow_mut(client_state().buy_cart()).clear();

                        self.interface.close_window_with_class(WindowClass::Sell);
                        self.interface.close_window_with_class(WindowClass::SellCart);
                    }
                    SellItemsResult::Error => {
                        self.client_state
                            .follow_mut(client_state().chat_messages())
                            .push(ChatMessage::new("Failed to sell items".to_owned(), MessageColor::Error));
                    }
                },
                NetworkEvent::AttackFailed {
                    target_entity_id,
                    target_position,
                    player_position,
                    attack_range,
                } => {
                    if let Some(map) = &self.map
                        && self.client_state.try_follow_mut(this_entity()).is_some()
                        // Make sure that the entity is on screen.
                        && self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .any(|entity| entity.get_entity_id() == target_entity_id)
                        && let Some(path) =
                            self.path_finder
                                .find_walkable_path_in_range(&**map, player_position, target_position, attack_range)
                    {
                        let nearest_tile = path.last().unwrap();

                        let _ = self.networking_system.player_move(WorldPosition {
                            x: nearest_tile.x,
                            y: nearest_tile.y,
                            direction: Direction::North,
                        });

                        *self.client_state.follow_mut(client_state().buffered_attack_entity()) = Some(target_entity_id);
                    }
                }
            }
        }

        #[cfg(feature = "debug")]
        network_event_measurement.stop();

        #[cfg(feature = "debug")]
        let input_event_measurement = Profiler::start_measurement("process user events");

        self.interface.process_events(&mut self.input_event_buffer);
        let interface_has_focus = self.interface.has_focus();

        if self.interface.get_mouse_mode().is_rotating_camera() {
            // TODO: Does this really need to be a InputEvent?
            let rotation = input_report.mouse_delta.width;
            self.input_event_buffer.push(InputEvent::RotateCamera { rotation });
        }

        if !interface_has_focus {
            self.input_system.handle_keyboard_input(
                &mut self.input_event_buffer,
                #[cfg(feature = "debug")]
                self.interface.get_mouse_mode().is_default(),
                #[cfg(feature = "debug")]
                *self.client_state.follow(client_state().render_options().use_debug_camera()),
            );
        }

        for event in self.input_event_buffer.drain(..) {
            match event {
                InputEvent::LogIn {
                    service_id,
                    username,
                    password,
                } => {
                    let service = self
                        .client_state
                        .follow(client_state().client_info().services())
                        .iter()
                        .find(|service| service.service_id() == service_id)
                        .unwrap();
                    let address = format!("{}:{}", service.address, service.port);
                    let socket_address = address
                        .to_socket_addrs()
                        .expect("Failed to resolve IP")
                        .next()
                        .expect("ill formatted service IP");

                    let packet_version = match service.packet_version {
                        Some(packet_version) => match packet_version {
                            PacketVersion::_20220406 => SupportedPacketVersion::_20220406,
                            PacketVersion::Unsupported(packet_version) => {
                                self.interface.open_window(ErrorWindow::new(format!(
                                    "Selected server has an unsupported package version: {packet_version}"
                                )));
                                continue;
                            }
                        },
                        None => FALLBACK_PACKET_VERSION,
                    };

                    self.saved_login_server_address = Some(socket_address);
                    self.saved_username = username.clone();
                    self.saved_password = password.clone();
                    self.saved_packet_version = packet_version;

                    self.networking_system
                        .connect_to_login_server(packet_version, socket_address, username, password);
                }
                InputEvent::SelectServer {
                    character_server_information,
                } => {
                    self.saved_character_server = Some(character_server_information.clone());

                    self.networking_system.disconnect_from_login_server();

                    // Korangar should never attempt to connect to the character
                    // server before it logged in to the login server, so it's fine to
                    // unwrap here.
                    let login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system
                        .connect_to_character_server(self.saved_packet_version, login_data, character_server_information);
                }
                InputEvent::Respawn => {
                    let _ = self.networking_system.respawn();
                    self.interface.close_window_with_class(WindowClass::Respawn);
                }
                InputEvent::LogOut => {
                    let _ = self.networking_system.log_out();
                }
                InputEvent::LogOutCharacter => {
                    self.networking_system.disconnect_from_character_server();
                }
                InputEvent::Exit => event_loop.exit(),
                InputEvent::ZoomCamera { zoom_factor } => self.player_camera.soft_zoom(zoom_factor),
                InputEvent::RotateCamera { rotation } => self.player_camera.soft_rotate(rotation),
                InputEvent::ResetCameraRotation => self.player_camera.reset_rotation(),
                InputEvent::ToggleMenuWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Menu) {
                            true => self.interface.close_window_with_class(WindowClass::Menu),
                            false => self.interface.open_window(MenuWindow),
                        }
                    }
                }
                InputEvent::ToggleInventoryWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Inventory) {
                            true => self.interface.close_window_with_class(WindowClass::Inventory),
                            false => self.interface.open_window(InventoryWindow::new(client_state().inventory().items())),
                        }
                    }
                }
                InputEvent::ToggleEquipmentWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Equipment) {
                            true => self.interface.close_window_with_class(WindowClass::Equipment),
                            false => self.interface.open_window(EquipmentWindow::new(client_state().inventory().items())),
                        }
                    }
                }
                InputEvent::ToggleSkillTreeWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::SkillTree) {
                            true => self.interface.close_window_with_class(WindowClass::SkillTree),
                            false => self
                                .interface
                                .open_window(SkillTreeWindow::new(client_state().skill_tree().skills())),
                        }
                    }
                }
                InputEvent::ToggleStatsWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Stats) {
                            true => self.interface.close_window_with_class(WindowClass::Stats),
                            false => self.interface.open_window(StatsWindow::new(this_player().manually_asserted())),
                        }
                    }
                }
                InputEvent::ToggleGameSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GameSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GameSettings),
                    false => self.interface.open_window(GameSettingsWindow::new(client_state().game_settings())),
                },
                InputEvent::ToggleInterfaceSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::InterfaceSettings)
                {
                    true => self.interface.close_window_with_class(WindowClass::InterfaceSettings),
                    false => self.interface.open_window(InterfaceSettingsWindow::new(
                        client_state().interface_settings(),
                        client_state().interface_settings_capabilities(),
                    )),
                },
                InputEvent::ToggleGraphicsSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::GraphicsSettings) {
                    true => self.interface.close_window_with_class(WindowClass::GraphicsSettings),
                    false => self.interface.open_window(GraphicsSettingsWindow::new(
                        client_state().graphics_settings(),
                        client_state().graphics_settings_capabilities(),
                    )),
                },
                InputEvent::ToggleAudioSettingsWindow => match self.interface.is_window_with_class_open(WindowClass::AudioSettings) {
                    true => self.interface.close_window_with_class(WindowClass::AudioSettings),
                    false => self
                        .interface
                        .open_window(AudioSettingsWindow::new(client_state().audio_settings())),
                },
                InputEvent::ToggleFriendListWindow => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::FriendList) {
                            true => self.interface.close_window_with_class(WindowClass::FriendList),
                            false => self.interface.open_window(FriendListWindow::new(
                                client_state().friend_list_window(),
                                client_state().friend_list(),
                            )),
                        }
                    }
                }
                InputEvent::CloseTopWindow => self.interface.close_top_window(&self.client_state),
                InputEvent::ToggleShowInterface => self.show_interface = !self.show_interface,
                InputEvent::SelectCharacter { slot } => {
                    let _ = self.networking_system.select_character(slot);
                }
                InputEvent::OpenCharacterCreationWindow { slot } => {
                    // Clear the name before opening the window.
                    self.client_state.follow_mut(client_state().create_character_name()).clear();

                    self.interface
                        .open_window(CharacterCreationWindow::new(client_state().create_character_name(), slot))
                }
                InputEvent::CreateCharacter { slot, name } => {
                    let _ = self.networking_system.create_character(slot, name);
                }
                InputEvent::DeleteCharacter { character_id } => {
                    if self.client_state.follow(client_state().currently_deleting()).is_none() {
                        let _ = self.networking_system.delete_character(character_id);
                        *self.client_state.follow_mut(client_state().currently_deleting()) = Some(character_id);
                    }
                }
                InputEvent::SwitchCharacterSlot {
                    origin_slot,
                    destination_slot,
                } => {
                    let _ = self.networking_system.switch_character_slot(origin_slot, destination_slot);
                }
                InputEvent::PlayerMove { destination } => {
                    if self.client_state.try_follow(this_entity()).is_some() {
                        let _ = self.networking_system.player_move(WorldPosition {
                            x: destination.x,
                            y: destination.y,
                            direction: Direction::North,
                        });
                    }

                    // Unbuffer any buffered attack.
                    *self.client_state.follow_mut(client_state().buffered_attack_entity()) = None;
                }
                InputEvent::PlayerInteract { entity_id } => {
                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        let _ = match entity.get_entity_type() {
                            EntityType::Npc => self.networking_system.start_dialog(entity_id),
                            EntityType::Monster => {
                                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                                let buffered_attack_entity = self.client_state.follow_mut(client_state().buffered_attack_entity());

                                if auto_attack {
                                    *buffered_attack_entity = Some(entity_id);
                                }

                                self.networking_system.player_attack(entity_id)
                            }
                            EntityType::Warp => self.networking_system.player_move({
                                let position = entity.get_tile_position();
                                WorldPosition {
                                    x: position.x,
                                    y: position.y,
                                    direction: Direction::North,
                                }
                            }),
                            _ => Ok(()),
                        };
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::WarpToMap { map_name, position } => {
                    let _ = self.networking_system.warp_to_map(map_name, position);
                }
                InputEvent::SendMessage { text } => {
                    // Handle special client commands.
                    if text.as_str() == "/nc" {
                        let auto_attack = self.client_state.follow_mut(client_state().game_settings().auto_attack());
                        *auto_attack = !*auto_attack;
                        continue;
                    }

                    let _ = self
                        .networking_system
                        .send_chat_message(self.client_state.follow(client_state().player_name()), &text);
                }
                InputEvent::NextDialog { npc_id } => {
                    let _ = self.networking_system.next_dialog(npc_id);
                }
                InputEvent::CloseDialog { npc_id } => {
                    let _ = self.networking_system.close_dialog(npc_id);
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);
                }
                InputEvent::ChooseDialogOption { npc_id, option } => {
                    let _ = self.networking_system.choose_dialog_option(npc_id, option);

                    if option == -1 {
                        self.interface.close_window_with_class(WindowClass::Dialog);
                    }
                }
                InputEvent::MoveItem { source, destination, item } => match (source, destination) {
                    (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                        let _ = self.networking_system.request_item_equip(item.index, position);
                    }
                    (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                        let _ = self.networking_system.request_item_unequip(item.index);
                    }
                    _ => {}
                },
                InputEvent::MoveSkill {
                    source,
                    destination,
                    skill,
                } => match (source, destination) {
                    (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .update_slot(&mut self.networking_system, slot, skill);
                    }
                    (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                        self.client_state.follow_mut(client_state().hotbar()).swap_slot(
                            &mut self.networking_system,
                            source_slot,
                            destination_slot,
                        );
                    }
                    _ => {}
                },
                InputEvent::CastSkill { slot } => {
                    if let Some(skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref() {
                        match skill.skill_type {
                            SkillType::Passive => {}
                            SkillType::Attack => {
                                if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                                    let _ = self.networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                }
                            }
                            SkillType::Ground | SkillType::Trap => {
                                if let PickerTarget::Tile { x, y } = input_report.mouse_target {
                                    let _ = self
                                        .networking_system
                                        .cast_ground_skill(skill.skill_id, skill.skill_level, TilePosition { x, y });
                                }
                            }
                            SkillType::SelfCast => match skill.skill_id == ROLLING_CUTTER_ID {
                                true => {
                                    let _ = self.networking_system.cast_channeling_skill(
                                        skill.skill_id,
                                        skill.skill_level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                                false => {
                                    let _ = self.networking_system.cast_skill(
                                        skill.skill_id,
                                        skill.skill_level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                            },
                            SkillType::Support => {
                                if let PickerTarget::Entity(entity_id) = input_report.mouse_target {
                                    let _ = self.networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                } else {
                                    let _ = self.networking_system.cast_skill(
                                        skill.skill_id,
                                        skill.skill_level,
                                        self.client_state.follow(this_entity().manually_asserted()).get_entity_id(),
                                    );
                                }
                            }
                        }
                    }
                }
                InputEvent::StopSkill { slot } => {
                    if let Some(skill) = self.client_state.follow(client_state().hotbar()).get_skill_in_slot(slot).as_ref()
                        && skill.skill_id == ROLLING_CUTTER_ID
                    {
                        let _ = self.networking_system.stop_channeling_skill(skill.skill_id);
                    }
                }
                InputEvent::AddFriend { character_name } => {
                    if character_name.len() > 24 {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] friend name {} is too long", "error".red(), character_name.magenta());
                    } else {
                        let _ = self.networking_system.add_friend(character_name);
                    }
                }
                InputEvent::RemoveFriend { account_id, character_id } => {
                    let _ = self.networking_system.remove_friend(account_id, character_id);
                }
                InputEvent::RejectFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.reject_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::AcceptFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.accept_friend_request(account_id, character_id);
                    self.interface.close_window_with_class(WindowClass::FriendRequest);
                }
                InputEvent::BuyItems { items } => {
                    let _ = self.networking_system.purchase_items(items);
                }
                InputEvent::CloseShop => {
                    let _ = self.networking_system.close_shop();

                    // Clear the carts.
                    self.client_state.follow_mut(client_state().buy_cart()).clear();
                    self.client_state.follow_mut(client_state().sell_cart()).clear();

                    self.interface.close_window_with_class(WindowClass::Buy);
                    self.interface.close_window_with_class(WindowClass::BuyCart);
                    self.interface.close_window_with_class(WindowClass::Sell);
                    self.interface.close_window_with_class(WindowClass::SellCart);
                }
                InputEvent::BuyOrSell { shop_id, buy_or_sell } => {
                    let _ = self.networking_system.select_buy_or_sell(shop_id, buy_or_sell);
                    self.interface.close_window_with_class(WindowClass::BuyOrSell);
                }
                InputEvent::SellItems { items } => {
                    let _ = self.networking_system.sell_items(items);
                }
                InputEvent::StatUp { stat_type } => {
                    let _ = self.networking_system.request_stat_up(stat_type);
                }
                #[cfg(feature = "debug")]
                InputEvent::ReloadLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    *self.client_state.follow_mut(client_state().localization()) =
                        Localization::load_language(&self.game_file_loader, language);
                }
                #[cfg(feature = "debug")]
                InputEvent::SaveLanguage => {
                    let language = *self.client_state.follow(client_state().interface_settings().language());
                    self.client_state.follow(client_state().localization()).save_language(language);
                }
                #[cfg(feature = "debug")]
                InputEvent::OpenMarkerDetails { marker_identifier } => {
                    if let Some(map) = &self.map {
                        match marker_identifier {
                            MarkerIdentifier::Object(key) => {
                                let inspecting_objects = self.client_state.follow_mut(client_state().inspecting_objects());
                                let object = map.get_object(key);
                                let object_path = state::prepare_object_inspection(inspecting_objects, object);

                                self.interface.open_state_window(object_path);
                            }
                            MarkerIdentifier::LightSource(key) => {
                                let inspecting_lights = self.client_state.follow_mut(client_state().inspecting_light_sources());
                                let light_source = map.get_light_source(key);
                                let light_source_path = state::prepare_light_source_inspection(inspecting_lights, light_source);

                                self.interface.open_state_window(light_source_path);
                            }
                            MarkerIdentifier::SoundSource(index) => {
                                let inspecting_sounds = self.client_state.follow_mut(client_state().inspecting_sound_sources());
                                let sound_source = map.get_sound_source(index);
                                let sound_source_path = state::prepare_sound_source_inspection(inspecting_sounds, sound_source);

                                self.interface.open_state_window(sound_source_path);
                            }
                            MarkerIdentifier::EffectSource(index) => {
                                let inspecting_effects = self.client_state.follow_mut(client_state().inspecting_effect_sources());
                                let effect_source = map.get_effect_source(index);
                                let effect_source_path = state::prepare_effect_source_inspection(inspecting_effects, effect_source);

                                self.interface.open_state_window(effect_source_path);
                            }
                            MarkerIdentifier::Particle(..) => {
                                // TODO:
                            }
                            MarkerIdentifier::Entity(index) => {
                                let entity_id = self
                                    .client_state
                                    .try_follow(client_state().entities().index(index as usize))
                                    .expect("entity should exist")
                                    .get_entity_id();

                                // This can technically still be `None`, violating the API but we handle this
                                // case in the state window.
                                let entity_path = client_state().entities().lookup(entity_id).manually_asserted();

                                self.interface.open_state_window(entity_path);
                            }
                            MarkerIdentifier::Shadow(..) => {
                                // TODO:
                            }
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleRenderOptionsWindow => match self.interface.is_window_with_class_open(WindowClass::RenderOptions) {
                    true => self.interface.close_window_with_class(WindowClass::RenderOptions),
                    false => self
                        .interface
                        .open_window(RenderOptionsWindow::new(client_state().render_options())),
                },
                #[cfg(feature = "debug")]
                InputEvent::OpenMapDataWindow => {
                    if self.map.is_some() {
                        let inspecting_maps = self.client_state.follow_mut(client_state().inspecting_maps());
                        let map_data = self.map.as_ref().unwrap().get_map_data();
                        let map_data_path = state::prepare_map_inspection(inspecting_maps, map_data);

                        self.interface.open_state_window(map_data_path);
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleClientStateInspectorWindow => {
                    match self.interface.is_window_with_class_open(WindowClass::ClientStateInspector) {
                        true => self.interface.close_window_with_class(WindowClass::ClientStateInspector),
                        false => self.interface.open_state_window_mut(client_state()),
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleMapsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Maps) {
                            true => self.interface.close_window_with_class(WindowClass::Maps),
                            false => self.interface.open_window(MapsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleCommandsWindow => {
                    if self.map.is_some() {
                        match self.interface.is_window_with_class_open(WindowClass::Commands) {
                            true => self.interface.close_window_with_class(WindowClass::Commands),
                            false => self.interface.open_window(CommandsWindow),
                        }
                    }
                }
                #[cfg(feature = "debug")]
                InputEvent::ToggleThemeInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::ThemeInspector) {
                    true => self.interface.close_window_with_class(WindowClass::ThemeInspector),
                    false => self.interface.open_window(ThemeInspectorWindow::new(
                        client_state().theme_inspector_window(),
                        client_state().menu_theme(),
                        client_state().in_game_theme(),
                        client_state().world_theme(),
                    )),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleProfilerWindow => match self.interface.is_window_with_class_open(WindowClass::Profiler) {
                    true => self.interface.close_window_with_class(WindowClass::Profiler),
                    false => self.interface.open_window(ProfilerWindow::new(client_state().profiler_window())),
                },
                #[cfg(feature = "debug")]
                InputEvent::TogglePacketInspectorWindow => match self.interface.is_window_with_class_open(WindowClass::PacketInspector) {
                    true => self.interface.close_window_with_class(WindowClass::PacketInspector),
                    false => self
                        .interface
                        .open_window(PacketInspectorWindow::new(client_state().packet_history())),
                },
                #[cfg(feature = "debug")]
                InputEvent::ToggleCacheStatisticsWindow => match self.interface.is_window_with_class_open(WindowClass::CacheStatistics) {
                    true => self.interface.close_window_with_class(WindowClass::CacheStatistics),
                    false => self.interface.open_state_window(client_state().cache_statistics()),
                },
                #[cfg(feature = "debug")]
                InputEvent::CameraLookAround { offset } => self.debug_camera.look_around(offset),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time as f32),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time as f32),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time as f32),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveRight => self.debug_camera.move_right(delta_time as f32),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveUp => self.debug_camera.move_up(delta_time as f32),
                #[cfg(feature = "debug")]
                InputEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                InputEvent::CameraDecelerate => self.debug_camera.decelerate(),
                #[cfg(feature = "debug")]
                InputEvent::InspectFrame { measurement } => self.interface.open_window(FrameInspectorWindow::new(measurement)),
            }
        }

        #[cfg(feature = "debug")]
        input_event_measurement.stop();

        #[cfg(feature = "debug")]
        let loads_measurement = Profiler::start_measurement("complete async loads");

        for completed in self.async_loader.take_completed() {
            match completed {
                (LoaderId::AnimationData(entity_id), LoadableResource::AnimationData(animation_data)) => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture, location }) => match location {
                    ItemLocation::Inventory => {
                        self.client_state
                            .follow_mut(client_state().inventory())
                            .update_item_sprite(item_id, texture);
                    }
                    ItemLocation::Shop => {
                        self.client_state
                            .follow_mut(client_state().shop_items())
                            .iter_mut()
                            .filter(|item| item.item_id == item_id)
                            .for_each(|item| item.metadata.texture = Some(texture.clone()));
                    }
                },
                (LoaderId::Map(..), LoadableResource::Map { map, position }) => {
                    match self.client_state.try_follow(this_player()).is_none() {
                        true => {
                            // Load of main menu map
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);

                            self.interface.open_window(CharacterSelectionWindow::new(
                                client_state().character_slots(),
                                client_state().switch_request(),
                            ));

                            self.start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                        }
                        false => {
                            // Normal map switch
                            let map = self.map.insert(map);

                            map.set_ambient_sound_sources(&self.audio_engine);
                            self.audio_engine.play_background_music_track(map.background_music_track_name());

                            if let Some(position) = position {
                                // SAFETY
                                // `manually_asserted` is safe because we are in
                                // the branch where `this_player`
                                // is not `None`.
                                let player = self.client_state.follow_mut(this_entity().manually_asserted());

                                player.set_position(map, position, client_tick);
                                self.player_camera.set_focus_point(player.get_position());
                            }

                            self.directional_shadow_camera.set_level_bound(map.get_level_bound());
                            let _ = self.networking_system.map_loaded();
                        }
                    }
                }
                _ => {}
            }
        }

        #[cfg(feature = "debug")]
        loads_measurement.stop();

        // Update the packet history callback.
        #[cfg(feature = "debug")]
        {
            profile_block!("update packet history");

            let is_packet_inspector_open = self.interface.is_window_with_class_open(WindowClass::PacketInspector);
            self.client_state
                .follow_mut(client_state().packet_history())
                .update(is_packet_inspector_open);
        }

        #[cfg(feature = "debug")]
        {
            profile_block!("update cache statistics");

            self.client_state.follow_mut(client_state().cache_statistics()).update(
                delta_time,
                &self.texture_loader,
                &self.sprite_loader,
                &self.font_loader,
                &self.audio_engine,
                &self.action_loader,
                &self.animation_loader,
                &self.effect_loader,
            );
        }

        // Main map update and render loop
        if self.map.is_some() {
            #[cfg(feature = "debug")]
            let update_main_camera_measurement = Profiler::start_measurement("update main camera");

            let window_size = self.graphics_engine.get_window_size();
            let screen_size: ScreenSize = window_size.into();

            if self.client_state.try_follow(this_entity()).is_some() {
                self.player_camera.update(delta_time);
                self.player_camera.generate_view_projection(window_size);
            } else {
                self.start_camera.update(delta_time);
                self.start_camera.generate_view_projection(window_size);
            }

            #[cfg(feature = "debug")]
            let render_options = *self.client_state.follow(client_state().render_options());

            #[cfg(feature = "debug")]
            self.interface_renderer.update_render_options(&render_options);

            #[cfg(feature = "debug")]
            if render_options.use_debug_camera {
                self.debug_camera.generate_view_projection(window_size);
            }

            #[cfg(feature = "debug")]
            update_main_camera_measurement.stop();

            #[cfg(feature = "debug")]
            let update_entities_measurement = Profiler::start_measurement("update entities");

            let currently_playing = self.client_state.try_follow(this_player()).is_some();

            {
                let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
                    #[cfg(feature = "debug")]
                    _ if render_options.use_debug_camera => &self.debug_camera,
                    true => &self.player_camera,
                    false => &self.start_camera,
                };

                self.client_state
                    .follow_mut(client_state().entities())
                    .iter_mut()
                    .for_each(|entity| entity.update(&self.audio_engine, self.map.as_ref().unwrap(), current_camera, client_tick));

                self.client_state
                    .follow_mut(client_state().dead_entities())
                    .iter_mut()
                    .for_each(|entity| entity.update(&self.audio_engine, self.map.as_ref().unwrap(), current_camera, client_tick));

                // Remove entities that have finished fading out.
                self.client_state
                    .follow_mut(client_state().entities())
                    .retain(|entity| !entity.is_fading_out_complete(client_tick));

                // Buffered attack (the player tried attacking while out of range).
                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                if self
                    .client_state
                    .try_follow(this_entity())
                    .is_some_and(|player| player.stopped_moving())
                {
                    let buffered_attack_entity = self.client_state.follow_mut(client_state().buffered_attack_entity());
                    if let Some(entity_id) = buffered_attack_entity.take() {
                        let _ = self.networking_system.player_attack(entity_id);

                        if auto_attack {
                            *buffered_attack_entity = Some(entity_id);
                        }
                    }
                }
            }

            #[cfg(feature = "debug")]
            update_entities_measurement.stop();

            let map = self.map.as_ref().unwrap();

            #[cfg(feature = "debug")]
            let update_videos_measurement = Profiler::start_measurement("update videos");

            map.advance_videos(&self.queue, delta_time);

            #[cfg(feature = "debug")]
            update_videos_measurement.stop();

            if currently_playing {
                // SAFETY
                // `manually_asserted` is safe because we are in the branch where `this_player`
                // is not `None`.
                let position = self.client_state.follow(this_entity().manually_asserted()).get_position();
                self.player_camera.set_smoothed_focus_point(position);
            }

            let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
                #[cfg(feature = "debug")]
                _ if render_options.use_debug_camera => &self.debug_camera,
                true => &self.player_camera,
                false => &self.start_camera,
            };

            let (view_matrix, projection_matrix) = current_camera.view_projection_matrices();
            let camera_position = current_camera.camera_position().to_homogeneous();

            #[cfg(feature = "debug")]
            let update_shadow_camera_measurement = Profiler::start_measurement("update directional shadow camera");

            let lighting_mode = *self.client_state.follow(client_state().graphics_settings().lighting_mode());
            let shadow_resolution = *self.client_state.follow(client_state().graphics_settings().shadow_resolution());
            let shadow_method = *self.client_state.follow(client_state().graphics_settings().shadow_method());
            let shadow_detail = *self.client_state.follow(client_state().graphics_settings().shadow_detail());
            let sdsm_enabled = *self.client_state.follow(client_state().graphics_settings().sdsm());
            let use_sdsm = sdsm_enabled & !self.player_camera.is_rotating_or_zooming_fast();

            let ambient_light_color = map.ambient_light_color();

            let (directional_light_direction, directional_light_color) = map.directional_light();

            match use_sdsm {
                true => {
                    self.directional_shadow_camera.update_camera_sdsm(
                        directional_light_direction,
                        &view_matrix,
                        &projection_matrix,
                        shadow_resolution.directional_shadow_resolution(),
                        self.directional_shadow_partitions.lock().unwrap().deref(),
                    );
                }
                false => {
                    self.directional_shadow_camera.update_camera_pssm(
                        directional_light_direction,
                        &view_matrix,
                        &projection_matrix,
                        shadow_resolution.directional_shadow_resolution(),
                    );
                }
            }

            let directional_light_view_projection_matrix = self.directional_shadow_camera.view_projection_matrix();

            #[cfg(feature = "debug")]
            update_shadow_camera_measurement.stop();

            #[cfg(feature = "debug")]
            let frame_measurement = Profiler::start_measurement("update audio engine");

            // We set the listener roughly at ear height.
            const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
            let listener = current_camera.focus_point() + EAR_HEIGHT;

            self.audio_engine
                .set_spatial_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
            self.audio_engine.update();

            #[cfg(feature = "debug")]
            frame_measurement.stop();

            #[cfg(feature = "debug")]
            let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

            self.particle_holder.update(delta_time as f32);
            self.effect_holder
                .update(self.client_state.follow(client_state().entities()), delta_time as f32);

            self.mouse_cursor.update(client_tick);

            let walk_indicator_color = *self.client_state.follow(client_state().world_theme().indicator().walking());

            #[cfg(feature = "debug")]
            let hovered_marker_identifier = match input_report.mouse_target {
                PickerTarget::Marker(marker_identifier) => Some(marker_identifier),
                _ => None,
            };

            #[cfg(feature = "debug")]
            let point_light_manager_measurement = Profiler::start_measurement("point light manager");

            let point_light_set = {
                self.point_light_manager.prepare();

                self.effect_holder
                    .register_point_lights(&mut self.point_light_manager, current_camera);

                map.register_point_lights(&mut self.point_light_manager, &mut self.point_light_set_buffer, current_camera);

                match lighting_mode {
                    LightingMode::Classic => self.point_light_manager.create_point_light_set(0),
                    LightingMode::Enhanced => self.point_light_manager.create_point_light_set(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS),
                }
            };

            #[cfg(feature = "debug")]
            point_light_manager_measurement.stop();

            #[cfg(feature = "debug")]
            prepare_frame_measurement.stop();

            #[cfg(feature = "debug")]
            let collect_instructions_measurement = Profiler::start_measurement("collect instructions");

            let picker_position = ScreenPosition {
                left: input_report.mouse_position.left.clamp(0.0, window_size.x as f32),
                top: input_report.mouse_position.top.clamp(0.0, window_size.y as f32),
            };
            let mut indicator_instruction = None;
            let mut water_instruction = None;

            // Marker
            {
                #[cfg(feature = "debug")]
                map.render_markers(
                    &mut self.debug_marker_renderer,
                    current_camera,
                    &render_options,
                    self.client_state.follow(client_state().entities()),
                    &point_light_set,
                    hovered_marker_identifier,
                );

                #[cfg(feature = "debug")]
                map.render_markers(
                    &mut self.middle_interface_renderer,
                    current_camera,
                    &render_options,
                    self.client_state.follow(client_state().entities()),
                    &point_light_set,
                    hovered_marker_identifier,
                );
            }

            // Directional Shadows
            {
                for partition_index in 0..PARTITION_COUNT {
                    let partition_camera = self.directional_shadow_camera.get_partition_camera(partition_index);

                    let object_set = map.cull_objects_with_frustum(
                        &partition_camera,
                        &mut self.directional_shadow_object_set_buffer,
                        #[cfg(feature = "debug")]
                        render_options.frustum_culling,
                    );

                    let offset = self.directional_shadow_model_instructions.len();
                    let model_batches = &mut self.directional_shadow_model_batches[partition_index];
                    let entity_instructions = &mut self.directional_shadow_entity_instructions[partition_index];

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_objects))]
                    map.render_objects(
                        &mut self.directional_shadow_model_instructions,
                        &object_set,
                        animation_timer_ms,
                        &partition_camera,
                    );

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_map))]
                    map.render_ground(&mut self.directional_shadow_model_instructions);

                    let count = self.directional_shadow_model_instructions.len() - offset;

                    model_batches.push(ModelBatch {
                        offset,
                        count,
                        texture_set: map.get_texture_set().clone(),
                        vertex_buffer: map.get_model_vertex_buffer().clone(),
                        index_buffer: map.get_model_index_buffer().clone(),
                    });

                    #[cfg(feature = "debug")]
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_map_tiles))]
                    map.render_overlay_tiles(
                        &mut self.directional_shadow_model_instructions,
                        model_batches,
                        &self.tile_texture_set,
                    );

                    #[cfg(feature = "debug")]
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_pathing))]
                    map.render_entity_pathing(
                        &mut self.directional_shadow_model_instructions,
                        model_batches,
                        self.client_state.follow(client_state().entities()),
                        &self.pathing_texture_set,
                    );

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_entities))]
                    map.render_entities(
                        entity_instructions,
                        self.client_state.follow(client_state().entities()),
                        &partition_camera,
                        client_tick,
                    );

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_entities))]
                    map.render_dead_entities(
                        entity_instructions,
                        self.client_state.follow(client_state().dead_entities()),
                        &partition_camera,
                        client_tick,
                    );
                }
            }

            // Point Lights and Shadows
            {
                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.enable_point_lights))]
                point_light_set.render_point_lights(&mut self.point_light_instructions);

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.enable_point_lights))]
                point_light_set.render_point_lights_with_shadows(
                    map,
                    &mut self.point_shadow_camera,
                    &mut self.point_shadow_object_set_buffer,
                    &mut self.point_shadow_model_instructions,
                    &mut self.point_light_with_shadow_instructions,
                    animation_timer_ms,
                    #[cfg(feature = "debug")]
                    &render_options,
                );
            }

            // Geometry
            {
                let object_set = map.cull_objects_with_frustum(
                    current_camera,
                    &mut self.deferred_object_set_buffer,
                    #[cfg(feature = "debug")]
                    render_options.frustum_culling,
                );

                let offset = self.model_instructions.len();

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_objects))]
                map.render_objects(&mut self.model_instructions, &object_set, animation_timer_ms, current_camera);

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_map))]
                map.render_ground(&mut self.model_instructions);

                let count = self.model_instructions.len() - offset;

                self.model_batches.push(ModelBatch {
                    offset,
                    count,
                    texture_set: map.get_texture_set().clone(),
                    vertex_buffer: map.get_model_vertex_buffer().clone(),
                    index_buffer: map.get_model_index_buffer().clone(),
                });

                #[cfg(feature = "debug")]
                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_map_tiles))]
                map.render_overlay_tiles(&mut self.model_instructions, &mut self.model_batches, &self.tile_texture_set);

                #[cfg(feature = "debug")]
                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_pathing))]
                map.render_entity_pathing(
                    &mut self.model_instructions,
                    &mut self.model_batches,
                    self.client_state.follow(client_state().entities()),
                    &self.pathing_texture_set,
                );

                let entity_camera = match true {
                    #[cfg(feature = "debug")]
                    _ if *self.client_state.follow(client_state().render_options().show_entities_paper()) => &self.player_camera,
                    _ => current_camera,
                };

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_entities))]
                map.render_entities(
                    &mut self.entity_instructions,
                    self.client_state.follow(client_state().entities()),
                    entity_camera,
                    client_tick,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_entities))]
                map.render_dead_entities(
                    &mut self.entity_instructions,
                    self.client_state.follow(client_state().dead_entities()),
                    entity_camera,
                    client_tick,
                );

                #[cfg(feature = "debug")]
                if render_options.show_entities_debug {
                    map.render_entities_debug(
                        &mut self.rectangle_instructions,
                        self.client_state.follow(client_state().entities()),
                        entity_camera,
                    );

                    map.render_entities_debug(
                        &mut self.rectangle_instructions,
                        self.client_state.follow(client_state().dead_entities()),
                        entity_camera,
                    );
                }

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_water))]
                map.render_water(&mut water_instruction, animation_timer_ms);

                #[cfg(feature = "debug")]
                if render_options.show_bounding_boxes {
                    let culling_camera: &dyn Camera = match currently_playing {
                        true => &self.player_camera,
                        false => &self.start_camera,
                    };

                    let object_set = map.cull_objects_with_frustum(
                        culling_camera,
                        &mut self.bounding_box_object_set_buffer,
                        #[cfg(feature = "debug")]
                        render_options.frustum_culling,
                    );

                    map.render_bounding(&mut self.aabb_instructions, render_options.frustum_culling, &object_set);
                }
            }

            //  Sprites and Interface
            {
                #[cfg(feature = "debug")]
                if let Some(marker_identifier) = hovered_marker_identifier {
                    map.render_marker_overlay(
                        &mut self.aabb_instructions,
                        &mut self.circle_instructions,
                        current_camera,
                        marker_identifier,
                        &point_light_set,
                        animation_timer_ms,
                    );
                }

                self.particle_holder.render(
                    &self.bottom_interface_renderer,
                    current_camera,
                    screen_size,
                    scaling,
                    self.client_state.follow(client_state().entities()),
                );

                self.effect_holder.render(&mut self.effect_renderer, current_camera);

                if let Some(player) = self.client_state.try_follow(this_entity()) {
                    #[cfg(feature = "debug")]
                    profile_block!("render player status");

                    player.render_status(
                        &self.middle_interface_renderer,
                        current_camera,
                        self.client_state.follow(client_state().world_theme()),
                        screen_size,
                    );
                }

                let mouse_mode = self.interface.get_mouse_mode();
                let is_mouse_mode_default = mouse_mode.is_default();
                let last_walking_destination = mouse_mode.walk_destination();

                let mut interface_frame = {
                    #[cfg(feature = "debug")]
                    profile_block!("user interface");

                    let is_rotating_camera = mouse_mode.is_rotating_camera();
                    let is_chat_open = self.interface.is_window_with_class_open(WindowClass::Chat);

                    let mut interface_frame =
                        self.interface
                            .lay_out_windows(&self.client_state, scaling.get_factor(), input_report.mouse_position);

                    // We can only decide what to do with the user input once we know if the mouse
                    // is hovering a window, so we buffer any actions for the next frame.

                    let is_interface_hovered = interface_frame.is_interface_hovered();

                    let cursor_state = match input_report.mouse_target {
                        _ if is_rotating_camera => MouseCursorState::RotateCamera,
                        PickerTarget::Entity(entity_id) if !is_interface_hovered => self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| match entity.get_entity_type() {
                                EntityType::Npc => MouseCursorState::Dialog,
                                EntityType::Warp => MouseCursorState::Warp,
                                EntityType::Monster => MouseCursorState::Attack,
                                _ => MouseCursorState::Default,
                            })
                            .unwrap_or(MouseCursorState::Default),
                        _ => MouseCursorState::Default,
                    };
                    self.mouse_cursor.set_state(cursor_state, client_tick);

                    if let Some(mouse_button) = input_report.mouse_click {
                        if is_interface_hovered {
                            interface_frame.click(&self.client_state, mouse_button);
                        } else {
                            interface_frame.unfocus();

                            if mouse_button == MouseButton::Left {
                                match input_report.mouse_target {
                                    PickerTarget::Nothing => {}
                                    PickerTarget::Entity(entity_id) => {
                                        self.input_event_buffer.push(InputEvent::PlayerInteract { entity_id })
                                    }
                                    PickerTarget::Tile { x, y } => {
                                        let destination = TilePosition { x, y };

                                        interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });

                                        self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                                    }
                                    #[cfg(feature = "debug")]
                                    PickerTarget::Marker(marker_identifier) => {
                                        self.input_event_buffer.push(InputEvent::OpenMarkerDetails { marker_identifier })
                                    }
                                }
                            } else if mouse_button == MouseButton::Right && currently_playing {
                                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                                interface_frame.set_mouse_mode(MouseInputMode::RotateCamera);
                            } else if mouse_button == MouseButton::DoubleRight && currently_playing {
                                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                                self.input_event_buffer.push(InputEvent::ResetCameraRotation);
                            }
                        }
                    } else if let Some(last_destination) = last_walking_destination
                        && let PickerTarget::Tile { x, y } = input_report.mouse_target
                        && input_report.left_mouse_button_down
                    {
                        let destination = TilePosition { x, y };

                        if last_destination != destination {
                            interface_frame.set_mouse_mode(MouseInputMode::Walk { destination });
                            self.input_event_buffer.push(InputEvent::PlayerMove { destination });
                        }
                    }

                    if input_report.mouse_button_released {
                        interface_frame.drop(&self.client_state);
                    }

                    if let Some(delta) = input_report.scroll {
                        if is_interface_hovered {
                            interface_frame.scroll(&self.client_state, delta);
                        } else {
                            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(!render_options.use_debug_camera))]
                            self.input_event_buffer.push(InputEvent::ZoomCamera { zoom_factor: delta });
                        }
                    }

                    // Focus the chat if the interface is not focused, no other element is capturing
                    // the keyboard input, enter was pressed, and the chat
                    // window is open.
                    if (!interface_has_focus || !interface_frame.input_characters(&self.client_state, &input_report.characters))
                        && input_report.characters.contains(&'\x0d')
                        && is_chat_open
                    {
                        interface_frame.focus_element(ChatTextBox);
                    }

                    interface_frame
                };

                let buffered_attack_entity = *self.client_state.follow(client_state().buffered_attack_entity());

                if let Some(entity_id) = buffered_attack_entity
                    && let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                {
                    entity.render_status(
                        &self.middle_interface_renderer,
                        current_camera,
                        self.client_state.follow(client_state().world_theme()),
                        screen_size,
                    );
                }

                match input_report.mouse_target {
                    PickerTarget::Tile { x, y } => {
                        // Only show if the mouse mode is default or walking.
                        if currently_playing
                            && !interface_frame.is_interface_hovered()
                            && (is_mouse_mode_default || last_walking_destination.is_some())
                        {
                            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_options.show_indicators))]
                            map.render_walk_indicator(&mut indicator_instruction, walk_indicator_color, TilePosition { x, y });
                        }
                    }
                    PickerTarget::Entity(entity_id) => {
                        if !interface_frame.is_interface_hovered() && is_mouse_mode_default {
                            let entity = self
                                .client_state
                                .follow(client_state().entities())
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                // Since the buffered attack entity will render its status anyway,
                                // we make sure not to render it here again if it's the same.
                                if !buffered_attack_entity.is_some_and(|id| id == entity_id) {
                                    entity.render_status(
                                        &self.middle_interface_renderer,
                                        current_camera,
                                        self.client_state.follow(client_state().world_theme()),
                                        screen_size,
                                    );
                                }

                                if let Some(name) = &entity.get_details() {
                                    let name = name.split('#').next().unwrap();

                                    let offset = ScreenPosition {
                                        left: 15.0 * scaling.get_factor(),
                                        top: 15.0 * scaling.get_factor(),
                                    };

                                    self.middle_interface_renderer.render_text(
                                        name,
                                        input_report.mouse_position + offset,
                                        Color::WHITE,
                                        FontSize(16.0),
                                        AlignHorizontal::Mid,
                                    );
                                }
                            }
                        }
                    }
                    _ => {}
                }

                let in_game_theme_path = client_state().in_game_theme().tooltip();
                let menu_theme_path = client_state().menu_theme().tooltip();
                let tooltip_theme = match currently_playing {
                    true => self.client_state.get(&in_game_theme_path),
                    false => self.client_state.get(&menu_theme_path),
                };

                interface_frame.render(
                    &self.client_state,
                    &self.interface_renderer,
                    tooltip_theme,
                    input_report.mouse_position,
                );

                drop(interface_frame);

                if let Some(delta) = input_report.drag {
                    // TODO: The scaling should be removed here.
                    self.interface.handle_drag(delta, scaling.get_factor());
                }

                #[cfg(feature = "debug")]
                if render_options.show_frames_per_second {
                    let world_theme = self.client_state.follow(client_state().world_theme());

                    self.top_interface_renderer.render_text(
                        &self.game_timer.last_frames_per_second().to_string(),
                        world_theme.overlay.text_offset,
                        world_theme.overlay.foreground_color,
                        world_theme.overlay.font_size,
                        AlignHorizontal::Left,
                    );
                }

                if self.show_interface {
                    self.mouse_cursor.render(
                        &self.top_interface_renderer,
                        input_report.mouse_position,
                        self.interface.get_mouse_mode().grabbed(),
                        *self.client_state.follow(client_state().world_theme().cursor().color()),
                        self.client_state.follow(client_state().interface_settings().scaling()).get_factor(),
                    );
                }
            }

            #[cfg(feature = "debug")]
            collect_instructions_measurement.stop();

            #[cfg(feature = "debug")]
            let render_frame_measurement = Profiler::start_measurement("prepare next frame");

            let interface_instructions = self.interface_renderer.get_instructions();
            let bottom_layer_instructions = self.bottom_interface_renderer.get_instructions();
            let middle_layer_instructions = self.middle_interface_renderer.get_instructions();
            let top_layer_instructions = self.top_interface_renderer.get_instructions();

            let render_instruction = RenderInstruction {
                show_interface: self.show_interface,
                picker_position,
                uniforms: Uniforms {
                    view_matrix,
                    projection_matrix,
                    camera_position,
                    animation_timer_ms,
                    ambient_light_color,
                    enhanced_lighting: lighting_mode == LightingMode::Enhanced,
                    shadow_method,
                    shadow_detail,
                    use_sdsm,
                    sdsm_enabled,
                },
                indicator: indicator_instruction,
                interface: interface_instructions.as_slice(),
                bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
                middle_layer_rectangles: middle_layer_instructions.as_slice(),
                top_layer_rectangles: top_layer_instructions.as_slice(),
                directional_light: DirectionalLightInstruction {
                    view_projection_matrix: directional_light_view_projection_matrix,
                    direction: directional_light_direction,
                    color: directional_light_color,
                },
                directional_light_partitions: &self.directional_shadow_camera.get_partition_instructions(),
                point_light: &self.point_light_instructions,
                point_light_with_shadows: &self.point_light_with_shadow_instructions,
                model_batches: &self.model_batches,
                models: &mut self.model_instructions,
                entities: &mut self.entity_instructions,
                directional_shadow_model_batches: &self.directional_shadow_model_batches,
                directional_shadow_models: &self.directional_shadow_model_instructions,
                directional_shadow_entities: &mut self.directional_shadow_entity_instructions,
                point_shadow_models: &self.point_shadow_model_instructions,
                point_shadow_entities: &self.point_shadow_entity_instructions,
                effects: self.effect_renderer.get_instructions(),
                water: water_instruction,
                map_picker_tile_vertex_buffer: Some(map.get_tile_picker_vertex_buffer()),
                map_picker_tile_index_buffer: Some(map.get_tile_picker_index_buffer()),
                font_map_texture: Some(self.font_loader.get_font_map()),
                #[cfg(feature = "debug")]
                render_options,
                #[cfg(feature = "debug")]
                aabb: &self.aabb_instructions,
                #[cfg(feature = "debug")]
                circles: &self.circle_instructions,
                #[cfg(feature = "debug")]
                rectangles: &self.rectangle_instructions,
                #[cfg(feature = "debug")]
                marker: self.debug_marker_renderer.get_instructions(),
            };

            self.graphics_engine.render_next_frame(frame, render_instruction);

            #[cfg(feature = "debug")]
            render_frame_measurement.stop();
        } else {
            #[cfg(feature = "debug")]
            let render_frame_measurement = Profiler::start_measurement("prepare next frame");

            self.graphics_engine.render_next_frame(frame, RenderInstruction::default());

            #[cfg(feature = "debug")]
            render_frame_measurement.stop();
        }

        // Apply the game state after all the UI work + rendering is done.
        self.client_state.apply();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_settings(&mut self) {
        let graphics_settings = self.client_state.follow(client_state().graphics_settings());

        if self.active_graphics_settings.vsync != graphics_settings.vsync {
            self.graphics_engine.set_vsync(graphics_settings.vsync);
            self.active_graphics_settings.vsync = graphics_settings.vsync;
        }

        if self.active_graphics_settings.limit_framerate != graphics_settings.limit_framerate {
            self.graphics_engine.set_limit_framerate(graphics_settings.limit_framerate);
            self.active_graphics_settings.limit_framerate = graphics_settings.limit_framerate;
        }

        if self.active_graphics_settings.triple_buffering != graphics_settings.triple_buffering {
            self.graphics_engine.set_triple_buffering(graphics_settings.triple_buffering);
            self.active_graphics_settings.triple_buffering = graphics_settings.triple_buffering;
        }

        if self.active_graphics_settings.texture_filtering != graphics_settings.texture_filtering {
            self.graphics_engine.set_texture_sampler_type(graphics_settings.texture_filtering);
            self.active_graphics_settings.texture_filtering = graphics_settings.texture_filtering;
        }

        if self.active_graphics_settings.msaa != graphics_settings.msaa {
            self.graphics_engine.set_msaa(graphics_settings.msaa);
            self.active_graphics_settings.msaa = graphics_settings.msaa;
        }

        if self.active_graphics_settings.ssaa != graphics_settings.ssaa {
            self.graphics_engine.set_ssaa(graphics_settings.ssaa);
            self.active_graphics_settings.ssaa = graphics_settings.ssaa;
        }

        if self.active_graphics_settings.screen_space_anti_aliasing != graphics_settings.screen_space_anti_aliasing {
            self.graphics_engine
                .set_screen_space_anti_aliasing(graphics_settings.screen_space_anti_aliasing);
            self.active_graphics_settings.screen_space_anti_aliasing = graphics_settings.screen_space_anti_aliasing;
        }

        if self.active_graphics_settings.shadow_resolution != graphics_settings.shadow_resolution {
            self.graphics_engine.set_shadow_resolution(graphics_settings.shadow_resolution);
            self.active_graphics_settings.shadow_resolution = graphics_settings.shadow_resolution;
        }

        if self.active_graphics_settings.high_quality_interface != graphics_settings.high_quality_interface {
            self.interface_renderer
                .update_high_quality_interface(graphics_settings.high_quality_interface);
            self.graphics_engine
                .set_high_quality_interface(graphics_settings.high_quality_interface);
            self.active_graphics_settings.high_quality_interface = graphics_settings.high_quality_interface;
        }

        let language = *self.client_state.follow(client_state().interface_settings().language());

        if self.active_interface_settings.language != language {
            *self.client_state.follow_mut(client_state().localization()) = Localization::load_language(&self.game_file_loader, language);
            self.active_interface_settings.language = language;
        }

        let interface_settings = self.client_state.follow_mut(client_state().interface_settings());

        if self.active_interface_settings.menu_theme != interface_settings.menu_theme {
            let menu_theme = interface_settings.menu_theme.clone();
            let theme = InterfaceTheme::load(state::theme::InterfaceThemeType::Menu, &menu_theme);
            *self.client_state.follow_mut(client_state().menu_theme()) = theme;
            self.active_interface_settings.menu_theme = menu_theme;
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.in_game_theme != interface_settings.in_game_theme {
            let in_game_theme = interface_settings.in_game_theme.clone();
            let theme = InterfaceTheme::load(InterfaceThemeType::InGame, &in_game_theme);
            *self.client_state.follow_mut(client_state().in_game_theme()) = theme;
            self.active_interface_settings.in_game_theme = in_game_theme;
        }

        let interface_settings = self.client_state.follow(client_state().interface_settings());

        if self.active_interface_settings.world_theme != interface_settings.world_theme {
            let world_theme = interface_settings.world_theme.clone();
            let theme = WorldTheme::load(&world_theme);
            *self.client_state.follow_mut(client_state().world_theme()) = theme;
            self.active_interface_settings.world_theme = world_theme;
        }
    }
}

impl ApplicationHandler for Client {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // To be as portable as possible, winit recommends to initialize the window and
        // graphics backend after the first resume event is received.
        if self.window.is_none() {
            time_phase!("create window", {
                let reader = ImageReader::with_format(Cursor::new(ICON_DATA), ImageFormat::Png);
                let image_buffer = reader.decode().unwrap().to_rgba8();
                let image_data = image_buffer.as_bytes().to_vec();

                assert_eq!(image_buffer.width(), image_buffer.height(), "icon must be square");
                let icon = Icon::from_rgba(image_data, image_buffer.width(), image_buffer.height()).unwrap();

                let window_attributes = Window::default_attributes()
                    .with_inner_size(LogicalSize {
                        width: INITIAL_SCREEN_SIZE.width,
                        height: INITIAL_SCREEN_SIZE.height,
                    })
                    .with_title(CLIENT_NAME)
                    .with_window_icon(Some(icon))
                    .with_visible(false);
                let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

                let backend_name = self.graphics_engine.get_backend_name();
                window.set_title(&format!("{CLIENT_NAME} ({})", str::to_uppercase(&backend_name)));
                window.set_cursor_visible(false);

                self.window = Some(window);

                #[cfg(feature = "debug")]
                print_debug!("created {}", "window".magenta());
            });
        }

        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if let Some(window) = self.window.as_ref() {
            let path = client_state().graphics_settings();
            let graphics_settings = self.client_state.follow(path);

            self.graphics_engine.on_resume(
                window.clone(),
                graphics_settings.triple_buffering,
                graphics_settings.vsync,
                graphics_settings.limit_framerate,
                graphics_settings.shadow_resolution,
                graphics_settings.texture_filtering,
                graphics_settings.msaa,
                graphics_settings.ssaa,
                graphics_settings.screen_space_anti_aliasing,
                graphics_settings.high_quality_interface,
            );

            // Update graphics settings capabilities based on the new surface.
            // We don't expect the capabilities to change on consecutive calls but we
            // can't get the present mode info when initializing the client, so
            // we do it here instead.
            self.client_state
                .follow_mut(client_state().graphics_settings_capabilities())
                .update(
                    self.graphics_engine.get_supported_msaa(),
                    self.graphics_engine.get_present_mode_info(),
                );

            window.set_visible(true);
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            self.audio_engine.mute(false);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(screen_size) => {
                let screen_size = screen_size.max(PhysicalSize::new(1, 1)).into();
                *self.client_state.follow_mut(client_state().window_size()) = screen_size;
                self.graphics_engine.on_resize(screen_size);
                self.interface.update_window_size(screen_size);
                self.interface_renderer.update_window_size(screen_size);
                self.bottom_interface_renderer.update_window_size(screen_size);
                self.middle_interface_renderer.update_window_size(screen_size);
                self.top_interface_renderer.update_window_size(screen_size);
                self.effect_renderer.update_window_size(screen_size);

                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::Focused(focused) => {
                if !focused {
                    self.input_system.reset();
                }

                if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
                    self.audio_engine.mute(!focused);
                }
            }
            WindowEvent::CursorLeft { .. } => self.mouse_cursor.hide(),
            WindowEvent::CursorEntered { .. } => self.mouse_cursor.show(),
            WindowEvent::CursorMoved { position, .. } => self.input_system.update_mouse_position(position),
            WindowEvent::MouseInput { button, state, .. } => self.input_system.update_mouse_buttons(button, state),
            WindowEvent::MouseWheel { delta, .. } => self.input_system.update_mouse_wheel(delta),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(keycode) = event.physical_key {
                    self.input_system.update_keyboard(keycode, event.state);
                }

                // TODO: NHA We should also support IME in the long term (winit::event::Ime)
                if let Some(text) = event.text
                    && event.state.is_pressed()
                {
                    for char in text.chars() {
                        self.input_system.buffer_character(char);
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if self.window.is_some() {
                    self.render_frame(event_loop);
                    self.window.as_mut().unwrap().request_redraw();
                }
            }
            _ignored => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.graphics_engine.on_suspended();

        if let Some(window) = self.window.as_ref() {
            window.set_visible(false);
        }

        if *self.client_state.follow(client_state().audio_settings().mute_on_focus_loss()) {
            self.audio_engine.mute(true);
        }
    }
}
