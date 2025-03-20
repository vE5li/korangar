#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(random)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]

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
#[macro_use]
mod interface;
mod inventory;
mod loaders;
mod renderer;
mod settings;
mod system;
mod world;

use std::io::Cursor;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};

use cgmath::{Point3, Vector2, Vector3};
#[cfg(feature = "debug")]
use graphics::RenderSettings;
use image::{EncodableLayout, ImageFormat, ImageReader};
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::{Colorize, print_debug};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_interface::Interface;
use korangar_interface::application::{Application, FocusState, FontSizeTrait, PositionTraitExt};
use korangar_interface::state::{
    MappedRemote, PlainTrackedState, Remote, TrackedState, TrackedStateExt, TrackedStateTake, TrackedStateVec, ValueState,
};
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SellItem,
    ShopItem,
};
use korangar_util::pathing::PathFinder;
#[cfg(not(feature = "debug"))]
use ragnarok_packets::handler::NoPacketCallback;
use ragnarok_packets::{
    BuyShopItemsResult, CharacterId, CharacterInformation, CharacterServerInformation, Direction, DisappearanceReason, Friend, HotbarSlot,
    SellItemsResult, SkillId, SkillType, TilePosition, UnitId, WorldPosition,
};
use renderer::InterfaceRenderer;
use settings::AudioSettings;
use wgpu::util::initialize_adapter_from_env_or_default;
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, Dx12BackendOptions, Dx12Compiler, GlBackendOptions, Gles3MinorVersion, Instance,
    InstanceDescriptor, InstanceFlags, MemoryHints,
};
#[cfg(feature = "debug")]
use wgpu::{Device, Queue};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Icon, Window, WindowId};

use crate::graphics::*;
use crate::input::{InputSystem, UserEvent};
use crate::interface::application::InterfaceSettings;
use crate::interface::cursor::{MouseCursor, MouseCursorState};
use crate::interface::dialog::DialogSystem;
#[cfg(feature = "debug")]
use crate::interface::elements::PacketHistoryCallback;
use crate::interface::layout::{ScreenPosition, ScreenSize};
use crate::interface::linked::LinkedElement;
use crate::interface::resource::{ItemSource, Move, SkillSource};
use crate::interface::windows::*;
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::*;
#[cfg(feature = "debug")]
use crate::renderer::DebugMarkerRenderer;
use crate::renderer::{AlignHorizontal, EffectRenderer, GameInterfaceRenderer};
use crate::settings::{GraphicsSettings, LightingMode};
use crate::system::GameTimer;
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

static ICON_DATA: &[u8] = include_bytes!("../archive/data/icon.png");

/// CTR+C was sent, and the client is supposed to close.
pub static SHUTDOWN_SIGNAL: LazyLock<AtomicBool> = LazyLock::new(|| AtomicBool::new(false));

// Create the `threads` module.
#[cfg(feature = "debug")]
korangar_debug::create_profiler_threads!(threads, {
    Main,
    Loader,
});

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
    action_loader: Arc<ActionLoader>,
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
    directional_shadow_model_batches: Vec<ModelBatch>,
    directional_shadow_model_instructions: Vec<ModelInstruction>,
    directional_shadow_entity_instructions: Vec<EntityInstruction>,
    point_shadow_model_batches: Vec<ModelBatch>,
    point_shadow_model_instructions: Vec<ModelInstruction>,
    point_shadow_entity_instructions: Vec<EntityInstruction>,
    point_light_with_shadow_instructions: Vec<PointShadowCasterInstruction>,
    point_light_instructions: Vec<PointLightInstruction>,

    input_system: InputSystem,
    lighting_mode: MappedRemote<GraphicsSettings, LightingMode>,
    vsync: MappedRemote<GraphicsSettings, bool>,
    limit_framerate: MappedRemote<GraphicsSettings, LimitFramerate>,
    triple_buffering: MappedRemote<GraphicsSettings, bool>,
    texture_filtering: MappedRemote<GraphicsSettings, TextureSamplerType>,
    shadow_detail: MappedRemote<GraphicsSettings, ShadowDetail>,
    shadow_quality: MappedRemote<GraphicsSettings, ShadowQuality>,
    msaa: MappedRemote<GraphicsSettings, Msaa>,
    ssaa: MappedRemote<GraphicsSettings, Ssaa>,
    screen_space_anti_aliasing: MappedRemote<GraphicsSettings, ScreenSpaceAntiAliasing>,
    high_quality_interface: MappedRemote<GraphicsSettings, bool>,
    #[cfg(feature = "debug")]
    render_settings: PlainTrackedState<RenderSettings>,
    mute_on_focus_loss: MappedRemote<AudioSettings, bool>,

    application: InterfaceSettings,
    interface: Interface<InterfaceSettings>,
    focus_state: FocusState<InterfaceSettings>,
    mouse_cursor: MouseCursor,
    dialog_system: DialogSystem,
    show_interface: bool,
    game_timer: GameTimer,

    #[cfg(feature = "debug")]
    debug_camera: DebugCamera,
    start_camera: StartCamera,
    player_camera: PlayerCamera,
    directional_shadow_camera: DirectionalShadowCamera,
    point_shadow_camera: PointShadowCamera,

    network_event_buffer: NetworkEventBuffer,
    client_info: ClientInfo,
    friend_list: PlainTrackedState<Vec<(Friend, LinkedElement)>>,
    saved_login_data: Option<LoginServerLoginData>,
    saved_character_server: Option<CharacterServerInformation>,
    saved_characters: PlainTrackedState<Vec<CharacterInformation>>,
    shop_items: PlainTrackedState<Vec<ShopItem<ResourceMetadata>>>,
    sell_items: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>>,
    currently_deleting: Option<CharacterId>,
    saved_player_name: String,
    move_request: PlainTrackedState<Option<usize>>,
    saved_login_server_address: Option<SocketAddr>,
    saved_password: String,
    saved_username: String,
    saved_slot_count: usize,

    particle_holder: ParticleHolder,
    point_light_manager: PointLightManager,
    effect_holder: EffectHolder,
    entities: Vec<Entity>,
    player_inventory: Inventory,
    player_skill_tree: SkillTree,
    hotbar: Hotbar,
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

    chat_messages: PlainTrackedState<Vec<ChatMessage>>,
    main_menu_click_sound_effect: SoundEffectKey,

    map: Option<Box<Map>>,

    #[cfg(feature = "debug")]
    packet_history_callback: PacketHistoryCallback,
    #[cfg(feature = "debug")]
    networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    networking_system: NetworkingSystem<NoPacketCallback>,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    graphics_engine: GraphicsEngine,
    #[cfg(feature = "debug")]
    queue: Arc<Queue>,
    #[cfg(feature = "debug")]
    device: Arc<Device>,
    window: Option<Arc<Window>>,
}

impl Client {
    fn init(sync_cache: bool) -> Option<Self> {
        time_phase!("load settings", {
            let picker_value = Arc::new(AtomicU64::new(0));
            let input_system = InputSystem::new(picker_value.clone());
            let graphics_settings = PlainTrackedState::new(GraphicsSettings::new());
            let application = InterfaceSettings::load_or_default();

            let lighting_mode = graphics_settings.mapped(|settings| &settings.lighting_mode).new_remote();
            let vsync = graphics_settings.mapped(|settings| &settings.vsync).new_remote();
            let limit_framerate = graphics_settings.mapped(|settings| &settings.limit_framerate).new_remote();
            let triple_buffering = graphics_settings.mapped(|settings| &settings.triple_buffering).new_remote();
            let texture_filtering = graphics_settings.mapped(|settings| &settings.texture_filtering).new_remote();
            let shadow_detail = graphics_settings.mapped(|settings| &settings.shadow_detail).new_remote();
            let shadow_quality = graphics_settings.mapped(|settings| &settings.shadow_quality).new_remote();
            let msaa = graphics_settings.mapped(|settings| &settings.msaa).new_remote();
            let ssaa = graphics_settings.mapped(|settings| &settings.ssaa).new_remote();
            let screen_space_anti_aliasing = graphics_settings
                .mapped(|settings| &settings.screen_space_anti_aliasing)
                .new_remote();
            let high_quality_interface = graphics_settings.mapped(|settings| &settings.high_quality_interface).new_remote();

            #[cfg(feature = "debug")]
            let render_settings = PlainTrackedState::new(RenderSettings::new());
        });

        time_phase!("create adapter", {
            let instance = Instance::new(&InstanceDescriptor {
                backends: Backends::all().with_env(),
                flags: InstanceFlags::from_build_config().with_env(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::Automatic.with_env(),
                    },
                    dx12: Dx12BackendOptions {
                        shader_compiler: Dx12Compiler::StaticDxc.with_env(),
                    },
                },
            });

            let adapter = pollster::block_on(async { initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });
            let adapter = Arc::new(adapter);

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
                    .request_device(
                        &DeviceDescriptor {
                            label: None,
                            required_features: capabilities.get_required_features(),
                            required_limits: capabilities.get_required_limits(),
                            memory_hints: MemoryHints::Performance,
                        },
                        std::env::var("WGPU_TRACE").ok().as_ref().map(std::path::Path::new),
                    )
                    .await
                    .unwrap()
            });
            let device = Arc::new(device);
            let queue = Arc::new(queue);

            #[cfg(feature = "debug")]
            device.on_uncaptured_error(Box::new(error_handler));

            #[cfg(feature = "debug")]
            print_debug!("received {} and {}", "queue".magenta(), "device".magenta());
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
            let audio_settings = PlainTrackedState::new(AudioSettings::new());
            let mute_on_focus_loss = audio_settings.mapped(|settings| &settings.mute_on_focus_loss).new_remote();

            let audio_engine = Arc::new(AudioEngine::new(game_file_loader.clone()));
            audio_engine.set_background_music_volume(0.1);
        });

        time_phase!("create resource managers", {
            std::fs::create_dir_all("client/themes").unwrap();

            let model_loader = Arc::new(ModelLoader::new(game_file_loader.clone(), capabilities.bindless_support()));
            let texture_loader = Arc::new(TextureLoader::new(
                device.clone(),
                queue.clone(),
                &capabilities,
                game_file_loader.clone(),
            ));
            let font_loader = Arc::new(FontLoader::new(application.get_fonts(), &game_file_loader, &texture_loader));
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
                library.clone(),
            ));

            let interface_renderer = InterfaceRenderer::new(
                INITIAL_SCREEN_SIZE,
                font_loader.clone(),
                &texture_loader,
                *high_quality_interface.get(),
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
            let directional_shadow_model_batches = Vec::default();
            let directional_shadow_model_instructions = Vec::default();
            let directional_shadow_entity_instructions = Vec::default();
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
                texture_loader: texture_loader.clone(),
                picker_value,
            });
        });

        time_phase!("initialize interface", {
            let mut interface = Interface::new(INITIAL_SCREEN_SIZE);
            let mut focus_state = FocusState::default();
            let mouse_cursor = MouseCursor::new(&sprite_loader, &action_loader);
            let dialog_system = DialogSystem::default();
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
            directional_shadow_camera.set_focus_point(start_camera.focus_point(), start_camera.view_direction());
        });

        time_phase!("initialize networking", {
            let client_info = load_client_info(&game_file_loader);

            #[cfg(not(feature = "debug"))]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn();
            #[cfg(feature = "debug")]
            let packet_history_callback = PacketHistoryCallback::get_static_instance();
            #[cfg(feature = "debug")]
            let (networking_system, network_event_buffer) = NetworkingSystem::spawn_with_callback(packet_history_callback.clone());

            let friend_list: PlainTrackedState<Vec<(Friend, LinkedElement)>> = PlainTrackedState::default();
            let saved_login_data: Option<LoginServerLoginData> = None;
            let saved_character_server: Option<CharacterServerInformation> = None;
            let saved_characters: PlainTrackedState<Vec<CharacterInformation>> = PlainTrackedState::default();
            let shop_items: PlainTrackedState<Vec<ShopItem<ResourceMetadata>>> = PlainTrackedState::default();
            let sell_items: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>> = PlainTrackedState::default();
            let currently_deleting: Option<CharacterId> = None;
            let saved_player_name = String::new();
            let move_request: PlainTrackedState<Option<usize>> = PlainTrackedState::default();
            let saved_login_server_address = None;
            let saved_password = String::new();
            let saved_username = String::new();
            let saved_slot_count = 0;

            interface.open_window(&application, &mut focus_state, &LoginWindow::new(&client_info));
        });

        time_phase!("create resources", {
            let particle_holder = ParticleHolder::default();
            let point_light_manager = PointLightManager::new();
            let effect_holder = EffectHolder::default();
            let entities = Vec::<Entity>::new();
            let player_inventory = Inventory::default();
            let player_skill_tree = SkillTree::default();
            let hotbar = Hotbar::default();
            let path_finder = PathFinder::default();

            let point_light_set_buffer = ResourceSetBuffer::default();
            let directional_shadow_object_set_buffer = ResourceSetBuffer::default();
            let point_shadow_object_set_buffer = ResourceSetBuffer::default();
            let deferred_object_set_buffer = ResourceSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ResourceSetBuffer::default();

            #[cfg(feature = "debug")]
            let pathing_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), "pathing", &[
                "pathing_goal.png",
                "pathing_straight.png",
                "pathing_diagonal.png",
            ]);
            #[cfg(feature = "debug")]
            let pathing_texture_set = Arc::new(pathing_texture_set);

            #[cfg(feature = "debug")]
            let tile_texture_set = TextureSetBuilder::build_from_group(texture_loader.clone(), "tile", &[
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

            let welcome_string = format!(
                "Welcome to ^ff8800Korangar^000000 version ^ff8800{}^000000!",
                env!("CARGO_PKG_VERSION")
            );
            let chat_messages = PlainTrackedState::new(vec![ChatMessage {
                text: welcome_string,
                color: MessageColor::Server,
            }]);

            let main_menu_click_sound_effect = audio_engine.load(MAIN_MENU_CLICK_SOUND_EFFECT);
        });

        time_phase!("load default map", {
            let map = map_loader
                .load(DEFAULT_MAP.to_string(), &model_loader, texture_loader.clone(), &library)
                .expect("failed to load initial map");

            audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);
            map.set_ambient_sound_sources(&audio_engine);
        });

        Some(Self {
            action_loader,
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
            lighting_mode,
            vsync,
            limit_framerate,
            triple_buffering,
            texture_filtering,
            shadow_detail,
            shadow_quality,
            msaa,
            ssaa,
            screen_space_anti_aliasing,
            high_quality_interface,
            #[cfg(feature = "debug")]
            render_settings,
            mute_on_focus_loss,
            application,
            interface,
            focus_state,
            mouse_cursor,
            dialog_system,
            show_interface,
            game_timer,
            #[cfg(feature = "debug")]
            debug_camera,
            start_camera,
            player_camera,
            directional_shadow_camera,
            point_shadow_camera,
            network_event_buffer,
            client_info,
            friend_list,
            saved_login_data,
            saved_character_server,
            saved_characters,
            shop_items,
            sell_items,
            currently_deleting,
            saved_player_name,
            move_request,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_slot_count,
            particle_holder,
            point_light_manager,
            effect_holder,
            entities,
            player_inventory,
            player_skill_tree,
            hotbar,
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
            chat_messages,
            main_menu_click_sound_effect,
            map: Some(map),
            #[cfg(feature = "debug")]
            packet_history_callback,
            networking_system,
            audio_engine,
            graphics_engine,
            #[cfg(feature = "debug")]
            queue,
            #[cfg(feature = "debug")]
            device,
            window: None,
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
        self.directional_shadow_model_batches.clear();
        self.directional_shadow_model_instructions.clear();
        self.directional_shadow_entity_instructions.clear();
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
        self.update_graphic_settings();

        // TODO: NHA We want to have an event or a remote setting that the scaling
        //       changed.
        let scaling = self.application.get_scaling();
        self.bottom_interface_renderer.update_scaling(scaling);
        self.middle_interface_renderer.update_scaling(scaling);
        self.top_interface_renderer.update_scaling(scaling);

        let frame = self.graphics_engine.wait_for_next_frame();

        #[cfg(feature = "debug")]
        let timer_measurement = Profiler::start_measurement("update timers");

        self.input_system.update_delta();

        let delta_time = self.game_timer.update();
        let day_timer = self.game_timer.get_day_timer();
        let animation_timer = self.game_timer.get_animation_timer();
        let client_tick = self.game_timer.get_client_tick();

        #[cfg(feature = "debug")]
        timer_measurement.stop();

        self.networking_system.get_events(&mut self.network_event_buffer);

        let (user_events, hovered_element, focused_element, mouse_target, mouse_position) = self.input_system.user_events(
            &mut self.interface,
            &self.application,
            &mut self.focus_state,
            &mut self.mouse_cursor,
            #[cfg(feature = "debug")]
            &self.render_settings,
            client_tick,
        );

        #[cfg(feature = "debug")]
        let picker_measurement = Profiler::start_measurement("update picker target");

        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
            if let Some(entity) = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                if entity.are_details_unavailable() && self.networking_system.entity_details(entity_id).is_ok() {
                    entity.set_details_requested();
                }

                match entity.get_entity_type() {
                    EntityType::Npc => self.mouse_cursor.set_state(MouseCursorState::Dialog, client_tick),
                    EntityType::Warp => self.mouse_cursor.set_state(MouseCursorState::Warp, client_tick),
                    EntityType::Monster => self.mouse_cursor.set_state(MouseCursorState::Attack, client_tick),
                    _ => {}
                }
            }
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

                    self.interface.close_all_windows_except(&mut self.focus_state);
                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &SelectServerWindow::new(character_servers),
                    );
                }
                NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_login_server();

                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::LoginServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let socket_address = self.saved_login_server_address.unwrap();
                        self.networking_system
                            .connect_to_login_server(socket_address, &self.saved_username, &self.saved_password);
                    }
                }
                NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                    self.saved_slot_count = normal_slot_count;
                    let _ = self.networking_system.request_character_list();
                }
                NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                    self.networking_system.disconnect_from_character_server();
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterServerDisconnected { reason } => {
                    if reason != DisconnectReason::ClosedByClient {
                        // TODO: Make this an on-screen popup.
                        #[cfg(feature = "debug")]
                        print_debug!("Disconnection from the character server with error");

                        let login_data = self.saved_login_data.as_ref().unwrap();
                        let server = self.saved_character_server.clone().unwrap();
                        self.networking_system.connect_to_character_server(login_data, server);
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
                    self.networking_system.connect_to_character_server(login_data, server);

                    self.map = None;
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    self.entities.clear();

                    self.audio_engine.play_background_music_track(None);

                    self.interface.close_all_windows_except(&mut self.focus_state);

                    self.async_loader
                        .request_map_load(DEFAULT_MAP.to_string(), Some(TilePosition::new(0, 0)));
                }
                NetworkEvent::ResurrectPlayer { entity_id } => {
                    // If the resurrected player is us, close the resurrect window.
                    if self.entities[0].get_entity_id() == entity_id {
                        self.interface
                            .close_window_with_class(&mut self.focus_state, RespawnWindow::WINDOW_CLASS);
                    }
                }
                NetworkEvent::PlayerStandUp { entity_id } => {
                    if let Some(entity) = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                        entity.set_idle(client_tick);
                    }
                }
                NetworkEvent::AccountId(..) => {}
                NetworkEvent::CharacterList { characters } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    self.saved_characters.set(characters);
                    let character_selection_window = CharacterSelectionWindow::new(
                        self.saved_characters.new_remote(),
                        self.move_request.new_remote(),
                        self.saved_slot_count,
                    );

                    // TODO: this will do one unnecessary restore_focus. check if
                    // that will be problematic
                    self.interface.close_all_windows_except(&mut self.focus_state);
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &character_selection_window);
                }
                NetworkEvent::CharacterSelectionFailed { message, .. } => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &ErrorWindow::new(message.to_owned()))
                }
                NetworkEvent::CharacterDeleted => {
                    let character_id = self.currently_deleting.take().unwrap();
                    self.saved_characters.retain(|character| character.character_id != character_id);
                }
                NetworkEvent::CharacterDeletionFailed { message, .. } => {
                    self.currently_deleting = None;
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &ErrorWindow::new(message.to_owned()))
                }
                NetworkEvent::CharacterSelected { login_data, .. } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    let saved_login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system.connect_to_map_server(saved_login_data, login_data);
                    // Ask for the client tick right away, so that the player isn't de-synced when
                    // they spawn on the map.
                    let _ = self.networking_system.request_client_tick();

                    let character_information = self
                        .saved_characters
                        .get()
                        .iter()
                        .find(|character| character.character_id == login_data.character_id)
                        .cloned()
                        .unwrap();

                    self.saved_player_name = character_information.name.clone();

                    let mut player = Entity::Player(Player::new(saved_login_data.account_id, &character_information, client_tick));

                    let entity_id = player.get_entity_id();
                    let entity_type = player.get_entity_type();
                    let entity_part_files = player.get_entity_part_files(&self.library);

                    if let Some(animation_data) = self
                        .async_loader
                        .request_animation_data_load(entity_id, entity_type, entity_part_files)
                    {
                        player.set_animation_data(animation_data);
                    }

                    self.entities.push(player);

                    // TODO: This will do one unnecessary restore_focus. Check if
                    //       that will be problematic.
                    self.interface
                        .close_window_with_class(&mut self.focus_state, CharacterSelectionWindow::WINDOW_CLASS);
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &CharacterOverviewWindow::new());
                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &ChatWindow::new(self.chat_messages.new_remote(), self.font_loader.clone()),
                    );
                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &HotbarWindow::new(self.hotbar.get_skills()),
                    );

                    // Put the dialog system in a well-defined state.
                    self.dialog_system.close_dialog();

                    self.map = None;
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();
                }
                NetworkEvent::CharacterCreated { character_information } => {
                    self.saved_characters.push(character_information);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, CharacterCreationWindow::WINDOW_CLASS);
                }
                NetworkEvent::CharacterCreationFailed { message, .. } => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &ErrorWindow::new(message.to_owned()));
                }
                NetworkEvent::CharacterSlotSwitched => {}
                NetworkEvent::CharacterSlotSwitchFailed => {
                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &ErrorWindow::new("Failed to switch character slots".to_owned()),
                    );
                }
                NetworkEvent::AddEntity(entity_data) => {
                    if let Some(map) = self.map.as_ref() {
                        let mut npc = Entity::Npc(Npc::new(map, entity_data, client_tick));

                        let entity_id = npc.get_entity_id();
                        let entity_type = npc.get_entity_type();
                        let entity_part_files = npc.get_entity_part_files(&self.library);

                        // Sometimes (like after a job change) the server will tell the client
                        // that a new entity appeared, even though it was already on screen. So
                        // to prevent the entity existing twice, we remove the old one.
                        self.entities.retain(|entity| entity.get_entity_id() != entity_id);

                        if let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(entity_id, entity_type, entity_part_files)
                        {
                            npc.set_animation_data(animation_data);
                        }

                        #[cfg(feature = "debug")]
                        npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

                        self.entities.push(npc);
                    }
                }
                NetworkEvent::RemoveEntity { entity_id, reason } => {
                    //If the motive is dead, you need to set the player to dead
                    if reason == DisappearanceReason::Died {
                        if let Some(entity) = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                            let entity_type = entity.get_entity_type();

                            if entity_type == EntityType::Monster {
                                // TODO: If the entity is a monster, it will just disappear. This
                                // is not the desired behavior and should be updated at some point.
                                self.entities.retain(|entity| entity.get_entity_id() != entity_id);
                            } else if entity_type == EntityType::Player {
                                entity.set_dead(client_tick);

                                // If the player is us, we need to open the respawn window.
                                if entity_id == self.entities[0].get_entity_id() {
                                    self.interface.open_window(&self.application, &mut self.focus_state, &RespawnWindow);
                                }
                            }
                        }
                    } else {
                        self.entities.retain(|entity| entity.get_entity_id() != entity_id);
                    }
                }
                NetworkEvent::EntityMove(entity_id, position_from, position_to, starting_timestamp) => {
                    let entity = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity
                        && let Some(map) = self.map.as_ref()
                    {
                        let position_from = Vector2::new(position_from.x, position_from.y);
                        let position_to = Vector2::new(position_to.x, position_to.y);

                        entity.move_from_to(map, &mut self.path_finder, position_from, position_to, starting_timestamp);
                        #[cfg(feature = "debug")]
                        entity.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::PlayerMove(position_from, position_to, starting_timestamp) => {
                    if let Some(map) = self.map.as_ref() {
                        let position_from = Vector2::new(position_from.x, position_from.y);
                        let position_to = Vector2::new(position_to.x, position_to.y);

                        self.entities[0].move_from_to(map, &mut self.path_finder, position_from, position_to, starting_timestamp);
                        #[cfg(feature = "debug")]
                        self.entities[0].generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);
                    }
                }
                NetworkEvent::ChangeMap(map_name, player_position) => {
                    self.map = None;
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.clear_ambient_sound();

                    // Only the player must stay alive between map changes.
                    self.entities.truncate(1);

                    self.async_loader.request_map_load(map_name, Some(player_position));
                }
                NetworkEvent::UpdateClientTick { client_tick, received_at } => {
                    self.game_timer.set_client_tick(client_tick, received_at);
                }
                NetworkEvent::ChatMessage { text, color } => {
                    self.chat_messages.push(ChatMessage { text, color });
                }
                NetworkEvent::UpdateEntityDetails(entity_id, name) => {
                    let entity = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.set_details(name);
                    }
                }
                NetworkEvent::DamageEffect { entity_id, damage_amount } => {
                    let entity = self
                        .entities
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .unwrap_or(&self.entities[0]);

                    self.particle_holder
                        .spawn_particle(Box::new(DamageNumber::new(entity.get_position(), damage_amount.to_string())));
                }
                NetworkEvent::HealEffect(entity_id, damage_amount) => {
                    let entity = self
                        .entities
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .unwrap_or(&self.entities[0]);

                    self.particle_holder
                        .spawn_particle(Box::new(HealNumber::new(entity.get_position(), damage_amount.to_string())));
                }
                NetworkEvent::UpdateEntityHealth(entity_id, health_points, maximum_health_points) => {
                    let entity = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.update_health(health_points, maximum_health_points);
                    }
                }
                NetworkEvent::UpdateStatus(status_type) => {
                    let Entity::Player(player) = &mut self.entities[0] else {
                        panic!();
                    };

                    player.update_status(status_type);
                }
                NetworkEvent::OpenDialog(text, npc_id) => {
                    if let Some(dialog_window) = self.dialog_system.open_dialog_window(text, npc_id) {
                        self.interface.open_window(&self.application, &mut self.focus_state, &dialog_window);
                    }
                }
                NetworkEvent::AddNextButton => self.dialog_system.add_next_button(),
                NetworkEvent::AddCloseButton => self.dialog_system.add_close_button(),
                NetworkEvent::AddChoiceButtons(choices) => self.dialog_system.add_choice_buttons(choices),
                NetworkEvent::AddQuestEffect(quest_effect) => {
                    if let Some(map) = self.map.as_ref() {
                        self.particle_holder.add_quest_icon(&self.texture_loader, map, quest_effect)
                    }
                }
                NetworkEvent::RemoveQuestEffect(entity_id) => self.particle_holder.remove_quest_icon(entity_id),
                NetworkEvent::SetInventory { items } => {
                    self.player_inventory.fill(&self.async_loader, &self.library, items);
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.player_inventory.add_item(&self.async_loader, &self.library, item);

                    // TODO: Update the selling items. If you pick up an item
                    // that you already have the sell window
                    // should allow you to sell the new
                    // amount of items.
                }
                NetworkEvent::InventoryItemRemoved {
                    reason: _reason,
                    index,
                    amount,
                } => {
                    self.player_inventory.remove_item(index, amount);
                }
                NetworkEvent::SkillTree(skill_information) => {
                    self.player_skill_tree
                        .fill(&self.sprite_loader, &self.action_loader, skill_information, client_tick);
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.player_inventory.update_equipped_position(index, equipped_position);
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    let entity = self
                        .entities
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
                        .entities
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
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &FriendRequestWindow::new(requestee))
                }
                NetworkEvent::FriendRemoved { account_id, character_id } => {
                    self.friend_list
                        .retain(|(friend, _)| !(friend.account_id == account_id && friend.character_id == character_id));
                }
                NetworkEvent::FriendAdded { friend } => {
                    self.friend_list.push((friend, LinkedElement::new()));
                }
                NetworkEvent::VisualEffect(path, entity_id) => {
                    let effect = self.effect_loader.get_or_load(path, &self.texture_loader).unwrap();
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
                NetworkEvent::AddSkillUnit(entity_id, unit_id, position) => {
                    let Some(map) = self.map.as_ref() else { continue };

                    match unit_id {
                        UnitId::Firewall => {
                            let position = Vector2::new(position.x as usize, position.y as usize);
                            let position = map.get_world_position(position);
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
                            let position = Vector2::new(position.x as usize, position.y as usize);
                            let position = map.get_world_position(position);
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
                NetworkEvent::RemoveSkillUnit(entity_id) => {
                    self.effect_holder.remove_unit(entity_id);
                }
                NetworkEvent::SetFriendList { friends } => {
                    self.friend_list.mutate(|friend_list| {
                        *friend_list = friends.into_iter().map(|friend| (friend, LinkedElement::new())).collect();
                    });
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
                                let Some(mut skill) = self.player_skill_tree.find_skill(SkillId(hotkey.skill_id as u16)) else {
                                    self.hotbar.clear_slot(&mut self.networking_system, HotbarSlot(index as u16));
                                    continue;
                                };

                                skill.skill_level = hotkey.quantity_or_skill_level;
                                self.hotbar.set_slot(HotbarSlot(index as u16), skill);
                            }
                            HotkeyState::Unbound => self.hotbar.unset_slot(HotbarSlot(index as u16)),
                        }
                    }
                }
                NetworkEvent::OpenShop { items } => {
                    self.shop_items.mutate(|shop_items| {
                        *shop_items = items
                            .into_iter()
                            .map(|item| self.library.load_shop_item_metadata(&self.async_loader, item))
                            .collect()
                    });

                    let cart = PlainTrackedState::default();

                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &BuyWindow::new(self.shop_items.new_remote(), cart.clone()),
                    );
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &BuyCartWindow::new(cart));
                }
                NetworkEvent::AskBuyOrSell { shop_id } => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &BuyOrSellWindow::new(shop_id));
                }
                NetworkEvent::BuyingCompleted { result } => match result {
                    BuyShopItemsResult::Success => {
                        let _ = self.networking_system.close_shop();

                        self.interface
                            .close_window_with_class(&mut self.focus_state, BuyWindow::WINDOW_CLASS);
                        self.interface
                            .close_window_with_class(&mut self.focus_state, BuyCartWindow::WINDOW_CLASS);
                    }
                    BuyShopItemsResult::Error => {
                        self.chat_messages.push(ChatMessage {
                            text: "Failed to buy items".to_owned(),
                            color: MessageColor::Error,
                        });
                    }
                },
                NetworkEvent::SellItemList { items } => {
                    let inventory_items = self.player_inventory.get_items();

                    self.sell_items.mutate(|sell_items| {
                        *sell_items = items
                            .into_iter()
                            .map(|item| {
                                let inventory_item = &inventory_items
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
                            .collect()
                    });

                    let cart = PlainTrackedState::default();

                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &SellWindow::new(self.sell_items.new_remote(), cart.clone()),
                    );
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &SellCartWindow::new(cart.clone()));
                }
                NetworkEvent::SellingCompleted { result } => match result {
                    SellItemsResult::Success => {
                        self.interface
                            .close_window_with_class(&mut self.focus_state, SellWindow::WINDOW_CLASS);
                        self.interface
                            .close_window_with_class(&mut self.focus_state, SellCartWindow::WINDOW_CLASS);
                    }
                    SellItemsResult::Error => {
                        self.chat_messages.push(ChatMessage {
                            text: "Failed to sell items".to_owned(),
                            color: MessageColor::Error,
                        });
                    }
                },
            }
        }

        #[cfg(feature = "debug")]
        network_event_measurement.stop();

        #[cfg(feature = "debug")]
        let user_event_measurement = Profiler::start_measurement("process user events");

        for event in user_events {
            match event {
                UserEvent::LogIn {
                    service_id,
                    username,
                    password,
                } => {
                    let service = self
                        .client_info
                        .services
                        .iter()
                        .find(|service| service.service_id() == service_id)
                        .unwrap();
                    let address = format!("{}:{}", service.address, service.port);
                    let socket_address = address
                        .to_socket_addrs()
                        .expect("Failed to resolve IP")
                        .next()
                        .expect("ill formatted service IP");

                    self.saved_login_server_address = Some(socket_address);
                    self.saved_username = username.clone();
                    self.saved_password = password.clone();

                    self.networking_system.connect_to_login_server(socket_address, username, password);
                }
                UserEvent::SelectServer(server) => {
                    self.saved_character_server = Some(server.clone());

                    self.networking_system.disconnect_from_login_server();

                    // Korangar should never attempt to connect to the character
                    // server before it logged in to the login server, so it's fine to
                    // unwrap here.
                    let login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system.connect_to_character_server(login_data, server);
                }
                UserEvent::Respawn => {
                    let _ = self.networking_system.respawn();
                    self.interface
                        .close_window_with_class(&mut self.focus_state, RespawnWindow::WINDOW_CLASS);
                }
                UserEvent::LogOut => {
                    let _ = self.networking_system.log_out();
                }
                UserEvent::Exit => event_loop.exit(),
                UserEvent::CameraZoom(factor) => self.player_camera.soft_zoom(factor),
                UserEvent::CameraRotate(factor) => self.player_camera.soft_rotate(factor),
                UserEvent::CameraResetRotation => self.player_camera.reset_rotation(),
                UserEvent::OpenMenuWindow => {
                    if !self.entities.is_empty() {
                        self.interface.open_window(&self.application, &mut self.focus_state, &MenuWindow)
                    }
                }
                UserEvent::OpenInventoryWindow => {
                    if !self.entities.is_empty() {
                        self.interface.open_window(
                            &self.application,
                            &mut self.focus_state,
                            &InventoryWindow::new(self.player_inventory.item_remote()),
                        )
                    }
                }
                UserEvent::OpenEquipmentWindow => {
                    if !self.entities.is_empty() {
                        self.interface.open_window(
                            &self.application,
                            &mut self.focus_state,
                            &EquipmentWindow::new(self.player_inventory.item_remote()),
                        )
                    }
                }
                UserEvent::OpenSkillTreeWindow => {
                    if !self.entities.is_empty() {
                        self.interface.open_window(
                            &self.application,
                            &mut self.focus_state,
                            &SkillTreeWindow::new(self.player_skill_tree.get_skills()),
                        )
                    }
                }
                UserEvent::OpenGraphicsSettingsWindow => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    &GraphicsSettingsWindow::new(
                        self.graphics_engine.get_present_mode_info(),
                        self.graphics_engine.get_supported_msaa(),
                        self.lighting_mode.clone_state(),
                        self.vsync.clone_state(),
                        self.limit_framerate.clone_state(),
                        self.triple_buffering.clone_state(),
                        self.texture_filtering.clone_state(),
                        self.msaa.clone_state(),
                        self.ssaa.clone_state(),
                        self.screen_space_anti_aliasing.clone_state(),
                        self.shadow_detail.clone_state(),
                        self.shadow_quality.clone_state(),
                        self.high_quality_interface.clone_state(),
                    ),
                ),
                UserEvent::OpenAudioSettingsWindow => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    &AudioSettingsWindow::new(self.mute_on_focus_loss.clone_state()),
                ),
                UserEvent::OpenFriendsWindow => {
                    self.interface.open_window(
                        &self.application,
                        &mut self.focus_state,
                        &FriendsWindow::new(self.friend_list.new_remote()),
                    );
                }
                UserEvent::ToggleShowInterface => self.show_interface = !self.show_interface,
                UserEvent::SetThemeFile { theme_file, theme_kind } => self.application.set_theme_file(theme_file, theme_kind),
                UserEvent::SaveTheme { theme_kind } => self.application.save_theme(theme_kind),
                UserEvent::ReloadTheme { theme_kind } => self.application.reload_theme(theme_kind),
                UserEvent::SelectCharacter(character_slot) => {
                    let _ = self.networking_system.select_character(character_slot);
                }
                UserEvent::OpenCharacterCreationWindow(character_slot) => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    &CharacterCreationWindow::new(character_slot),
                ),
                UserEvent::CreateCharacter(character_slot, name) => {
                    let _ = self.networking_system.create_character(character_slot, name);
                }
                UserEvent::DeleteCharacter(character_id) => {
                    if self.currently_deleting.is_none() {
                        let _ = self.networking_system.delete_character(character_id);
                        self.currently_deleting = Some(character_id);
                    }
                }
                UserEvent::RequestSwitchCharacterSlot(origin_slot) => self.move_request.set(Some(origin_slot)),
                UserEvent::CancelSwitchCharacterSlot => self.move_request.set(None),
                UserEvent::SwitchCharacterSlot(destination_slot) => {
                    let _ = self
                        .networking_system
                        .switch_character_slot(self.move_request.take().unwrap(), destination_slot);
                }
                UserEvent::RequestPlayerMove(destination) => {
                    if !self.entities.is_empty() {
                        let _ = self.networking_system.player_move(WorldPosition {
                            x: destination.x,
                            y: destination.y,
                            direction: Direction::N,
                        });
                    }
                }
                UserEvent::RequestPlayerInteract(entity_id) => {
                    let entity = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        let _ = match entity.get_entity_type() {
                            EntityType::Npc => self.networking_system.start_dialog(entity_id),
                            EntityType::Monster => self.networking_system.player_attack(entity_id),
                            EntityType::Warp => self.networking_system.player_move({
                                let position = entity.get_grid_position();
                                WorldPosition {
                                    x: position.x,
                                    y: position.y,
                                    direction: Direction::N,
                                }
                            }),
                            _ => Ok(()),
                        };
                    }
                }
                UserEvent::RequestWarpToMap(map_name, position) => {
                    let _ = self.networking_system.warp_to_map(map_name, position);
                }
                UserEvent::SendMessage(message) => {
                    let _ = self.networking_system.send_chat_message(&self.saved_player_name, &message);
                    // TODO: maybe find a better solution for unfocusing the message box if
                    // this becomes problematic
                    self.focus_state.remove_focus();
                }
                UserEvent::NextDialog(npc_id) => {
                    let _ = self.networking_system.next_dialog(npc_id);
                }
                UserEvent::CloseDialog(npc_id) => {
                    let _ = self.networking_system.close_dialog(npc_id);
                    self.dialog_system.close_dialog();
                    self.interface
                        .close_window_with_class(&mut self.focus_state, DialogWindow::WINDOW_CLASS);
                }
                UserEvent::ChooseDialogOption(npc_id, option) => {
                    let _ = self.networking_system.choose_dialog_option(npc_id, option);

                    if option == -1 {
                        self.dialog_system.close_dialog();
                        self.interface
                            .close_window_with_class(&mut self.focus_state, DialogWindow::WINDOW_CLASS);
                    }
                }
                UserEvent::MoveResource(r#move) => match r#move {
                    Move::Item { source, destination, item } => match (source, destination) {
                        (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                            let _ = self.networking_system.request_item_equip(item.index, position);
                        }
                        (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                            let _ = self.networking_system.request_item_unequip(item.index);
                        }
                        _ => {}
                    },
                    Move::Skill {
                        source,
                        destination,
                        skill,
                    } => match (source, destination) {
                        (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                            self.hotbar.update_slot(&mut self.networking_system, slot, skill);
                        }
                        (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                            self.hotbar.swap_slot(&mut self.networking_system, source_slot, destination_slot);
                        }
                        _ => {}
                    },
                },
                UserEvent::CastSkill(slot) => {
                    if let Some(skill) = self.hotbar.get_skill_in_slot(slot).as_ref() {
                        match skill.skill_type {
                            SkillType::Passive => {}
                            SkillType::Attack => {
                                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                    let _ = self.networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                }
                            }
                            SkillType::Ground | SkillType::Trap => {
                                if let Some(PickerTarget::Tile { x, y }) = mouse_target {
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
                                        self.entities[0].get_entity_id(),
                                    );
                                }
                                false => {
                                    let _ = self.networking_system.cast_skill(
                                        skill.skill_id,
                                        skill.skill_level,
                                        self.entities[0].get_entity_id(),
                                    );
                                }
                            },
                            SkillType::Support => {
                                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                    let _ = self.networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                } else {
                                    let _ = self.networking_system.cast_skill(
                                        skill.skill_id,
                                        skill.skill_level,
                                        self.entities[0].get_entity_id(),
                                    );
                                }
                            }
                        }
                    }
                }
                UserEvent::StopSkill(slot) => {
                    if let Some(skill) = self.hotbar.get_skill_in_slot(slot).as_ref() {
                        if skill.skill_id == ROLLING_CUTTER_ID {
                            let _ = self.networking_system.stop_channeling_skill(skill.skill_id);
                        }
                    }
                }
                UserEvent::AddFriend(name) => {
                    if name.len() > 24 {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] friend name {} is too long", "error".red(), name.magenta());
                    } else {
                        let _ = self.networking_system.add_friend(name);
                    }
                }
                UserEvent::RemoveFriend { account_id, character_id } => {
                    let _ = self.networking_system.remove_friend(account_id, character_id);
                }
                UserEvent::RejectFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.reject_friend_request(account_id, character_id);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, FriendRequestWindow::WINDOW_CLASS);
                }
                UserEvent::AcceptFriendRequest { account_id, character_id } => {
                    let _ = self.networking_system.accept_friend_request(account_id, character_id);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, FriendRequestWindow::WINDOW_CLASS);
                }
                UserEvent::BuyItems { items } => {
                    let _ = self.networking_system.purchase_items(items);
                }
                UserEvent::CloseShop => {
                    let _ = self.networking_system.close_shop();

                    self.interface
                        .close_window_with_class(&mut self.focus_state, BuyWindow::WINDOW_CLASS);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, BuyCartWindow::WINDOW_CLASS);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, SellWindow::WINDOW_CLASS);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, SellCartWindow::WINDOW_CLASS);
                }
                UserEvent::BuyOrSell { shop_id, buy_or_sell } => {
                    let _ = self.networking_system.select_buy_or_sell(shop_id, buy_or_sell);
                    self.interface
                        .close_window_with_class(&mut self.focus_state, BuyOrSellWindow::WINDOW_CLASS);
                }
                UserEvent::SellItems { items } => {
                    let _ = self.networking_system.sell_items(items);
                }
                UserEvent::FocusChatWindow => {
                    self.interface
                        .focus_window_with_class(&mut self.focus_state, ChatWindow::WINDOW_CLASS);
                }
                #[cfg(feature = "debug")]
                UserEvent::OpenMarkerDetails(marker_identifier) => {
                    if let Some(map) = self.map.as_ref() {
                        self.interface.open_window(
                            &self.application,
                            &mut self.focus_state,
                            map.resolve_marker(&self.entities, marker_identifier),
                        );
                    }
                }
                #[cfg(feature = "debug")]
                UserEvent::OpenRenderSettingsWindow => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    &RenderSettingsWindow::new(self.render_settings.clone()),
                ),
                #[cfg(feature = "debug")]
                UserEvent::OpenMapDataWindow => {
                    if let Some(map) = self.map.as_ref() {
                        self.interface
                            .open_window(&self.application, &mut self.focus_state, map.to_prototype_window());
                    }
                }
                #[cfg(feature = "debug")]
                UserEvent::OpenMapsWindow => self.interface.open_window(&self.application, &mut self.focus_state, &MapsWindow),
                #[cfg(feature = "debug")]
                UserEvent::OpenCommandsWindow => self
                    .interface
                    .open_window(&self.application, &mut self.focus_state, &CommandsWindow),
                #[cfg(feature = "debug")]
                UserEvent::OpenTimeWindow => self.interface.open_window(&self.application, &mut self.focus_state, &TimeWindow),
                #[cfg(feature = "debug")]
                UserEvent::SetDawn => self.game_timer.set_day_timer(5.0 * 3600.0),
                #[cfg(feature = "debug")]
                UserEvent::SetNoon => self.game_timer.set_day_timer(12.0 * 3600.0),
                #[cfg(feature = "debug")]
                UserEvent::SetDusk => self.game_timer.set_day_timer(17.0 * 3600.0),
                #[cfg(feature = "debug")]
                UserEvent::SetMidnight => self.game_timer.set_day_timer(24.0 * 3600.0),
                #[cfg(feature = "debug")]
                UserEvent::OpenThemeViewerWindow => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, self.application.theme_window())
                }
                #[cfg(feature = "debug")]
                UserEvent::OpenProfilerWindow => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &ProfilerWindow::new())
                }
                #[cfg(feature = "debug")]
                UserEvent::OpenPacketWindow => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    &PacketWindow::new(self.packet_history_callback.remote(), PlainTrackedState::default()),
                ),
                #[cfg(feature = "debug")]
                UserEvent::ClearPacketHistory => self.packet_history_callback.clear_all(),
                #[cfg(feature = "debug")]
                UserEvent::CameraLookAround(offset) => self.debug_camera.look_around(offset),
                #[cfg(feature = "debug")]
                UserEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time as f32),
                #[cfg(feature = "debug")]
                UserEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time as f32),
                #[cfg(feature = "debug")]
                UserEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time as f32),
                #[cfg(feature = "debug")]
                UserEvent::CameraMoveRight => self.debug_camera.move_right(delta_time as f32),
                #[cfg(feature = "debug")]
                UserEvent::CameraMoveUp => self.debug_camera.move_up(delta_time as f32),
                #[cfg(feature = "debug")]
                UserEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                UserEvent::CameraDecelerate => self.debug_camera.decelerate(),
            }
        }

        #[cfg(feature = "debug")]
        user_event_measurement.stop();

        #[cfg(feature = "debug")]
        let loads_measurement = Profiler::start_measurement("complete async loads");

        for completed in self.async_loader.take_completed() {
            match completed {
                (LoaderId::AnimationData(entity_id), LoadableResource::AnimationData(animation_data)) => {
                    if let Some(entity) = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                        entity.set_animation_data(animation_data);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture, location }) => match location {
                    ItemLocation::Inventory => {
                        self.player_inventory.update_item_sprite(item_id, texture);
                    }
                    ItemLocation::Shop => {
                        self.shop_items.mutate(|items| {
                            items
                                .iter_mut()
                                .filter(|item| item.item_id == item_id)
                                .for_each(|item| item.metadata.texture = Some(texture.clone()));

                            ValueState::Mutated(())
                        });
                    }
                },
                (LoaderId::Map(..), LoadableResource::Map { map, player_position }) => match self.entities.is_empty() {
                    true => {
                        // Load of main menu map
                        let map = self.map.insert(map);

                        map.set_ambient_sound_sources(&self.audio_engine);
                        self.audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);

                        let character_selection_window = CharacterSelectionWindow::new(
                            self.saved_characters.new_remote(),
                            self.move_request.new_remote(),
                            self.saved_slot_count,
                        );
                        self.interface
                            .open_window(&self.application, &mut self.focus_state, &character_selection_window);

                        self.start_camera.set_focus_point(START_CAMERA_FOCUS_POINT);
                        self.directional_shadow_camera
                            .set_focus_point(self.start_camera.focus_point(), self.start_camera.view_direction());
                    }
                    false => {
                        // Normal map switch
                        let map = self.map.insert(map);

                        map.set_ambient_sound_sources(&self.audio_engine);
                        self.audio_engine.play_background_music_track(map.background_music_track_name());

                        if let Some(player_position) = player_position {
                            let player_position = Vector2::new(player_position.x as usize, player_position.y as usize);
                            self.entities[0].set_position(map, player_position, client_tick);
                            self.player_camera.set_focus_point(self.entities[0].get_position());
                        }

                        self.interface.schedule_render();
                        let _ = self.networking_system.map_loaded();
                    }
                },
                _ => {}
            }
        }

        #[cfg(feature = "debug")]
        loads_measurement.stop();

        // Main map update and render loop
        match self.map.as_ref() {
            Some(map) => {
                #[cfg(feature = "debug")]
                let update_main_camera_measurement = Profiler::start_measurement("update main camera");

                let window_size = self.graphics_engine.get_window_size();
                let screen_size: ScreenSize = window_size.into();

                if self.entities.is_empty() {
                    self.start_camera.update(delta_time);
                    self.start_camera.generate_view_projection(window_size);
                } else {
                    self.player_camera.update(delta_time);
                    self.player_camera.generate_view_projection(window_size);
                }

                #[cfg(feature = "debug")]
                if self.render_settings.get().use_debug_camera {
                    self.debug_camera.generate_view_projection(window_size);
                }

                #[cfg(feature = "debug")]
                update_main_camera_measurement.stop();

                #[cfg(feature = "debug")]
                let update_entities_measurement = Profiler::start_measurement("update entities");

                {
                    let current_camera: &(dyn Camera + Send + Sync) = match self.entities.is_empty() {
                        #[cfg(feature = "debug")]
                        _ if self.render_settings.get().use_debug_camera => &self.debug_camera,
                        true => &self.start_camera,
                        false => &self.player_camera,
                    };

                    self.entities
                        .iter_mut()
                        .for_each(|entity| entity.update(&self.audio_engine, map, current_camera, client_tick));
                }

                match self.entities.is_empty() {
                    true => {
                        self.directional_shadow_camera
                            .set_focus_point(self.start_camera.focus_point(), self.start_camera.view_direction());
                    }
                    false => {
                        let player_position = self.entities[0].get_position();
                        self.player_camera.set_smoothed_focus_point(player_position);
                        self.directional_shadow_camera
                            .set_focus_point(self.player_camera.focus_point(), self.player_camera.view_direction());
                    }
                }

                let current_camera: &(dyn Camera + Send + Sync) = match self.entities.is_empty() {
                    #[cfg(feature = "debug")]
                    _ if self.render_settings.get().use_debug_camera => &self.debug_camera,
                    true => &self.start_camera,
                    false => &self.player_camera,
                };

                let (view_matrix, projection_matrix) = current_camera.view_projection_matrices();
                let camera_position = current_camera.camera_position().to_homogeneous();

                #[cfg(feature = "debug")]
                update_entities_measurement.stop();

                #[cfg(feature = "debug")]
                let update_shadow_camera_measurement = Profiler::start_measurement("update directional shadow camera");

                let lighting_mode = *self.lighting_mode.get();
                let shadow_detail = *self.shadow_detail.get();
                let shadow_quality = *self.shadow_quality.get();
                let shadow_map_size = shadow_detail.directional_shadow_resolution();
                let ambient_light_color = map.ambient_light_color(lighting_mode, day_timer);

                let (directional_light_direction, directional_light_color) = map.directional_light(lighting_mode, day_timer);

                self.directional_shadow_camera
                    .update(directional_light_direction, current_camera.view_direction(), shadow_map_size);
                self.directional_shadow_camera.generate_view_projection(window_size);

                let (directional_light_view_matrix, directional_light_projection_matrix) =
                    self.directional_shadow_camera.view_projection_matrices();
                let directional_light_view_projection_matrix = directional_light_projection_matrix * directional_light_view_matrix;

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
                self.effect_holder.update(&self.entities, delta_time as f32);

                let (clear_interface, render_interface) =
                    self.interface
                        .update(&self.application, self.font_loader.clone(), &mut self.focus_state);
                self.mouse_cursor.update(client_tick);

                #[cfg(feature = "debug")]
                let render_settings = &*self.render_settings.get();
                let walk_indicator_color = self.application.get_game_theme().indicator.walking.get();

                #[cfg(feature = "debug")]
                let hovered_marker_identifier = match mouse_target {
                    Some(PickerTarget::Marker(marker_identifier)) => Some(marker_identifier),
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
                    left: mouse_position.left.clamp(0.0, window_size.x as f32),
                    top: mouse_position.top.clamp(0.0, window_size.y as f32),
                };
                let mut indicator_instruction = None;
                let mut water_instruction = None;

                // Marker
                {
                    #[cfg(feature = "debug")]
                    map.render_markers(
                        &mut self.debug_marker_renderer,
                        current_camera,
                        render_settings,
                        &self.entities,
                        &point_light_set,
                        hovered_marker_identifier,
                    );

                    #[cfg(feature = "debug")]
                    map.render_markers(
                        &mut self.middle_interface_renderer,
                        current_camera,
                        render_settings,
                        &self.entities,
                        &point_light_set,
                        hovered_marker_identifier,
                    );
                }

                // Directional Shadows
                {
                    let object_set = map.cull_objects_with_frustum(
                        &self.directional_shadow_camera,
                        &mut self.directional_shadow_object_set_buffer,
                        #[cfg(feature = "debug")]
                        render_settings.frustum_culling,
                    );

                    let offset = self.directional_shadow_model_instructions.len();

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
                    map.render_objects(
                        &mut self.directional_shadow_model_instructions,
                        &object_set,
                        client_tick,
                        &self.directional_shadow_camera,
                    );

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
                    map.render_ground(&mut self.directional_shadow_model_instructions);

                    let count = self.directional_shadow_model_instructions.len() - offset;

                    self.directional_shadow_model_batches.push(ModelBatch {
                        offset,
                        count,
                        texture_set: map.get_texture_set().clone(),
                        vertex_buffer: map.get_model_vertex_buffer().clone(),
                    });

                    #[cfg(feature = "debug")]
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map_tiles))]
                    map.render_overlay_tiles(
                        &mut self.directional_shadow_model_instructions,
                        &mut self.directional_shadow_model_batches,
                        &self.tile_texture_set,
                    );

                    #[cfg(feature = "debug")]
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_pathing))]
                    map.render_entity_pathing(
                        &mut self.directional_shadow_model_instructions,
                        &mut self.directional_shadow_model_batches,
                        &self.entities,
                        &self.pathing_texture_set,
                    );

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
                    map.render_entities(
                        &mut self.directional_shadow_entity_instructions,
                        &mut self.entities,
                        &self.directional_shadow_camera,
                    );
                }

                // Point Lights and Shadows
                {
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_point_lights))]
                    point_light_set.render_point_lights(&mut self.point_light_instructions);

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_point_lights))]
                    point_light_set.render_point_lights_with_shadows(
                        map,
                        &mut self.point_shadow_camera,
                        &mut self.point_shadow_object_set_buffer,
                        &mut self.point_shadow_model_instructions,
                        &mut self.point_light_with_shadow_instructions,
                        client_tick,
                        #[cfg(feature = "debug")]
                        render_settings,
                    );
                }

                // Geometry
                {
                    let object_set = map.cull_objects_with_frustum(
                        current_camera,
                        &mut self.deferred_object_set_buffer,
                        #[cfg(feature = "debug")]
                        render_settings.frustum_culling,
                    );

                    let offset = self.model_instructions.len();

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
                    map.render_objects(&mut self.model_instructions, &object_set, client_tick, current_camera);

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
                    map.render_ground(&mut self.model_instructions);

                    let count = self.model_instructions.len() - offset;

                    self.model_batches.push(ModelBatch {
                        offset,
                        count,
                        texture_set: map.get_texture_set().clone(),
                        vertex_buffer: map.get_model_vertex_buffer().clone(),
                    });

                    #[cfg(feature = "debug")]
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map_tiles))]
                    map.render_overlay_tiles(&mut self.model_instructions, &mut self.model_batches, &self.tile_texture_set);

                    #[cfg(feature = "debug")]
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_pathing))]
                    map.render_entity_pathing(
                        &mut self.model_instructions,
                        &mut self.model_batches,
                        &self.entities,
                        &self.pathing_texture_set,
                    );

                    let entity_camera = match true {
                        #[cfg(feature = "debug")]
                        _ if self.render_settings.get().show_entities_paper => &self.player_camera,
                        _ => current_camera,
                    };

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
                    map.render_entities(&mut self.entity_instructions, &mut self.entities, entity_camera);

                    #[cfg(feature = "debug")]
                    if render_settings.show_entities_debug {
                        map.render_entities_debug(&mut self.rectangle_instructions, &self.entities, entity_camera);
                    }

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_water))]
                    map.render_water(&mut water_instruction, client_tick);

                    #[cfg(feature = "debug")]
                    if render_settings.show_bounding_boxes {
                        let object_set = map.cull_objects_with_frustum(
                            &self.player_camera,
                            &mut self.bounding_box_object_set_buffer,
                            #[cfg(feature = "debug")]
                            render_settings.frustum_culling,
                        );

                        map.render_bounding(&mut self.aabb_instructions, render_settings.frustum_culling, &object_set);
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
                            animation_timer,
                        );
                    }

                    self.particle_holder.render(
                        &self.bottom_interface_renderer,
                        current_camera,
                        screen_size,
                        scaling,
                        &self.entities,
                    );

                    self.effect_holder.render(&mut self.effect_renderer, current_camera);

                    if let Some(PickerTarget::Tile { x, y }) = mouse_target
                        && !&self.entities.is_empty()
                    {
                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators))]
                        map.render_walk_indicator(
                            &mut indicator_instruction,
                            walk_indicator_color,
                            Vector2::new(x as usize, y as usize),
                        );
                    } else if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                        let entity = &self.entities.iter().find(|entity| entity.get_entity_id() == entity_id);

                        if let Some(entity) = entity {
                            entity.render_status(
                                &self.middle_interface_renderer,
                                current_camera,
                                self.application.get_game_theme(),
                                screen_size,
                            );

                            if let Some(name) = &entity.get_details() {
                                let name = name.split('#').next().unwrap();

                                let offset = ScreenPosition { left: 15.0, top: 15.0 }.scaled(scaling);

                                self.middle_interface_renderer.render_text(
                                    name,
                                    mouse_position + offset,
                                    Color::WHITE,
                                    FontSize::new(16.0),
                                    AlignHorizontal::Mid,
                                );
                            }
                        }
                    }

                    if !&self.entities.is_empty() {
                        #[cfg(feature = "debug")]
                        profile_block!("render player status");

                        self.entities[0].render_status(
                            &self.middle_interface_renderer,
                            current_camera,
                            self.application.get_game_theme(),
                            screen_size,
                        );
                    }

                    if render_interface {
                        #[cfg(feature = "debug")]
                        profile_block!("render user interface");

                        self.interface.render(
                            &self.interface_renderer,
                            &self.application,
                            hovered_element,
                            focused_element,
                            self.input_system.get_mouse_mode(),
                        );
                    }

                    #[cfg(feature = "debug")]
                    if render_settings.show_frames_per_second {
                        let game_theme = self.application.get_game_theme();

                        self.top_interface_renderer.render_text(
                            &self.game_timer.last_frames_per_second().to_string(),
                            game_theme.overlay.text_offset.get(),
                            game_theme.overlay.foreground_color.get(),
                            game_theme.overlay.font_size.get(),
                            AlignHorizontal::Left,
                        );
                    }

                    if self.show_interface {
                        self.mouse_cursor.render(
                            &self.top_interface_renderer,
                            mouse_position,
                            self.input_system.get_mouse_mode().grabbed(),
                            self.application.get_game_theme().cursor.color.get(),
                            &self.application,
                        );
                    }
                }

                #[cfg(feature = "debug")]
                collect_instructions_measurement.stop();

                #[cfg(feature = "debug")]
                let render_frame_measurement = Profiler::start_measurement("render next frame");

                let interface_instructions = self.interface_renderer.get_instructions();
                let bottom_layer_instructions = self.bottom_interface_renderer.get_instructions();
                let middle_layer_instructions = self.middle_interface_renderer.get_instructions();
                let top_layer_instructions = self.top_interface_renderer.get_instructions();

                let render_instruction = RenderInstruction {
                    clear_interface,
                    show_interface: self.show_interface,
                    picker_position,
                    uniforms: Uniforms {
                        view_matrix,
                        projection_matrix,
                        camera_position,
                        animation_timer,
                        day_timer,
                        ambient_light_color,
                        enhanced_lighting: lighting_mode == LightingMode::Enhanced,
                        shadow_quality,
                    },
                    indicator: indicator_instruction,
                    interface: interface_instructions.as_slice(),
                    bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
                    middle_layer_rectangles: middle_layer_instructions.as_slice(),
                    top_layer_rectangles: top_layer_instructions.as_slice(),
                    directional_light_with_shadow: DirectionalShadowCasterInstruction {
                        view_projection_matrix: directional_light_view_projection_matrix,
                        view_matrix: directional_light_view_matrix,
                        direction: directional_light_direction,
                        color: directional_light_color,
                    },
                    point_light_shadow_caster: &self.point_light_with_shadow_instructions,
                    point_light: &self.point_light_instructions,
                    model_batches: &self.model_batches,
                    models: &mut self.model_instructions,
                    entities: &mut self.entity_instructions,
                    directional_model_batches: &self.directional_shadow_model_batches,
                    directional_shadow_models: &self.directional_shadow_model_instructions,
                    directional_shadow_entities: &self.directional_shadow_entity_instructions,
                    point_shadow_models: &self.point_shadow_model_instructions,
                    point_shadow_entities: &self.point_shadow_entity_instructions,
                    effects: self.effect_renderer.get_instructions(),
                    water: water_instruction,
                    map_picker_tile_vertex_buffer: Some(map.get_tile_picker_vertex_buffer()),
                    font_map_texture: Some(self.font_loader.get_font_map()),
                    #[cfg(feature = "debug")]
                    render_settings: *self.render_settings.get(),
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
            }
            _ => {
                #[cfg(feature = "debug")]
                let render_frame_measurement = Profiler::start_measurement("render next frame");

                self.graphics_engine.render_next_frame(frame, RenderInstruction::default());

                #[cfg(feature = "debug")]
                render_frame_measurement.stop();
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_graphic_settings(&mut self) {
        // For some reason the interface buffer becomes messed up when
        // recreating the surface, so we need to render it again.
        let mut update_interface = false;

        if self.vsync.consume_changed() {
            self.graphics_engine.set_vsync(*self.vsync.get());
            update_interface = true;
        }

        if self.limit_framerate.consume_changed() {
            self.graphics_engine.set_limit_framerate(*self.limit_framerate.get());
        }

        if self.triple_buffering.consume_changed() {
            self.graphics_engine.set_triple_buffering(*self.triple_buffering.get());
            update_interface = true;
        }

        if self.texture_filtering.consume_changed() {
            self.graphics_engine.set_texture_sampler_type(*self.texture_filtering.get());
            update_interface = true;
        }

        if self.msaa.consume_changed() {
            self.graphics_engine.set_msaa(*self.msaa.get());
            update_interface = true;
        }

        if self.ssaa.consume_changed() {
            self.graphics_engine.set_ssaa(*self.ssaa.get());
            update_interface = true;
        }

        if self.screen_space_anti_aliasing.consume_changed() {
            self.graphics_engine
                .set_screen_space_anti_aliasing(*self.screen_space_anti_aliasing.get());
        }

        if self.shadow_detail.consume_changed() {
            self.graphics_engine.set_shadow_detail(*self.shadow_detail.get());
        }

        if self.high_quality_interface.consume_changed() {
            let high_quality_interface = *self.high_quality_interface.get();
            self.interface_renderer.update_high_quality_interface(high_quality_interface);
            self.graphics_engine.set_high_quality_interface(high_quality_interface);
            update_interface = true;
        }

        if update_interface {
            self.interface.schedule_render();
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
            self.graphics_engine.on_resume(
                window.clone(),
                *self.triple_buffering.get(),
                *self.vsync.get(),
                *self.limit_framerate.get(),
                *self.shadow_detail.get(),
                *self.texture_filtering.get(),
                *self.msaa.get(),
                *self.ssaa.get(),
                *self.screen_space_anti_aliasing.get(),
                *self.high_quality_interface.get(),
            );

            window.set_visible(true);
        }

        if *self.mute_on_focus_loss.get() {
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
                    self.focus_state.remove_focus();
                }
                let mute_on_focus_loss = *self.mute_on_focus_loss.get();
                if mute_on_focus_loss {
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

        if *self.mute_on_focus_loss.get() {
            self.audio_engine.mute(true);
        }
    }
}
