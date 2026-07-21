#![allow(incomplete_features)]
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
#![feature(anonymous_lifetime_in_impl_trait)]
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
mod loaders;
#[cfg(feature = "debug")]
mod networking;
mod renderer;
mod settings;
mod system;
mod world;

use std::io::Cursor;
use std::net::{SocketAddr, ToSocketAddrs};
use std::num::NonZeroU32;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use cgmath::{Point3, Vector3};
use image::{EncodableLayout, ImageFormat, ImageReader};
use input::{MouseInputMode, MouseModeExt};
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
    AttackRange, BuyShopItemsResult, CharacterServerInformation, ClientTick, Direction, DisappearanceReason, EntityId, HotbarSlot,
    SellItemsResult, SkillId, SkillLevel, SkillType, SkillUseFailureCode, TilePosition, WorldPosition,
};
use renderer::InterfaceRenderer;
use rust_state::{ManuallyAssertExt, State};
#[cfg(feature = "debug")]
use rust_state::{VecIndexExt, VecLookupExt};
use settings::{
    AudioSettings, AudioSettingsPathExt, GraphicsSettingsCapabilities, GraphicsSettingsPathExt, InterfaceSettings, InterfaceSettingsPathExt,
};
use state::hotbar::HotbarPathExt;
use state::inventory::InventoryPathExt;
use state::localization::Localization;
use state::skills::SkillTreePathExt;
use state::theme::{CursorThemePathExt, IndicatorThemePathExt, InterfaceThemePathExt, WorldThemePathExt};
use state::{ChatMessage, ClientState, ClientStatePathExt, client_state, this_entity, this_player};
#[cfg(feature = "debug")]
use wgpu::Device;
use wgpu::util::initialize_adapter_from_env_or_default;
use wgpu::wgt::{Dx12SwapchainKind, Dx12UseFrameLatencyWaitableObject};
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, Dx12BackendOptions, Dx12Compiler, ExperimentalFeatures, ForceShaderModelToken,
    GlBackendOptions, GlDebugFns, GlFenceBehavior, Gles3MinorVersion, Instance, InstanceDescriptor, InstanceFlags, MemoryBudgetThresholds,
    MemoryHints, NoopBackendOptions, Queue, Trace,
};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::PhysicalKey;
use winit::window::{Icon, Window, WindowId};

use crate::graphics::*;
use crate::input::{InputEvent, InputReport, InputSystem, SkillActivation, SkillCastTarget};
use crate::interface::cursor::{MouseCursor, MouseCursorState};
use crate::interface::resource::{ItemSource, SkillSource};
use crate::interface::windows::*;
use crate::loaders::*;
#[cfg(feature = "debug")]
use crate::renderer::{AlignHorizontal, DebugMarkerRenderer};
use crate::renderer::{EffectRenderer, GameInterfaceRenderer};
use crate::settings::{
    GameSettingsPathExt, GraphicsSettings, IN_GAME_THEMES_PATH, LightingMode, MENU_THEMES_PATH, ServiceSettingsPathExt, WORLD_THEMES_PATH,
};
use crate::state::skills::{LearnedSkill, SkillTreeLayoutPathExt, bring_skill_to_level};
use crate::state::theme::{InterfaceTheme, InterfaceThemeType, WorldTheme};
use crate::state::{BufferedAction, SelectedServicePath};
use crate::system::{FrameTimers, GameTimer};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::*;

const CLIENT_NAME: &str = "Korangar";
const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
const DEFAULT_MAP: &str = "geffen";
const START_CAMERA_FOCUS_POINT: Point3<f32> = Point3::new(600.0, 0.0, 240.0);
const DEFAULT_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3");
const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "버튼소리.wav";
const ITEM_PICKUP_RANGE: AttackRange = AttackRange(1);
// TODO: The number of point lights that can cast shadows should be configurable
// through the graphics settings. For now I just chose an arbitrary smaller
// number that should be playable on most devices.
const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;
const SKILL_SOUND_LOAD_TIMEOUT_SECONDS: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArmedSkill {
    skill_id: SkillId,
    skill_level: SkillLevel,
    skill_type: SkillType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveContinuousSkill {
    skill_id: SkillId,
    held_by: Option<HotbarSlot>,
}

#[derive(Clone, Copy)]
struct PendingSkillEffect {
    remaining_delay: f32,
    recipe: SkillVisualRecipe,
    source_entity_id: EntityId,
    destination_entity_id: EntityId,
    ground_position: Option<TilePosition>,
    unit_entity_id: Option<EntityId>,
}

#[derive(Clone, Copy)]
struct PendingSkillSound {
    timing: SkillSoundSequenceTiming,
    recipe: SkillSoundRecipe,
    sound_effect_key: SoundEffectKey,
    source_entity_id: EntityId,
    destination_entity_id: EntityId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct SkillSoundSequenceTiming {
    next_delay: f32,
    load_wait_remaining: f32,
    hit_interval: f32,
    hits_remaining: usize,
}

impl SkillSoundSequenceTiming {
    fn new(next_delay: f32, hit_interval: f32, hits_remaining: usize) -> Self {
        Self {
            next_delay: next_delay.max(0.0),
            load_wait_remaining: SKILL_SOUND_LOAD_TIMEOUT_SECONDS,
            hit_interval,
            hits_remaining: hits_remaining.max(1),
        }
    }

    /// Advances the sequence and returns how much of this frame elapsed after
    /// the next hit became due.
    fn wait_elapsed_if_due(&mut self, delta_time: f32) -> Option<f32> {
        if self.next_delay > delta_time {
            self.next_delay -= delta_time;
            return None;
        }

        let wait_elapsed = (delta_time - self.next_delay).max(0.0);
        self.next_delay = 0.0;
        Some(wait_elapsed)
    }

    /// Records a successful playback and returns whether the sequence remains
    /// active.
    fn playback_succeeded(&mut self) -> bool {
        self.hits_remaining = self.hits_remaining.saturating_sub(1);
        if self.hits_remaining == 0 {
            return false;
        }

        self.next_delay = self.hit_interval;
        self.load_wait_remaining = SKILL_SOUND_LOAD_TIMEOUT_SECONDS;
        true
    }

    /// Records time spent waiting for decoded audio and returns whether the
    /// sequence should retry.
    fn playback_unavailable(&mut self, wait_elapsed: f32) -> bool {
        self.load_wait_remaining -= wait_elapsed;
        self.load_wait_remaining > 0.0
    }
}

#[derive(Clone, Copy)]
struct PendingSkillSprite {
    remaining_delay: f32,
    recipe: SkillSpriteVisualRecipe,
    source_entity_id: EntityId,
    destination_entity_id: EntityId,
    play_sound: bool,
}

#[derive(Clone, Copy)]
struct PendingSkillDamageParticle {
    remaining_delay: f32,
    destination_entity_id: EntityId,
    display: SkillDamageDisplay,
}

#[derive(Clone, Copy)]
struct PendingProceduralSkillVisual {
    remaining_delay: f32,
    recipe: SkillProceduralVisualRecipe,
    source_entity_id: EntityId,
    destination_entity_id: EntityId,
    sequence_index: usize,
    flight_time: f32,
}

fn is_skill_target_confirmation(mouse_button: MouseButton) -> bool {
    matches!(mouse_button, MouseButton::Left | MouseButton::DoubleLeft)
}

fn is_skill_target_cancellation(mouse_button: MouseButton) -> bool {
    matches!(mouse_button, MouseButton::Right | MouseButton::DoubleRight)
}

fn resolve_skill_cast_target(
    skill_type: SkillType,
    picker_target: PickerTarget,
    player_entity_id: Option<EntityId>,
    entity_position: impl Fn(EntityId) -> Option<TilePosition>,
    ground_item_position: impl Fn(EntityId) -> Option<TilePosition>,
) -> Option<SkillCastTarget> {
    match skill_type {
        SkillType::Attack => match picker_target {
            PickerTarget::Entity(entity_id) if entity_position(entity_id).is_some() => Some(SkillCastTarget::Entity(entity_id)),
            _ => None,
        },
        SkillType::Support => match picker_target {
            PickerTarget::Entity(entity_id) if entity_position(entity_id).is_some() => Some(SkillCastTarget::Entity(entity_id)),
            PickerTarget::Nothing | PickerTarget::Tile { .. } => player_entity_id.map(SkillCastTarget::Entity),
            _ => None,
        },
        SkillType::Ground | SkillType::Trap => match picker_target {
            PickerTarget::Tile { x, y } => Some(SkillCastTarget::Ground(TilePosition { x, y })),
            PickerTarget::Entity(entity_id) => entity_position(entity_id)
                .or_else(|| ground_item_position(entity_id))
                .map(SkillCastTarget::Ground),
            _ => None,
        },
        SkillType::Passive | SkillType::SelfCast => None,
    }
}

fn take_resolved_armed_skill(
    armed_skill: &mut Option<ArmedSkill>,
    resolved_target: Option<SkillCastTarget>,
) -> Option<(ArmedSkill, SkillCastTarget)> {
    let resolved_target = resolved_target?;
    armed_skill.take().map(|armed_skill| (armed_skill, resolved_target))
}

fn activate_continuous_skill(
    active_skill: &mut Option<ActiveContinuousSkill>,
    skill_id: SkillId,
    activation: SkillActivation,
    source_slot: Option<HotbarSlot>,
) -> (Option<SkillId>, bool) {
    if activation == SkillActivation::Toggle
        && active_skill.is_some_and(|active_skill| active_skill.skill_id == skill_id && active_skill.held_by.is_none())
    {
        return (active_skill.take().map(|active_skill| active_skill.skill_id), false);
    }

    let stopped_skill_id = active_skill.take().map(|active_skill| active_skill.skill_id);
    let held_by = match activation {
        SkillActivation::Hold => {
            debug_assert!(source_slot.is_some());
            source_slot
        }
        SkillActivation::Toggle => None,
    };

    *active_skill = Some(ActiveContinuousSkill { skill_id, held_by });

    (stopped_skill_id, true)
}

fn release_continuous_skill(active_skill: &mut Option<ActiveContinuousSkill>, slot: HotbarSlot) -> Option<SkillId> {
    match active_skill.is_some_and(|active_skill| active_skill.held_by == Some(slot)) {
        true => active_skill.take().map(|active_skill| active_skill.skill_id),
        false => None,
    }
}

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

pub fn init_tls_rand() {
    use std::random::*;
    let mut seed = [0; 32];
    DefaultRandomSource.fill_bytes(&mut seed);
    rand_aes::tls::rand_seed(seed.into());
}

fn initialize_shutdown_signal() {
    ctrlc::set_handler(|| {
        println!("CTRL-C received. Shutting down");
        SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
}

pub struct Client {
    game_file_loader: Arc<GameFileLoader>,
    action_loader: Arc<ActionLoader>,
    #[cfg(feature = "debug")]
    animation_loader: Arc<AnimationLoader>,
    async_loader: Arc<AsyncLoader>,
    effect_loader: Arc<EffectLoader>,
    font_loader: Arc<FontLoader>,
    #[cfg(feature = "debug")]
    map_loader: Arc<MapLoader>,
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
    armed_skill: Option<ArmedSkill>,
    active_continuous_skill: Option<ActiveContinuousSkill>,
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
    pending_skill_effects: Vec<PendingSkillEffect>,
    pending_skill_sounds: Vec<PendingSkillSound>,
    pending_skill_sprites: Vec<PendingSkillSprite>,
    pending_skill_damage_particles: Vec<PendingSkillDamageParticle>,
    pending_procedural_skill_visuals: Vec<PendingProceduralSkillVisual>,
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

    map: Option<Arc<Map>>,
    client_state: State<ClientState>,
}

impl Client {
    pub fn init(sync_cache: bool) -> Option<Self> {
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

        time_phase!("load graphics settings", {
            let picker_value = Arc::new(AtomicU64::new(0));
            let directional_shadow_partitions = Arc::new(Mutex::new([DirectionalShadowPartition::default(); PARTITION_COUNT]));
            let input_system = InputSystem::new(picker_value.clone());
            let graphics_settings = GraphicsSettings::new();
        });

        time_phase!("create adapter", {
            let instance = Instance::new(InstanceDescriptor {
                backends: Backends::all().with_env(),
                flags: InstanceFlags::from_build_config().with_env(),
                memory_budget_thresholds: MemoryBudgetThresholds::default(),
                backend_options: BackendOptions {
                    gl: GlBackendOptions {
                        gles_minor_version: Gles3MinorVersion::Automatic,
                        fence_behavior: GlFenceBehavior::Normal,
                        debug_fns: GlDebugFns::Auto,
                    },
                    dx12: Dx12BackendOptions {
                        shader_compiler: Dx12Compiler::StaticDxc.with_env(),
                        presentation_system: Dx12SwapchainKind::DxgiFromHwnd,
                        latency_waitable_object: Dx12UseFrameLatencyWaitableObject::Wait,
                        force_shader_model: ForceShaderModelToken::default(),
                        agility_sdk: None,
                    },
                    noop: NoopBackendOptions { enable: false },
                },
                // On Vulkan, Metal and Dx12, this is currently unused.
                display: None,
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
            // Multi-hit skill sounds are scheduled only fractions of a second
            // apart. Start loading the verified GRF WAVs during client setup so
            // first-use async loading cannot collapse those requests together.
            for sound_path in SKILL_SOUND_PATHS {
                let _ = audio_engine.load(sound_path);
            }
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
            let client_state = State::new(ClientState::new(
                &game_file_loader,
                graphics_settings.clone(),
                #[cfg(feature = "debug")]
                packet_history,
            ));
        });

        let active_interface_settings = client_state.follow(crate::client_state().interface_settings()).clone();

        interface.open_window(LoginWindow::new(
            crate::client_state().login_window(),
            crate::client_state().login_settings(),
            crate::client_state().client_info(),
        ));

        Some(Self {
            game_file_loader,
            action_loader,
            #[cfg(feature = "debug")]
            animation_loader,
            async_loader,
            effect_loader,
            font_loader,
            #[cfg(feature = "debug")]
            map_loader,
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
            armed_skill: None,
            active_continuous_skill: None,
            saved_login_data,
            saved_character_server,
            saved_login_server_address,
            saved_password,
            saved_username,
            saved_packet_version,
            particle_holder,
            point_light_manager,
            effect_holder,
            pending_skill_effects: Vec::new(),
            pending_skill_sounds: Vec::new(),
            pending_skill_sprites: Vec::new(),
            pending_skill_damage_particles: Vec::new(),
            pending_procedural_skill_visuals: Vec::new(),
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

    pub fn run(&mut self) {
        let event_loop = EventLoop::new().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);
        let _ = event_loop.run_app(self);
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn clear_render_instructions(&mut self) {
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
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_client_state(&mut self) {
        // Unset the highlighted skill just before applying. That way, if the skill is
        // still hovered the value will be the same as before, if not it will clear the
        // highlighted.
        *self.client_state.follow_mut(client_state().skill_tree_window().highlighted_skill()) = None;

        // Apply the game state after all the UI work + rendering is done.
        if let Err(_errors) = self.client_state.apply() {
            #[cfg(feature = "debug")]
            {
                print_debug!("[{}] failed to apply {} updates: ", "error".red(), _errors.len());
                _errors.into_iter().for_each(|error| print_debug!("path: {}", error.type_name));
            }
        }
    }

    /// Apply any graphics or interface setting changes that the user
    /// dispatched during the previous frame.
    ///
    /// May reconfigure the GPU surface (MSAA, SSAA, present mode, shadow
    /// resolution, etc.). Surface reconfiguration is only safe between
    /// presenting the previous frame and acquiring the next swapchain image,
    /// so this *must* be called after [`Self::update_client_state`] (so the
    /// user's dispatched changes are visible) and *before*
    /// `graphics_engine.wait_for_next_frame()`. Calling it after
    /// `wait_for_next_frame` has been observed to cause surface configuration
    /// errors under DX12.
    #[inline(always)]
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

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_interface_scaling(&mut self, scaling: Scaling) {
        self.bottom_interface_renderer.update_scaling(scaling);
        self.middle_interface_renderer.update_scaling(scaling);
        self.top_interface_renderer.update_scaling(scaling);
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn request_entity_details(&mut self, input_report: &InputReport) {
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
    }

    fn entity_position(&self, entity_id: EntityId) -> Option<Point3<f32>> {
        self.client_state
            .follow(client_state().entities())
            .iter()
            .find(|entity| entity.get_entity_id() == entity_id)
            .map(Entity::get_position)
            .or_else(|| {
                self.client_state
                    .follow(client_state().dead_entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == entity_id)
                    .map(Entity::get_position)
            })
            .or_else(|| {
                self.client_state
                    .try_follow(this_entity())
                    .filter(|entity| entity.get_entity_id() == entity_id)
                    .map(Entity::get_position)
            })
    }

    fn spawn_skill_visual(
        &mut self,
        recipe: SkillVisualRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        ground_position: Option<TilePosition>,
        unit_entity_id: Option<EntityId>,
    ) {
        let (center, effect_position) = match recipe.anchor {
            SkillVisualAnchor::SourceEntity => {
                let Some(position) = self.entity_position(source_entity_id) else {
                    return;
                };
                (EffectCenter::Entity(source_entity_id, position), position)
            }
            SkillVisualAnchor::DestinationEntity => {
                let Some(position) = self.entity_position(destination_entity_id) else {
                    return;
                };
                (EffectCenter::Entity(destination_entity_id, position), position)
            }
            SkillVisualAnchor::GroundPosition | SkillVisualAnchor::SkillUnit => {
                let Some(map) = &self.map else {
                    return;
                };
                let Some(tile_position) = ground_position else {
                    return;
                };
                let Some(position) = map.get_world_position(tile_position) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] skill effect at {:?} is out of map bounds", "error".red(), tile_position);
                    return;
                };
                (EffectCenter::Position(position), position)
            }
        };

        if let Some(sound_path) = recipe.sound_path {
            let sound_effect = self.audio_engine.load(sound_path);
            self.audio_engine
                .play_spatial_sound_effect(sound_effect, effect_position, recipe.sound_range);
        }

        // The official client varies several impact animations per hit.
        let effect_path = pick_variant(recipe.effect_path, recipe.effect_path_variants, rand_aes::tls::rand_f32());
        let effect = match self.effect_loader.get_or_load(effect_path, &self.texture_loader) {
            Ok(effect) => effect,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to load skill effect '{}': {:?}",
                    "error".red(),
                    effect_path,
                    _error
                );
                return;
            }
        };
        let frame_timer = effect.new_frame_timer();
        let effect_offset = Vector3::new(recipe.effect_offset[0], recipe.effect_offset[1], recipe.effect_offset[2]);

        let effect: Box<dyn EffectBase + Send + Sync> = match recipe.light {
            Some(light) => Box::new(EffectWithLight::new(
                effect,
                frame_timer,
                center,
                effect_offset,
                next_dynamic_point_light_id(),
                Vector3::new(light.offset[0], light.offset[1], light.offset[2]),
                light.color,
                light.intensity,
                recipe.repeating,
            )),
            None => Box::new(EffectWithLight::without_light(
                effect,
                frame_timer,
                center,
                effect_offset,
                recipe.repeating,
            )),
        };

        match unit_entity_id {
            Some(entity_id) => self.effect_holder.add_unit(effect, entity_id),
            None => self.effect_holder.add_effect(effect),
        }
    }

    fn queue_skill_damage_visual(
        &mut self,
        recipe: SkillVisualRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        hit_count: i16,
        initial_delay: f32,
    ) {
        let repeat_delays = std::iter::once(0.0).chain(skill_effect_repeat_delays(recipe.hit_interval, hit_count));

        for repeat_delay in repeat_delays {
            let remaining_delay = initial_delay + repeat_delay;
            if remaining_delay <= 0.0 {
                self.spawn_skill_visual(recipe, source_entity_id, destination_entity_id, None, None);
            } else {
                self.pending_skill_effects.push(PendingSkillEffect {
                    recipe,
                    source_entity_id,
                    destination_entity_id,
                    remaining_delay,
                    ground_position: None,
                    unit_entity_id: None,
                });
            }
        }
    }

    fn update_pending_skill_effects(&mut self, delta_time: f32) {
        let pending_effects = std::mem::take(&mut self.pending_skill_effects);

        for mut pending in pending_effects {
            pending.remaining_delay -= delta_time;

            if pending.remaining_delay <= 0.0 {
                self.spawn_skill_visual(
                    pending.recipe,
                    pending.source_entity_id,
                    pending.destination_entity_id,
                    pending.ground_position,
                    pending.unit_entity_id,
                );
            } else {
                self.pending_skill_effects.push(pending);
            }
        }
    }

    fn queue_ground_skill_visual(
        &mut self,
        recipe: SkillVisualRecipe,
        source_entity_id: EntityId,
        ground_position: TilePosition,
        initial_delay: f32,
    ) {
        if initial_delay <= 0.0 {
            self.spawn_skill_visual(recipe, source_entity_id, source_entity_id, Some(ground_position), None);
        } else {
            self.pending_skill_effects.push(PendingSkillEffect {
                remaining_delay: initial_delay,
                recipe,
                source_entity_id,
                destination_entity_id: source_entity_id,
                ground_position: Some(ground_position),
                unit_entity_id: None,
            });
        }
    }

    fn spawn_skill_sprite_visual(
        &mut self,
        recipe: SkillSpriteVisualRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        play_sound: bool,
    ) {
        let entity_id = match recipe.anchor {
            SkillVisualAnchor::SourceEntity => source_entity_id,
            SkillVisualAnchor::DestinationEntity => destination_entity_id,
            SkillVisualAnchor::GroundPosition | SkillVisualAnchor::SkillUnit => return,
        };
        let Some(position) = self.entity_position(entity_id) else {
            return;
        };

        if play_sound && let Some(sound_path) = recipe.sound_path {
            let sound_effect = self.audio_engine.load(sound_path);
            self.audio_engine
                .play_spatial_sound_effect(sound_effect, position, recipe.sound_range);
        }

        let sprite = match self.sprite_loader.get_or_load(recipe.sprite_path) {
            Ok(sprite) => sprite,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to load skill sprite '{}': {:?}",
                    "error".red(),
                    recipe.sprite_path,
                    _error
                );
                return;
            }
        };
        let actions = match self.action_loader.get_or_load(recipe.action_path) {
            Ok(actions) => actions,
            Err(_error) => {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to load skill actions '{}': {:?}",
                    "error".red(),
                    recipe.action_path,
                    _error
                );
                return;
            }
        };

        self.particle_holder.spawn_attached_sprite(EntityAttachedSprite::new(
            entity_id,
            recipe.attachment_key,
            position,
            Vector3::new(recipe.position_offset[0], recipe.position_offset[1], recipe.position_offset[2]),
            sprite,
            actions,
            recipe.action_index,
            recipe.repeating,
            recipe.maximum_duration,
            recipe.scaling,
        ));
    }

    fn queue_skill_sprite_visual(
        &mut self,
        recipe: SkillSpriteVisualRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        play_sound: bool,
        initial_delay: f32,
    ) {
        if initial_delay <= 0.0 {
            self.spawn_skill_sprite_visual(recipe, source_entity_id, destination_entity_id, play_sound);
        } else {
            self.pending_skill_sprites.push(PendingSkillSprite {
                remaining_delay: initial_delay,
                recipe,
                source_entity_id,
                destination_entity_id,
                play_sound,
            });
        }
    }

    fn update_pending_skill_sprites(&mut self, delta_time: f32) {
        let pending_sprites = std::mem::take(&mut self.pending_skill_sprites);

        for mut pending in pending_sprites {
            pending.remaining_delay -= delta_time;

            if pending.remaining_delay <= 0.0 {
                self.spawn_skill_sprite_visual(
                    pending.recipe,
                    pending.source_entity_id,
                    pending.destination_entity_id,
                    pending.play_sound,
                );
            } else {
                self.pending_skill_sprites.push(pending);
            }
        }
    }

    fn spawn_skill_damage_particle(&mut self, destination_entity_id: EntityId, display: SkillDamageDisplay) {
        let Some(position) = self.entity_position(destination_entity_id) else {
            return;
        };
        let particle: Box<dyn Particle + Send + Sync> = match display {
            SkillDamageDisplay::Suppressed => return,
            SkillDamageDisplay::Miss => Box::new(Miss::new(position)),
            SkillDamageDisplay::Damage { amount, is_critical } => Box::new(DamageNumber::new(position, amount.to_string(), is_critical)),
        };
        self.particle_holder.spawn_particle(particle);
    }

    fn queue_skill_damage_particle(
        &mut self,
        destination_entity_id: EntityId,
        display: SkillDamageDisplay,
        hit_count: i16,
        hit_interval: Option<f32>,
        initial_delay: f32,
    ) {
        let displays = match hit_interval {
            Some(_) => display.split_for_hits(hit_count),
            None => (display != SkillDamageDisplay::Suppressed).then_some(display).into_iter().collect(),
        };

        for (hit_index, display) in displays.into_iter().enumerate() {
            let remaining_delay = initial_delay + hit_interval.unwrap_or_default() * hit_index as f32;
            if remaining_delay <= 0.0 {
                self.spawn_skill_damage_particle(destination_entity_id, display);
            } else {
                self.pending_skill_damage_particles.push(PendingSkillDamageParticle {
                    remaining_delay,
                    destination_entity_id,
                    display,
                });
            }
        }
    }

    fn update_pending_skill_damage_particles(&mut self, delta_time: f32) {
        let pending_particles = std::mem::take(&mut self.pending_skill_damage_particles);

        for mut pending in pending_particles {
            pending.remaining_delay -= delta_time;

            if pending.remaining_delay <= 0.0 {
                self.spawn_skill_damage_particle(pending.destination_entity_id, pending.display);
            } else {
                self.pending_skill_damage_particles.push(pending);
            }
        }
    }

    fn load_skill_particle_texture(&self, path: &str) -> Option<Arc<Texture>> {
        self.texture_loader
            .get_or_load(path, ImageType::Color)
            .inspect_err(|_error| {
                #[cfg(feature = "debug")]
                print_debug!(
                    "[{}] failed to load skill particle texture '{}': {:?}",
                    "error".red(),
                    path,
                    _error
                );
            })
            .ok()
    }

    /// Floats the skill's name over its caster, as the classic client does
    /// when a cast fires. Names come from the archive's own skill table, so
    /// any caster's skills are covered, not only the local player's.
    fn spawn_skill_name_bubble(&mut self, source_entity_id: EntityId, skill_id: SkillId) {
        let name = &self.library.get::<SkillListInformation>(skill_id).name;
        if name.is_empty() {
            return;
        }
        let Some(position) = self.entity_position(source_entity_id) else {
            return;
        };

        let text = format!("{name} !!");
        self.particle_holder
            .spawn_entity_particle(Box::new(SkillNameBubble::new(source_entity_id, position, text)));
    }

    fn spawn_bolt_projectile(
        &mut self,
        art: BoltProjectileArt,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        sequence_index: usize,
        initial_elapsed: f32,
        flight_time: f32,
    ) {
        let Some(position) = self.entity_position(destination_entity_id) else {
            return;
        };

        let (textures, frame_duration) = match art.source {
            BoltFrameSource::Textures { paths, frame_duration } => {
                let mut textures = Vec::with_capacity(paths.len());
                for texture_path in paths {
                    let Some(texture) = self.load_skill_particle_texture(texture_path) else {
                        return;
                    };
                    textures.push(texture);
                }
                (textures, frame_duration)
            }
            BoltFrameSource::SpriteAction { sprite_path, action_path } => {
                let Ok(sprite) = self.sprite_loader.get_or_load(sprite_path) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to load projectile sprite '{}'", "error".red(), sprite_path);
                    return;
                };
                let Ok(actions) = self.action_loader.get_or_load(action_path) else {
                    #[cfg(feature = "debug")]
                    print_debug!("[{}] failed to load projectile actions '{}'", "error".red(), action_path);
                    return;
                };

                // The ACT's own cadence, from its first action.
                let frame_duration = actions.delays.first().copied().unwrap_or(1.0).max(f32::EPSILON) * ACT_DELAY_UNIT;
                (sprite.textures.clone(), frame_duration)
            }
        };

        let (width, height) = match art.size {
            BoltQuadSize::Fixed { width, height } => (width, height),
            BoltQuadSize::Native { scale } => {
                let Some(first) = textures.first() else {
                    return;
                };
                let size = first.get_size();
                (size.width as f32 * scale, size.height as f32 * scale)
            }
        };

        // The official client randomizes the launch sound per bolt.
        if let Some(first) = art.launch_sounds.first() {
            let sound_path = pick_variant(first, art.launch_sounds, rand_aes::tls::rand_f32());
            let sound_effect = self.audio_engine.load(sound_path);
            self.audio_engine.play_spatial_sound_effect(sound_effect, position, art.sound_range);
        }

        // A fixed reference flight time takes precedence over the attack-
        // motion derivation; the hit keeps the server's own timing either
        // way, so a faster projectile lands earlier than the impact rather
        // than desynchronizing it.
        let flight_time = art.flight_override.unwrap_or(flight_time);

        // A travelling projectile launches from wherever the caster stood at
        // spawn time. The fallback mirrors the Frost Diver convention for a
        // caster that despawned before its projectile resolved.
        let launch_origin = match art.motion {
            BoltMotion::FallOntoTarget => None,
            BoltMotion::TravelFromSource => Some(
                self.entity_position(source_entity_id)
                    .unwrap_or(position + Vector3::new(-22.0, 12.0, 0.0))
                    + Vector3::new(0.0, 7.0, 0.0),
            ),
        };

        let resolved = ResolvedBoltFrames {
            textures,
            frame_duration,
            half_width: width * 0.5,
            half_height: height * 0.5,
        };

        let mut effect: Box<dyn EffectBase + Send + Sync> = Box::new(BoltProjectile::new(
            art,
            resolved,
            destination_entity_id,
            position,
            launch_origin,
            sequence_index,
            flight_time,
        ));

        if initial_elapsed > 0.0 && !effect.update(&[], None, initial_elapsed) {
            return;
        }

        self.effect_holder.add_effect(effect);
    }

    fn spawn_procedural_skill_visual(
        &mut self,
        recipe: SkillProceduralVisualRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        sequence_index: usize,
        initial_elapsed: f32,
        flight_time: f32,
    ) {
        let mut particle: Box<dyn EntityParticle + Send + Sync> = match recipe.kind {
            // Both bolts render through the effect pipeline so they can be
            // rotated onto their flight direction, so they do not join the
            // interface-sprite particles below.
            SkillProceduralVisualKind::FireBoltProjectile => {
                self.spawn_bolt_projectile(
                    FIRE_BOLT_ART,
                    source_entity_id,
                    destination_entity_id,
                    sequence_index,
                    initial_elapsed,
                    flight_time,
                );
                return;
            }
            SkillProceduralVisualKind::ArrowProjectile => {
                self.spawn_bolt_projectile(
                    ARROW_ART,
                    source_entity_id,
                    destination_entity_id,
                    sequence_index,
                    initial_elapsed,
                    flight_time,
                );
                return;
            }
            SkillProceduralVisualKind::FireBallProjectile => {
                self.spawn_bolt_projectile(
                    FIRE_BALL_ART,
                    source_entity_id,
                    destination_entity_id,
                    sequence_index,
                    initial_elapsed,
                    flight_time,
                );
                return;
            }
            SkillProceduralVisualKind::ColdBolt => {
                self.spawn_bolt_projectile(
                    COLD_BOLT_ART,
                    source_entity_id,
                    destination_entity_id,
                    sequence_index,
                    initial_elapsed,
                    flight_time,
                );
                return;
            }
            SkillProceduralVisualKind::ColdImpact => {
                let Some(position) = self.entity_position(destination_entity_id) else {
                    return;
                };
                let Some(arrow_texture) = self.load_skill_particle_texture(ICE_ARROW_TEXTURE_PATH) else {
                    return;
                };
                let Some(impact_texture) = self.load_skill_particle_texture(ICE_IMPACT_TEXTURE_PATH) else {
                    return;
                };

                Box::new(ColdBoltParticle::impact(
                    destination_entity_id,
                    position,
                    arrow_texture,
                    impact_texture,
                ))
            }
            SkillProceduralVisualKind::FrostDiver => {
                let Some(destination_position) = self.entity_position(destination_entity_id) else {
                    return;
                };
                let source_position = self
                    .entity_position(source_entity_id)
                    .unwrap_or(destination_position + Vector3::new(-22.0, 12.0, 0.0));
                let Some(projectile_texture) = self.load_skill_particle_texture(FROST_DIVER_TEXTURE_PATH) else {
                    return;
                };
                let Some(impact_texture) = self.load_skill_particle_texture(ICE_IMPACT_TEXTURE_PATH) else {
                    return;
                };

                Box::new(FrostDiverParticle::new(
                    None,
                    Some(destination_entity_id),
                    source_position,
                    destination_position,
                    projectile_texture,
                    impact_texture,
                ))
            }
            SkillProceduralVisualKind::FrostDiverPreview => {
                let Some(destination_position) = self.entity_position(destination_entity_id) else {
                    return;
                };
                let source_position = destination_position + Vector3::new(-22.0, 12.0, 0.0);
                let Some(projectile_texture) = self.load_skill_particle_texture(FROST_DIVER_TEXTURE_PATH) else {
                    return;
                };

                Box::new(FrostDiverParticle::travel_only(
                    source_position,
                    destination_entity_id,
                    destination_position,
                    projectile_texture,
                ))
            }
            SkillProceduralVisualKind::FrostDiverImpact => {
                let Some(position) = self.entity_position(destination_entity_id) else {
                    return;
                };
                let Some(impact_texture) = self.load_skill_particle_texture(ICE_IMPACT_TEXTURE_PATH) else {
                    return;
                };

                Box::new(FrostDiverParticle::impact(destination_entity_id, position, impact_texture))
            }
        };

        if initial_elapsed > 0.0 && !particle.update(&[], None, initial_elapsed) {
            return;
        }

        self.particle_holder.spawn_entity_particle(particle);
    }

    fn queue_procedural_skill_visual(
        &mut self,
        recipe: SkillProceduralVisualRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        hit_count: i16,
        initial_delay: f32,
        flight_time: f32,
    ) {
        let repeat_delays = std::iter::once(0.0).chain(skill_effect_repeat_delays(recipe.hit_interval, hit_count));

        for (sequence_index, repeat_delay) in repeat_delays.enumerate() {
            let remaining_delay = initial_delay + repeat_delay;
            if remaining_delay <= 0.0 {
                self.spawn_procedural_skill_visual(
                    recipe,
                    source_entity_id,
                    destination_entity_id,
                    sequence_index,
                    0.0,
                    flight_time,
                );
            } else {
                self.pending_procedural_skill_visuals.push(PendingProceduralSkillVisual {
                    remaining_delay,
                    recipe,
                    source_entity_id,
                    destination_entity_id,
                    sequence_index,
                    flight_time,
                });
            }
        }
    }

    fn update_pending_procedural_skill_visuals(&mut self, delta_time: f32) {
        let pending_visuals = std::mem::take(&mut self.pending_procedural_skill_visuals);

        for mut pending in pending_visuals {
            pending.remaining_delay -= delta_time;

            if pending.remaining_delay <= 0.0 {
                self.spawn_procedural_skill_visual(
                    pending.recipe,
                    pending.source_entity_id,
                    pending.destination_entity_id,
                    pending.sequence_index,
                    -pending.remaining_delay,
                    pending.flight_time,
                );
            } else {
                self.pending_procedural_skill_visuals.push(pending);
            }
        }
    }

    fn play_skill_sound(
        &self,
        recipe: SkillSoundRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        ground_position: Option<TilePosition>,
    ) {
        let position = match recipe.anchor {
            SkillVisualAnchor::SourceEntity => self.entity_position(source_entity_id),
            SkillVisualAnchor::DestinationEntity => self.entity_position(destination_entity_id),
            SkillVisualAnchor::GroundPosition | SkillVisualAnchor::SkillUnit => {
                let map = self.map.as_ref();
                ground_position.and_then(|position| map.and_then(|map| map.get_world_position(position)))
            }
        };
        let Some(position) = position else {
            return;
        };

        let sound_path = pick_variant(recipe.sound_path, recipe.sound_path_variants, rand_aes::tls::rand_f32());
        let sound_effect = self.audio_engine.load(sound_path);
        self.audio_engine
            .play_spatial_sound_effect(sound_effect, position, recipe.sound_range);
    }

    fn play_skill_damage_sound(
        &mut self,
        recipe: SkillSoundRecipe,
        source_entity_id: EntityId,
        destination_entity_id: EntityId,
        hit_count: i16,
        initial_delay: f32,
    ) {
        let hits_remaining = 1 + skill_effect_repeat_delays(recipe.hit_interval, hit_count).len();
        let sound_path = pick_variant(recipe.sound_path, recipe.sound_path_variants, rand_aes::tls::rand_f32());
        let sound_effect_key = self.audio_engine.load(sound_path);
        let mut pending = PendingSkillSound {
            timing: SkillSoundSequenceTiming::new(initial_delay, recipe.hit_interval.unwrap_or_default(), hits_remaining),
            recipe,
            sound_effect_key,
            source_entity_id,
            destination_entity_id,
        };

        if self.update_skill_sound_sequence(&mut pending, 0.0) {
            self.pending_skill_sounds.push(pending);
        }
    }

    fn update_skill_sound_sequence(&self, pending: &mut PendingSkillSound, delta_time: f32) -> bool {
        let Some(wait_elapsed) = pending.timing.wait_elapsed_if_due(delta_time) else {
            return true;
        };

        let entity_id = match pending.recipe.anchor {
            SkillVisualAnchor::SourceEntity => pending.source_entity_id,
            SkillVisualAnchor::DestinationEntity => pending.destination_entity_id,
            SkillVisualAnchor::GroundPosition | SkillVisualAnchor::SkillUnit => return false,
        };
        let Some(position) = self.entity_position(entity_id) else {
            return false;
        };

        if self
            .audio_engine
            .try_play_spatial_sound_effect(pending.sound_effect_key, position, pending.recipe.sound_range)
        {
            pending.timing.playback_succeeded()
        } else {
            // The entry may have been evicted between sequence hits. `load`
            // deduplicates in-flight work and restarts loading for an existing
            // key that is no longer cached.
            pending.sound_effect_key = self.audio_engine.load(pending.recipe.sound_path);
            pending.timing.playback_unavailable(wait_elapsed)
        }
    }

    fn update_pending_skill_sounds(&mut self, delta_time: f32) {
        let pending_sounds = std::mem::take(&mut self.pending_skill_sounds);

        for mut pending in pending_sounds {
            if self.update_skill_sound_sequence(&mut pending, delta_time) {
                self.pending_skill_sounds.push(pending);
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn handle_network_events(&mut self, client_tick: ClientTick) {
        self.networking_system.get_events(&mut self.network_event_buffer);

        let events: Vec<NetworkEvent> = self.network_event_buffer.drain().collect();

        for event in events {
            match event {
                NetworkEvent::LoginServerConnected {
                    character_servers,
                    login_data,
                } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    // Remove `_m`/`_f` suffix from the username. The suffix is only for *creating*
                    // an account and thus can (and needs to) be removed after the first successful
                    // login.
                    {
                        let selected_service_path =
                            SelectedServicePath::new(client_state().login_window(), client_state().login_settings());
                        let username_path = selected_service_path.username();

                        let username = self.client_state.follow_mut(username_path);

                        if let Some(stripped) = username.strip_suffix("_m") {
                            *username = stripped.to_owned();
                        } else if let Some(stripped) = username.strip_suffix("_f") {
                            *username = stripped.to_owned();
                        }
                    }

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
                    self.clear_skill_cast_state();

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

                    self.clear_world_feedback();

                    self.client_state.follow_mut(client_state().entities()).clear();
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;

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
                NetworkEvent::EntityStartCasting {
                    source_entity_id,
                    cast_time,
                    element,
                    ..
                } => {
                    if let Some(cast_time) = NonZeroU32::new(cast_time)
                        && let Some(entity) = self
                            .client_state
                            .follow_mut(client_state().entities())
                            .iter_mut()
                            .find(|entity| entity.get_entity_id() == source_entity_id)
                    {
                        entity.set_casting(cast_time, client_tick);
                    }

                    // The magic circle swirling for the length of the cast,
                    // in the element the acknowledgement declares.
                    if let Some(cast_time) = NonZeroU32::new(cast_time)
                        && let Some(position) = self.entity_position(source_entity_id)
                        && let Some(texture) = self.load_skill_particle_texture(cast_aura_texture(element))
                    {
                        let sound_effect = self.audio_engine.load(CAST_AURA_SOUND_PATH);
                        self.audio_engine.play_spatial_sound_effect(sound_effect, position, 55.0);
                        self.particle_holder.spawn_cast_aura(CastAura::new(
                            source_entity_id,
                            position,
                            texture,
                            cast_time.get() as f32 / 1000.0,
                        ));
                    }
                }
                NetworkEvent::EntityCancelCasting { entity_id } => {
                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.cancel_casting(client_tick);
                    }

                    // An interrupted cast tears its aura down with it.
                    self.particle_holder.remove_cast_aura(entity_id);
                }
                NetworkEvent::DisplayEmotion { entity_id, emotion } => {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        && let Ok(sprite) = self.sprite_loader.get_or_load("이팩트\\emotion.spr")
                        && let Ok(actions) = self.action_loader.get_or_load("이팩트\\emotion.act")
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(Emote::new(entity.get_position(), sprite, actions, emotion as usize)));
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
                    // NOTE: Nothing else must be sent to the map server until it responds:
                    // rAthena requires the first read of a session to contain exactly the
                    // login packet and disconnects otherwise. The client tick is part of
                    // the map server login success packet, so there is no need to request
                    // it here.

                    let character_information = self
                        .client_state
                        .follow(client_state().character_slots())
                        .with_id(login_data.character_id)
                        .cloned()
                        .unwrap();

                    let mut player = Entity::Player(Player::new(
                        &self.library,
                        saved_login_data.account_id,
                        &character_information,
                        client_tick,
                    ));

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

                    let layout = self.async_loader.request_skill_tree_layout_load(player.get_job_id(), client_tick);
                    *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
                    *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .clear();

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
                    self.interface.open_window(HotbarWindow::new(
                        client_state().hotbar().skills(),
                        client_state().skill_tree().skills(),
                    ));

                    // Put the dialog system in a well-defined state.
                    self.client_state.follow_mut(client_state().dialog_window()).end();

                    self.map = None;

                    self.clear_world_feedback();
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
                    let effect_state = entity_data.effect_state;
                    let mut sight_entity_id = None;

                    if let Some(map) = &self.map
                        && let Some(npc) = Npc::new(&self.library, map, &mut self.path_finder, entity_data, client_tick)
                    {
                        let mut npc = Entity::Npc(npc);

                        let entity_id = npc.get_entity_id();
                        let entity_type = npc.get_entity_type();
                        let entity_part_files = npc.get_entity_part_files(&self.library);

                        let entities = self.client_state.follow_mut(client_state().entities());

                        // If the entity was already visible, we use it's old alpha value.
                        if let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == entity_id) {
                            npc.inherit_fade_state(entity, client_tick);
                        };

                        // Sometimes (like after a job change) the server will tell the client
                        // that a new entity appeared, even though it was already on screen. So
                        // to prevent the entity existing twice, we remove the old one.
                        entities.retain(|entity| entity.get_entity_id() != entity_id);

                        if let Some(animation_data) =
                            self.async_loader
                                .request_animation_data_load(entity_id, entity_type, entity_part_files)
                        {
                            npc.set_animation_data(animation_data);
                        }

                        #[cfg(feature = "debug")]
                        npc.generate_pathing_mesh(&self.device, &self.queue, self.graphics_engine.bindless_support(), map);

                        entities.push(npc);

                        if effect_state & OPTION_SIGHT != 0 {
                            sight_entity_id = Some(entity_id);
                        }
                    }

                    if let Some(entity_id) = sight_entity_id {
                        self.spawn_skill_sprite_visual(sight_sprite_visual(), entity_id, entity_id, false);
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
                            entity.fade_out(reason, client_tick);
                        }
                    }

                    self.particle_holder.remove_attached_sprite(entity_id, SIGHT_ATTACHMENT_KEY);

                    // If the entity that was removed had an attack buffered we remove the entity
                    // from the buffer.
                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id)) {
                        *buffered_action = None;
                    }
                }
                NetworkEvent::AddGroundItem {
                    entity_id,
                    item_id,
                    is_identified,
                    quantity,
                    position,
                    x_offset,
                    y_offset,
                } => {
                    if let Some(map) = self.map.as_ref()
                        && let Some(mut ground_item) = GroundItem::new(
                            map,
                            item_id,
                            entity_id,
                            is_identified,
                            quantity,
                            position,
                            x_offset,
                            y_offset,
                            client_tick,
                        )
                    {
                        let ground_items = self.client_state.follow_mut(client_state().ground_items());
                        let entity_part_files = ground_item.get_entity_part_files(&self.library);

                        if let Some(animation_data) = self
                            .async_loader
                            // TODO: Technically Npc is not correct here. We could add an item
                            // variant or refactor this fuction to take an optional entity
                            // type.
                            .request_animation_data_load(entity_id, EntityType::Npc, entity_part_files)
                        {
                            ground_item.set_animation_data(animation_data);
                        }

                        ground_items.push(ground_item);
                    } else {
                        #[cfg(feature = "debug")]
                        print_debug!("[{}] failed to spawn item", "error".red());
                    }
                }
                NetworkEvent::RemoveGroundItem { entity_id } => {
                    if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.fade_out(client_tick);
                    }

                    let buffered_action = self.client_state.follow_mut(client_state().buffered_action());
                    if buffered_action.is_some_and(|buffered_action| buffered_action.is_pick_up_item(entity_id)) {
                        *buffered_action = None;
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
                    self.armed_skill = None;
                    self.stop_active_continuous_skill();
                    self.input_system.clear_hotbar_key_ownership();
                    self.map = None;
                    self.clear_world_feedback();

                    // Only the player must stay alive between map changes.
                    let entities = self.client_state.follow_mut(client_state().entities());
                    if let Some(player) = entities.first_mut() {
                        player.cancel_casting(client_tick);
                    }
                    entities.truncate(1);
                    self.client_state.follow_mut(client_state().dead_entities()).clear();
                    self.client_state.follow_mut(client_state().ground_items()).clear();
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;

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
                NetworkEvent::SkillUseRejected {
                    skill_id,
                    detail,
                    item_id,
                    cause,
                } => {
                    let item_name = (item_id.0 != 0
                        && matches!(cause, SkillUseFailureCode::NEED_ITEM | SkillUseFailureCode::NEED_EQUIPMENT))
                    .then(|| {
                        self.library
                            .get::<ItemName>(ItemNameKey {
                                item_id,
                                is_identified: true,
                            })
                            .to_string()
                    })
                    .filter(|name| name != "NOTFOUND");
                    let message = self.client_state.follow(client_state().localization()).skill_use_failure_message(
                        skill_id,
                        detail,
                        item_id,
                        item_name.as_deref(),
                        cause,
                    );
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, MessageColor::Error));
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
                        let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                        if let Some(BufferedAction::AttackEntity { entity_id }) = *buffered_action {
                            let _ = self.networking_system.player_attack(entity_id);

                            if !auto_attack {
                                *buffered_action = None;
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
                NetworkEvent::SkillDamage {
                    skill_id,
                    source_entity_id,
                    destination_entity_id,
                    start_time,
                    source_motion,
                    damage,
                    hit_count,
                    action,
                    ..
                } => {
                    // Skill hits intentionally provide number feedback only.
                    // Normal DamageEffect also drives auto-attacks and sprite
                    // actions, which must not be triggered by this packet.
                    let initial_delay = skill_effect_initial_delay(start_time, client_tick);
                    let visual_recipe = skill_damage_visual(skill_id);
                    let cast_visual_recipe = skill_damage_cast_visual(skill_id);
                    let sprite_recipe = skill_damage_sprite_visual(skill_id, action);
                    let procedural_recipe = skill_damage_procedural_visual(skill_id);
                    let sound_recipe = skill_damage_sound(skill_id);
                    let hit_interval = visual_recipe
                        .and_then(|recipe| recipe.hit_interval)
                        .or_else(|| procedural_recipe.and_then(|recipe| recipe.hit_interval))
                        .or_else(|| sound_recipe.and_then(|recipe| recipe.hit_interval))
                        .or_else(|| skill_damage_number_interval(skill_id));

                    // A projectile or leading cast visual launches when the
                    // packet says the skill resolved, but its hit only lands
                    // once it arrives. The server's attack motion is that
                    // flight time, so all hit feedback waits for it. Skills
                    // with neither keep a zero lead and are unaffected.
                    let flight_time = skill_impact_lead_time(procedural_recipe, cast_visual_recipe, source_motion);
                    let impact_delay = initial_delay + flight_time;

                    // One name per cast: splash victims carry their own
                    // packets and must not each raise a bubble.
                    if !matches!(action, 5 | 14) {
                        self.spawn_skill_name_bubble(source_entity_id, skill_id);
                    }

                    self.queue_skill_damage_particle(
                        destination_entity_id,
                        SkillDamageDisplay::from_packet(damage, action),
                        hit_count,
                        hit_interval,
                        impact_delay,
                    );
                    // The once-per-resolution component, independent of the
                    // hit count. Mirrors the official client's split between
                    // a skill's effect and its per-hit effect.
                    if let Some(recipe) = cast_visual_recipe {
                        self.queue_skill_damage_visual(recipe, source_entity_id, destination_entity_id, 1, initial_delay);
                    }
                    if let Some(recipe) = visual_recipe {
                        self.queue_skill_damage_visual(recipe, source_entity_id, destination_entity_id, hit_count, impact_delay);
                    }
                    if let Some(recipe) = sprite_recipe {
                        self.queue_skill_sprite_visual(recipe, source_entity_id, destination_entity_id, true, impact_delay);
                    }
                    if let Some(recipe) = procedural_recipe.filter(|recipe| procedural_spawns_for_action(*recipe, action)) {
                        self.queue_procedural_skill_visual(
                            recipe,
                            source_entity_id,
                            destination_entity_id,
                            hit_count,
                            initial_delay,
                            flight_time,
                        );
                    }
                    if let Some(recipe) = sound_recipe {
                        self.play_skill_damage_sound(recipe, source_entity_id, destination_entity_id, hit_count, impact_delay);
                    }
                    if let Some((recipe, followup_delay)) = skill_damage_followup_sound(skill_id) {
                        self.play_skill_damage_sound(
                            recipe,
                            source_entity_id,
                            destination_entity_id,
                            1,
                            impact_delay + followup_delay,
                        );
                    }
                }
                NetworkEvent::EntityPickUpItem { entity_id, item_entity_id } => {
                    let item_position = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == item_entity_id)
                        .map(|item| item.get_tile_position());

                    if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        if let Some(item_position) = item_position {
                            entity.rotate_towards(item_position);
                        }

                        if matches!(entity.get_entity_type(), EntityType::Player | EntityType::Hidden) {
                            entity.set_pickup(client_tick);
                        }
                    }
                }
                NetworkEvent::SkillEffectNoDamage {
                    skill_id,
                    heal_amount,
                    destination_entity_id,
                    source_entity_id,
                    result,
                } => {
                    if result != 0
                        && should_display_heal_number(skill_id)
                        && heal_amount > 0
                        && let Some(entity) = self
                            .client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == destination_entity_id)
                            .or_else(|| {
                                self.client_state
                                    .try_follow(this_entity())
                                    .filter(|entity| entity.get_entity_id() == destination_entity_id)
                            })
                    {
                        self.particle_holder
                            .spawn_particle(Box::new(HealNumber::new(entity.get_position(), heal_amount.to_string())));
                    }

                    if result != 0 {
                        self.spawn_skill_name_bubble(source_entity_id, skill_id);

                        if let Some(recipe) = no_damage_skill_visual(skill_id) {
                            self.spawn_skill_visual(recipe, source_entity_id, destination_entity_id, None, None);
                        }
                        if let Some(recipe) = no_damage_sprite_visual(skill_id) {
                            self.spawn_skill_sprite_visual(recipe, source_entity_id, destination_entity_id, true);
                        }
                        if let Some(recipe) = no_damage_skill_sound(skill_id) {
                            self.play_skill_sound(recipe, source_entity_id, destination_entity_id, None);
                        }
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
                        .fill(&self.async_loader, items);
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .add_item(&self.async_loader, item);

                    // TODO: Update the selling items. If you pick up an item
                    // that you already have the sell window
                    // should allow you to sell the new
                    // amount of items.
                }
                NetworkEvent::ItemObtained {
                    item_id,
                    quantity,
                    is_identified,
                } => {
                    let name = self.library.get::<ItemName>(ItemNameKey { item_id, is_identified }).to_string();
                    let message = format!("You got {name} ({quantity}).");
                    self.client_state
                        .follow_mut(client_state().chat_messages())
                        .push(ChatMessage::new(message, MessageColor::Information));
                }
                NetworkEvent::InventoryItemRemoved { index, amount, .. } => {
                    self.client_state.follow_mut(client_state().inventory()).remove_item(index, amount);
                }
                NetworkEvent::SkillTree { skill_information } => {
                    *self.client_state.follow_mut(client_state().skill_tree().skills()) =
                        skill_information.into_iter().map(LearnedSkill::new).collect();
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_equipped_position(index, equipped_position);
                }
                NetworkEvent::ChangeJob { account_id, job_id } => {
                    let layout = self.async_loader.request_skill_tree_layout_load(job_id, client_tick);
                    *self.client_state.follow_mut(client_state().skill_tree_window().selected_tab()) = layout.tabs.len().saturating_sub(1);
                    *self.client_state.follow_mut(client_state().skill_tree().layout()) = layout;
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .clear();

                    let entity = self
                        .client_state
                        .follow_mut(client_state().entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .unwrap();

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    entity.set_job(&self.library, job_id);

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
                    self.clear_skill_cast_state();
                    self.clear_world_feedback();
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
                    let Some(position) = self.entity_position(entity_id) else {
                        continue;
                    };
                    let effect = match self.effect_loader.get_or_load(effect_path, &self.texture_loader) {
                        Ok(effect) => effect,
                        Err(_error) => {
                            #[cfg(feature = "debug")]
                            print_debug!(
                                "[{}] failed to load entity effect '{}': {:?}",
                                "error".red(),
                                effect_path,
                                _error
                            );
                            continue;
                        }
                    };
                    let frame_timer = effect.new_frame_timer();

                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Entity(entity_id, position),
                        Vector3::new(0.0, 9.0, 0.0),
                        next_dynamic_point_light_id(),
                        Vector3::new(0.0, 12.0, 0.0),
                        Color::WHITE,
                        50.0,
                        false,
                    )));
                }
                NetworkEvent::SpecialEffect { entity_id, effect_id } => {
                    if let Some(recipe) = special_effect_visual(effect_id) {
                        self.spawn_skill_visual(recipe, entity_id, entity_id, None, None);
                    }
                    if let Some(recipe) = special_effect_sprite_visual(effect_id) {
                        self.spawn_skill_sprite_visual(recipe, entity_id, entity_id, true);
                    }
                    if let Some(recipe) = special_effect_procedural_visual(effect_id) {
                        // ZC_NOTIFY_EFFECT carries no motion time. None of the
                        // direct-effect recipes lead an impact, so the flight
                        // time is unused; the floor keeps any future
                        // projectile visible rather than single-frame.
                        self.spawn_procedural_skill_visual(recipe, entity_id, entity_id, 0, 0.0, skill_projectile_flight_time(0));
                    }
                    if let Some(recipe) = special_effect_sound(effect_id) {
                        self.play_skill_sound(recipe, entity_id, entity_id, None);
                    }
                }
                NetworkEvent::GroundSkill {
                    skill_id,
                    source_entity_id,
                    position,
                    start_time,
                    ..
                } => {
                    if let Some(recipe) = ground_skill_visual(skill_id) {
                        self.queue_ground_skill_visual(
                            recipe,
                            source_entity_id,
                            position,
                            skill_effect_initial_delay(start_time, client_tick),
                        );
                    }
                }
                NetworkEvent::StatusChange { .. } => {}
                NetworkEvent::EntityStateChange {
                    entity_id, effect_state, ..
                } => {
                    if effect_state & OPTION_SIGHT == 0 {
                        self.particle_holder.remove_attached_sprite(entity_id, SIGHT_ATTACHMENT_KEY);
                    } else if !self.particle_holder.has_attached_sprite(entity_id, SIGHT_ATTACHMENT_KEY) {
                        self.spawn_skill_sprite_visual(sight_sprite_visual(), entity_id, entity_id, false);
                    }
                }
                NetworkEvent::AddSkillUnit {
                    entity_id,
                    creator_id,
                    unit_id,
                    position,
                    visible,
                    ..
                } => {
                    if visible != 0
                        && let Some(recipe) = skill_unit_visual(unit_id)
                    {
                        self.spawn_skill_visual(recipe, creator_id, entity_id, Some(position), Some(entity_id));
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

                    if let Some(job_id) = self.client_state.try_follow(this_entity()).map(Entity::get_job_id) {
                        for (index, hotkey) in hotkeys.into_iter().take(10).enumerate() {
                            match hotkey {
                                HotkeyState::Bound(hotkey) => {
                                    // TODO: Properly distinguish between skill and item.
                                    let skill_id = SkillId(hotkey.item_or_skill_id as u16);

                                    let mut skill = self.async_loader.request_learnable_skill_load(job_id, skill_id, client_tick);
                                    skill.maximum_level.0 = hotkey.quantity_or_skill_level;

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
                }
                NetworkEvent::OpenShop { items } => {
                    // Close the dialog. Some NPCs don't use the `BuyOrSellPacket` and instead use
                    // the regular `DialogMenuPacket`. When opening the shop that dialog should be
                    // closed.
                    self.client_state.follow_mut(client_state().dialog_window()).end();
                    self.interface.close_window_with_class(WindowClass::Dialog);

                    *self.client_state.follow_mut(client_state().shop_items()) = items
                        .into_iter()
                        .map(|item| self.async_loader.request_shop_item_metadata_load(item))
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

                        *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity {
                            entity_id: target_entity_id,
                        });
                    }
                }
                NetworkEvent::UpdateSkill {
                    skill_id,
                    skill_level,
                    spell_point_cost,
                    attack_range,
                    upgradable,
                } => {
                    self.client_state.follow_mut(client_state().skill_tree()).update_skill(
                        skill_id,
                        skill_level,
                        spell_point_cost,
                        attack_range,
                        upgradable,
                    );
                }
                NetworkEvent::RemoveSkill { skill_id } => {
                    let removed_armed_skill = self.armed_skill.is_some_and(|armed_skill| armed_skill.skill_id == skill_id);
                    let removed_active_skill = self
                        .active_continuous_skill
                        .is_some_and(|active_skill| active_skill.skill_id == skill_id);

                    if removed_armed_skill {
                        self.armed_skill = None;
                    }

                    if removed_active_skill {
                        self.active_continuous_skill = None;
                    }

                    if removed_armed_skill || removed_active_skill {
                        self.input_system.clear_hotbar_key_ownership();
                    }

                    self.client_state.follow_mut(client_state().skill_tree()).remove_skill(skill_id);
                    self.client_state
                        .follow_mut(client_state().skill_tree_window().chosen_skill_level())
                        .remove(&skill_id);
                }
            }
        }
    }

    fn clear_skill_cast_state(&mut self) {
        self.armed_skill = None;
        self.active_continuous_skill = None;
        self.input_system.clear_hotbar_key_ownership();
    }

    fn clear_world_feedback(&mut self) {
        self.particle_holder.clear();
        self.effect_holder.clear();
        self.pending_skill_effects.clear();
        self.pending_skill_sounds.clear();
        self.pending_skill_sprites.clear();
        self.pending_skill_damage_particles.clear();
        self.pending_procedural_skill_visuals.clear();
        self.point_light_manager.clear();
        self.audio_engine.clear_ambient_sound();
        self.audio_engine.clear_queued_sound_effects();
    }

    fn stop_active_continuous_skill(&mut self) {
        if let Some(active_skill) = self.active_continuous_skill.take() {
            let _ = self.networking_system.stop_channeling_skill(active_skill.skill_id);
        }
    }

    fn cast_resolved_skill(
        &mut self,
        skill_id: SkillId,
        skill_level: SkillLevel,
        skill_type: SkillType,
        activation: SkillActivation,
        source_slot: Option<HotbarSlot>,
    ) {
        self.armed_skill = None;

        if skill_type == SkillType::Passive {
            return;
        }

        let Some(player_entity_id) = self.client_state.try_follow(this_entity()).map(Entity::get_entity_id) else {
            self.stop_active_continuous_skill();
            return;
        };

        if skill_type == SkillType::SelfCast && skill_id == ROLLING_CUTTER_ID {
            let (stopped_skill_id, should_start) =
                activate_continuous_skill(&mut self.active_continuous_skill, skill_id, activation, source_slot);

            if let Some(stopped_skill_id) = stopped_skill_id {
                let _ = self.networking_system.stop_channeling_skill(stopped_skill_id);
            }

            if should_start
                && self
                    .networking_system
                    .cast_channeling_skill(skill_id, skill_level, player_entity_id)
                    .is_err()
            {
                self.active_continuous_skill = None;
            }

            return;
        }

        self.stop_active_continuous_skill();

        match skill_type {
            SkillType::SelfCast => {
                let _ = self.networking_system.cast_skill(skill_id, skill_level, player_entity_id);
            }
            skill_type @ (SkillType::Attack | SkillType::Support | SkillType::Ground | SkillType::Trap) => {
                self.armed_skill = Some(ArmedSkill {
                    skill_id,
                    skill_level,
                    skill_type,
                });
            }
            SkillType::Passive => unreachable!(),
        }
    }

    fn resolve_armed_skill_target(&self, picker_target: PickerTarget) -> Option<SkillCastTarget> {
        let armed_skill = self.armed_skill?;
        let player_entity_id = self.client_state.try_follow(this_entity()).map(Entity::get_entity_id);

        resolve_skill_cast_target(
            armed_skill.skill_type,
            picker_target,
            player_entity_id,
            |entity_id| {
                self.client_state
                    .follow(client_state().entities())
                    .iter()
                    .find(|entity| entity.get_entity_id() == entity_id)
                    .map(Entity::get_tile_position)
            },
            |entity_id| {
                self.client_state
                    .follow(client_state().ground_items())
                    .iter()
                    .find(|item| item.get_entity_id() == entity_id)
                    .map(GroundItem::get_tile_position)
            },
        )
    }

    /// Returns whether or not the interface is focused.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn process_user_events(
        &mut self,
        input_report: &InputReport,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] delta_time: f32,
    ) -> bool {
        self.interface.process_events(&mut self.input_event_buffer);

        let interface_has_focus = self.interface.has_focus();
        let cancelled_armed_skill = self.armed_skill.is_some() && self.input_system.escape_pressed();

        if cancelled_armed_skill {
            self.armed_skill = None;
        }

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

        self.input_system.handle_hotbar_key_releases(&mut self.input_event_buffer);

        let events: Vec<InputEvent> = self.input_event_buffer.drain(..).collect();

        for event in events {
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
                    self.armed_skill = None;
                    self.stop_active_continuous_skill();
                    self.input_system.clear_hotbar_key_ownership();
                    let _ = self.networking_system.log_out();
                }
                InputEvent::LogOutCharacter => {
                    self.armed_skill = None;
                    self.stop_active_continuous_skill();
                    self.input_system.clear_hotbar_key_ownership();
                    self.networking_system.disconnect_from_character_server();
                }
                InputEvent::Exit => {
                    self.armed_skill = None;
                    self.stop_active_continuous_skill();
                    self.input_system.clear_hotbar_key_ownership();
                    SHUTDOWN_SIGNAL.store(true, Ordering::SeqCst);
                }
                InputEvent::ZoomCamera { zoom_factor } => self.player_camera.soft_zoom(zoom_factor),
                InputEvent::RotateCamera { rotation } => self.player_camera.soft_rotate(rotation),
                InputEvent::ResetCameraRotation => self.player_camera.reset_rotation(),
                InputEvent::ToggleMenuWindow => {
                    if !cancelled_armed_skill && self.client_state.try_follow(this_entity()).is_some() {
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
                            false => self.interface.open_window(SkillTreeWindow::new(
                                client_state().skill_tree_window(),
                                client_state().skill_tree().layout(),
                                client_state().skill_tree().skills(),
                                this_player().manually_asserted().skill_points(),
                            )),
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

                    // Unbuffer any buffered action.
                    *self.client_state.follow_mut(client_state().buffered_action()) = None;
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
                                let buffered_action = self.client_state.follow_mut(client_state().buffered_action());

                                if auto_attack {
                                    *buffered_action = Some(BufferedAction::AttackEntity { entity_id });
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
                InputEvent::PickUpItem { entity_id } => {
                    self.mouse_cursor.set_state(MouseCursorState::PickUpItem, client_tick);

                    if let Some(map) = &self.map {
                        let player_position = self.client_state.try_follow(this_entity()).map(|player| player.get_tile_position());
                        let item_position = self
                            .client_state
                            .follow(client_state().ground_items())
                            .iter()
                            .find(|item| item.get_entity_id() == entity_id)
                            .map(|item| item.get_tile_position());

                        if let (Some(player_position), Some(item_position)) = (player_position, item_position) {
                            if player_position
                                .x
                                .abs_diff(item_position.x)
                                .max(player_position.y.abs_diff(item_position.y))
                                <= ITEM_PICKUP_RANGE.0
                            {
                                let _ = self.networking_system.pick_up_item(entity_id);

                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            } else if let Some(path) =
                                self.path_finder
                                    .find_walkable_path_in_range(&**map, player_position, item_position, ITEM_PICKUP_RANGE)
                                && let Some(nearest_tile) = path.last()
                            {
                                let _ = self.networking_system.player_move(WorldPosition {
                                    x: nearest_tile.x,
                                    y: nearest_tile.y,
                                    direction: Direction::North,
                                });

                                *self.client_state.follow_mut(client_state().buffered_action()) =
                                    Some(BufferedAction::PickUpItem { entity_id });
                            } else {
                                *self.client_state.follow_mut(client_state().buffered_action()) = None;
                            }
                        }
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
                InputEvent::SendEmotion { emotion } => {
                    let _ = self.networking_system.send_emotion(emotion);
                }
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
                    (SkillSource::Hotbar { slot }, SkillSource::SkillTree) => {
                        self.client_state
                            .follow_mut(client_state().hotbar())
                            .clear_slot(&mut self.networking_system, slot);
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
                InputEvent::CastSkill { slot, activation } => {
                    let skill = self
                        .client_state
                        .follow(client_state().hotbar())
                        .get_skill_in_slot(slot)
                        .as_ref()
                        .and_then(|learnable_skill| {
                            self.client_state
                                .follow(client_state().skill_tree().skills())
                                .iter()
                                .find(|learned_skill| {
                                    learned_skill.skill_id == learnable_skill.skill_id
                                        && learned_skill.skill_level.0 >= learnable_skill.maximum_level.0
                                })
                                .map(|learned_skill| {
                                    (
                                        learnable_skill.skill_id,
                                        learnable_skill.maximum_level,
                                        learned_skill.skill_type,
                                    )
                                })
                        });

                    if let Some((skill_id, skill_level, skill_type)) = skill {
                        self.cast_resolved_skill(skill_id, skill_level, skill_type, activation, Some(slot));
                    }
                }
                InputEvent::CastLearnedSkill {
                    skill_id,
                    skill_level,
                    skill_type,
                    activation,
                } => {
                    self.cast_resolved_skill(skill_id, skill_level, skill_type, activation, None);
                }
                InputEvent::CastSkillAt {
                    skill_id,
                    skill_level,
                    target,
                } => match target {
                    SkillCastTarget::Entity(entity_id) => {
                        let _ = self.networking_system.cast_skill(skill_id, skill_level, entity_id);
                    }
                    SkillCastTarget::Ground(position) => {
                        let _ = self.networking_system.cast_ground_skill(skill_id, skill_level, position);
                    }
                },
                InputEvent::StopSkill { slot } => {
                    if let Some(skill_id) = release_continuous_skill(&mut self.active_continuous_skill, slot) {
                        let _ = self.networking_system.stop_channeling_skill(skill_id);
                    }
                }
                InputEvent::CycleSkillLevel { slot } => {
                    let learned_maximum_level = self
                        .client_state
                        .follow(client_state().hotbar())
                        .get_skill_in_slot(slot)
                        .as_ref()
                        .and_then(|skill| {
                            self.client_state
                                .follow(client_state().skill_tree().skills())
                                .iter()
                                .find(|learned_skill| learned_skill.skill_id == skill.skill_id)
                                .map(|learned_skill| learned_skill.skill_level)
                        });

                    if let Some(learned_maximum_level) = learned_maximum_level {
                        self.client_state.follow_mut(client_state().hotbar()).cycle_skill_level(
                            &mut self.networking_system,
                            slot,
                            learned_maximum_level,
                        );
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
                InputEvent::DistributePointsForSkill { skill_id } => {
                    if let Some(available_skill_points) = self.client_state.try_follow(this_player().skill_points()).copied() {
                        let job_id = self.client_state.follow(this_entity().manually_asserted()).get_job_id();
                        let pending_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .len();
                        let available_skill_points = (available_skill_points as usize).saturating_sub(pending_skill_points);

                        let skill_information = self.library.get::<SkillListInformation>(skill_id);
                        let learned_skills = self.client_state.follow(client_state().skill_tree().skills());

                        let mut new_skill_points = self
                            .client_state
                            .follow(client_state().skill_tree_window().pending_skill_points())
                            .clone();

                        let current_skill_level = learned_skills
                            .iter()
                            .find(|skill| skill.skill_id == skill_id)
                            .map(|skill| skill.skill_level.0)
                            .unwrap_or_default()
                            + new_skill_points
                                .iter()
                                .filter(|pending_skill_level| **pending_skill_level == skill_id)
                                .count() as u16;

                        // If the skill is already at max level we don't do anything
                        if current_skill_level < skill_information.maximum_level.0 {
                            let target_skill_level = SkillLevel(current_skill_level + 1);

                            bring_skill_to_level(
                                &mut new_skill_points,
                                &self.library,
                                learned_skills,
                                job_id,
                                skill_id,
                                target_skill_level,
                                available_skill_points,
                            );

                            *self
                                .client_state
                                .follow_mut(client_state().skill_tree_window().pending_skill_points()) = new_skill_points;
                        }
                    }
                }
                InputEvent::LevelUpSkills { skill_ids } => {
                    for skill_id in skill_ids {
                        if self.networking_system.level_up_skill(skill_id).is_err() {
                            break;
                        }
                    }
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
                    if let Some(map) = self.map.as_ref() {
                        let inspecting_maps = self.client_state.follow_mut(client_state().inspecting_maps());
                        let map_data_path = state::prepare_map_inspection(inspecting_maps, map.get_map_data());

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
                InputEvent::CameraMoveForward => self.debug_camera.move_forward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveBackward => self.debug_camera.move_backward(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveLeft => self.debug_camera.move_left(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveRight => self.debug_camera.move_right(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraMoveUp => self.debug_camera.move_up(delta_time),
                #[cfg(feature = "debug")]
                InputEvent::CameraAccelerate => self.debug_camera.accelerate(),
                #[cfg(feature = "debug")]
                InputEvent::CameraDecelerate => self.debug_camera.decelerate(),
                #[cfg(feature = "debug")]
                InputEvent::InspectFrame { measurement } => self.interface.open_window(FrameInspectorWindow::new(measurement)),
            }
        }

        interface_has_focus
    }

    #[cfg(feature = "debug")]
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_debug_windows(&mut self, delta_time: f64) {
        let is_packet_inspector_open = self.interface.is_window_with_class_open(WindowClass::PacketInspector);
        self.client_state
            .follow_mut(client_state().packet_history())
            .update(is_packet_inspector_open);

        self.client_state.follow_mut(client_state().cache_statistics()).update(
            delta_time,
            &self.map_loader,
            &self.texture_loader,
            &self.sprite_loader,
            &self.font_loader,
            &self.audio_engine,
            &self.action_loader,
            &self.animation_loader,
            &self.effect_loader,
        );
    }

    /// Drain finished async loads from the loader thread and apply their
    /// results to client state. This is the only place that promotes
    /// `self.map` from `None` to `Some` (when a map load completes), so it
    /// must run before the `self.map` check in [`Self::update_and_render`].
    ///
    /// Called as late as possible in the frame to give the loader thread the
    /// maximum window to finish in-flight work.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_loaded_resources(&mut self, client_tick: ClientTick) {
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
                    } else if let Some(entity) = self
                        .client_state
                        .follow_mut(client_state().dead_entities())
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        entity.set_animation_data(animation_data);
                    } else if let Some(item) = self
                        .client_state
                        .follow_mut(client_state().ground_items())
                        .iter_mut()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        item.set_animation_data(animation_data);
                    }
                }
                (LoaderId::ItemSprite(item_id), LoadableResource::ItemSprite { texture }) => {
                    self.client_state
                        .follow_mut(client_state().shop_items())
                        .iter_mut()
                        .filter(|item| item.item_id == item_id)
                        .for_each(|item| item.metadata.texture = Some(texture.clone()));

                    self.client_state
                        .follow_mut(client_state().inventory())
                        .update_item_sprite(item_id, texture);
                }
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
                                // `manually_asserted` is safe because we are in the branch where `this_player`
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
                (LoaderId::SkillSprite(skill_id), LoadableResource::SkillSprite { sprite }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar().skills())
                        .iter_mut()
                        .filter_map(|slot| slot.as_mut())
                        .filter(|skill| skill.skill_id == skill_id)
                        .for_each(|skill| {
                            skill.sprite = Some(sprite.clone());
                        });

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.sprite = Some(sprite);
                    }
                }
                (LoaderId::SkillActions(skill_id), LoadableResource::SkillActions { actions }) => {
                    self.client_state
                        .follow_mut(client_state().hotbar().skills())
                        .iter_mut()
                        .filter_map(|slot| slot.as_mut())
                        .filter(|skill| skill.skill_id == skill_id)
                        .for_each(|skill| {
                            skill.actions = Some(actions.clone());
                        });

                    if let Some(skill) = self
                        .client_state
                        .follow_mut(client_state().skill_tree().layout().tabs())
                        .iter_mut()
                        .flat_map(|tab| tab.skills.values_mut())
                        .find(|skill| skill.skill_id == skill_id)
                    {
                        skill.actions = Some(actions);
                    }
                }
                _ => {}
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_main_camera(&mut self, window_size: ScreenSize, delta_time: f64, #[cfg(feature = "debug")] render_options: &RenderOptions) {
        if self.client_state.try_follow(this_entity()).is_some() {
            self.player_camera.update(delta_time);
            self.player_camera.generate_view_projection(window_size);
        } else {
            self.start_camera.update(delta_time);
            self.start_camera.generate_view_projection(window_size);
        }

        #[cfg(feature = "debug")]
        self.interface_renderer.update_render_options(render_options);

        #[cfg(feature = "debug")]
        if render_options.use_debug_camera {
            self.debug_camera.generate_view_projection(window_size);
        }
    }

    /// Per-frame tick for all entities, dead entities, and ground items.
    ///
    /// Must be called after [`Self::handle_network_events`] so the entity
    /// set reflects the latest spawn/despawn/move packets. Running this
    /// with a stale entity list would tick entities that the network has
    /// already removed, or miss new ones that just appeared (and on a map
    /// transition, would tick entities from the previous map).
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_entities(
        &mut self,
        map: &Map,
        currently_playing: bool,
        client_tick: ClientTick,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
        let current_camera: &(dyn Camera + Send + Sync) = match currently_playing {
            #[cfg(feature = "debug")]
            _ if render_options.use_debug_camera => &self.debug_camera,
            true => &self.player_camera,
            false => &self.start_camera,
        };

        self.client_state
            .follow_mut(client_state().entities())
            .iter_mut()
            .for_each(|entity| entity.update(&self.audio_engine, map, current_camera, client_tick));

        self.client_state
            .follow_mut(client_state().dead_entities())
            .iter_mut()
            .for_each(|entity| {
                entity.update(&self.audio_engine, map, current_camera, client_tick);

                if entity.is_death_animation_over() && !entity.is_fading() {
                    entity.fade_out(DisappearanceReason::Died, client_tick);
                }
            });

        self.client_state
            .follow_mut(client_state().ground_items())
            .iter_mut()
            .for_each(|item| item.update(client_tick));

        // Remove entities that have finished fading out.
        self.client_state
            .follow_mut(client_state().entities())
            .retain(|entity| !entity.should_be_removed(client_tick));

        self.client_state
            .follow_mut(client_state().dead_entities())
            .retain(|entity| !entity.should_be_removed(client_tick));

        self.client_state
            .follow_mut(client_state().ground_items())
            .retain(|item| !item.should_be_removed(client_tick));
    }

    /// Fire any action that the player buffered while out of range or while
    /// still moving (attack, pick up item). Must be called after
    /// [`Self::update_entities`] so that the player's `stopped_moving` state
    /// reflects this frame.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn process_buffered_action(&mut self) {
        let Some(true) = self.client_state.try_follow(this_entity()).map(|player| player.stopped_moving()) else {
            return;
        };

        let Some(buffered_action) = self.client_state.follow_mut(client_state().buffered_action()).take() else {
            return;
        };

        match buffered_action {
            BufferedAction::AttackEntity { entity_id } => {
                let _ = self.networking_system.player_attack(entity_id);

                let auto_attack = *self.client_state.follow(client_state().game_settings().auto_attack());
                if auto_attack {
                    *self.client_state.follow_mut(client_state().buffered_action()) = Some(BufferedAction::AttackEntity { entity_id });
                }
            }
            BufferedAction::PickUpItem { entity_id } => {
                if self
                    .client_state
                    .follow(client_state().ground_items())
                    .iter()
                    .any(|item| item.get_entity_id() == entity_id)
                {
                    let _ = self.networking_system.pick_up_item(entity_id);
                }
            }
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn update_audio_engine(&self, current_camera: &dyn Camera) {
        // We set the listener roughly at ear height.
        const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
        let listener = current_camera.focus_point() + EAR_HEIGHT;

        self.audio_engine
            .set_spatial_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
        self.audio_engine.update();
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn create_point_light_set<'a>(
        point_light_manager: &'a mut PointLightManager,
        point_light_set_buffer: &mut ResourceSetBuffer<LightSourceKey>,
        map: &Map,
        effect_holder: &EffectHolder,
        current_camera: &dyn Camera,
        lighting_mode: LightingMode,
    ) -> PointLightSet<'a> {
        point_light_manager.prepare();

        effect_holder.register_point_lights(point_light_manager, current_camera);

        map.register_point_lights(point_light_manager, point_light_set_buffer, current_camera);

        match lighting_mode {
            LightingMode::Classic => point_light_manager.create_point_light_set(0),
            LightingMode::Enhanced => point_light_manager.create_point_light_set(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS),
        }
    }

    /// Applies any buffered drag from the previous frame's input and draws the
    /// per-frame top-level overlays (FPS counter when in debug, mouse cursor).
    /// Must be called after the laid-out interface frame has been dropped so
    /// that `self.interface` is no longer borrowed.
    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_ui_overlays(
        &mut self,
        input_report: &InputReport,
        scaling: Scaling,
        #[cfg(feature = "debug")] render_options: &RenderOptions,
    ) {
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
                self.armed_skill.map(|armed_skill| armed_skill.skill_level),
                *self.client_state.follow(client_state().world_theme().cursor().color()),
                self.client_state.follow(client_state().interface_settings().scaling()).get_factor(),
            );
        }
    }

    #[inline(always)]
    fn update_and_render(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            return;
        }

        if SHUTDOWN_SIGNAL.load(Ordering::SeqCst) {
            event_loop.exit();
            return;
        }

        // Clear the previous render instructions so we can rendering the new frame.
        self.clear_render_instructions();

        // It is important that we first apply any changes that were dispatched during
        // the last frame.
        self.update_client_state();

        // We can only apply the graphic changes and reconfigure the surface once the
        // previous image was presented. Moving this function to the end of the
        // function results in surface configuration errors under DX12.
        self.update_settings();

        // TODO: Shouldn't this happen later? After the scaling has been potentially
        // changed by the UI.
        let scaling = *self.client_state.follow(client_state().interface_settings().scaling());
        self.update_interface_scaling(scaling);

        let FrameTimers {
            delta_time,
            client_tick,
            animation_timer_ms,
        } = self.game_timer.update();

        let input_report = self.input_system.update_delta(client_tick);

        self.request_entity_details(&input_report);

        self.handle_network_events(client_tick);

        let interface_has_focus = self.process_user_events(
            &input_report,
            client_tick,
            #[cfg(feature = "debug")]
            (delta_time as f32),
        );

        // Some debug windows, such as the packet history or cache statistics, require
        // special update logic.
        #[cfg(feature = "debug")]
        self.update_debug_windows(delta_time);

        // We run this last to give the loader thread as much time as possible to
        // complete the loading. When starting the actual render process, we
        // can't modify resources anymore until the next frame.
        self.update_loaded_resources(client_tick);

        #[cfg(feature = "debug")]
        let render_options = *self.client_state.follow(client_state().render_options());

        let screen_size = self.graphics_engine.get_window_size().into();
        let currently_playing = self.client_state.try_follow(this_player()).is_some();

        self.mouse_cursor.update(client_tick);

        // Acquire the swapchain image as late as possible so all CPU-side
        // preparation overlaps with the previous frame's GPU work. After this
        // point we may not reconfigure the surface (see `update_settings`).
        let maybe_frame = self.graphics_engine.wait_for_next_frame();

        // If we don't have a map, the rendering ends here.
        let Some(map) = self.map.clone() else {
            if let Some(frame) = maybe_frame {
                self.graphics_engine.render_next_frame(frame, Default::default());
            }

            return;
        };

        self.update_entities(
            &map,
            currently_playing,
            client_tick,
            #[cfg(feature = "debug")]
            &render_options,
        );

        self.process_buffered_action();

        self.update_main_camera(
            screen_size,
            delta_time,
            #[cfg(feature = "debug")]
            &render_options,
        );

        map.advance_videos(&self.queue, delta_time);

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            self.player_camera.set_smoothed_focus_point(player.get_position());
        }

        // Update particles.
        self.update_pending_skill_effects(delta_time as f32);
        self.update_pending_skill_sounds(delta_time as f32);
        self.update_pending_skill_sprites(delta_time as f32);
        self.update_pending_skill_damage_particles(delta_time as f32);
        let local_entity = self
            .client_state
            .try_follow(this_entity())
            .map(|entity| (entity.get_entity_id(), entity.get_position()));
        self.particle_holder.update_with_local_entity(
            self.client_state.follow(client_state().entities()),
            local_entity,
            delta_time as f32,
        );
        // Delayed procedural particles are spawned after existing particles
        // advance. Their per-event overshoot is applied on creation, preserving
        // multi-hit phase spacing even when one frame crosses several deadlines.
        self.update_pending_procedural_skill_visuals(delta_time as f32);
        self.effect_holder.update_with_local_entity(
            self.client_state.follow(client_state().entities()),
            local_entity,
            delta_time as f32,
        );

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

        #[cfg(feature = "debug")]
        update_shadow_camera_measurement.stop();

        self.update_audio_engine(current_camera);

        #[cfg(feature = "debug")]
        let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

        #[cfg(feature = "debug")]
        let hovered_marker_identifier = match input_report.mouse_target {
            PickerTarget::Marker(marker_identifier) => Some(marker_identifier),
            _ => None,
        };

        let mut armed_skill = self.armed_skill;
        let armed_skill_target = self.resolve_armed_skill_target(input_report.mouse_target);

        let point_light_set = Self::create_point_light_set(
            &mut self.point_light_manager,
            &mut self.point_light_set_buffer,
            &map,
            &self.effect_holder,
            current_camera,
            lighting_mode,
        );

        #[cfg(feature = "debug")]
        prepare_frame_measurement.stop();

        let mut indicator_instruction = None;
        let mut water_instruction = None;

        let mouse_mode = self.interface.get_mouse_mode();
        let is_mouse_mode_default = mouse_mode.is_default();
        let last_walking_destination = mouse_mode.walk_destination();
        let mut clear_armed_skill = false;

        let mut interface_frame = {
            #[cfg(feature = "debug")]
            profile_block!("user interface");

            let is_rotating_camera = mouse_mode.is_rotating_camera();
            let is_grabbing = mouse_mode.is_grabbing();
            let is_chat_open = self.interface.is_window_with_class_open(WindowClass::Chat);

            let mut interface_frame = self
                .interface
                .lay_out_windows(&self.client_state, scaling.get_factor(), input_report.mouse_position);

            // We can only decide what to do with the user input once we know if the mouse
            // is hovering a window, so we buffer any actions for the next frame.

            let is_interface_hovered = interface_frame.is_interface_hovered();

            let cursor_state = match input_report.mouse_target {
                _ if is_rotating_camera => MouseCursorState::RotateCamera,
                _ if is_grabbing => MouseCursorState::GrabResource,
                _ if armed_skill.is_some() && !is_interface_hovered => MouseCursorState::Target,
                PickerTarget::Entity(entity_id) if !is_interface_hovered => {
                    if self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .any(|item| item.get_entity_id() == entity_id)
                    {
                        MouseCursorState::HoverItem
                    } else {
                        self.client_state
                            .follow(client_state().entities())
                            .iter()
                            .find(|entity| entity.get_entity_id() == entity_id)
                            .map(|entity| match entity.get_entity_type() {
                                EntityType::Npc => MouseCursorState::Dialog,
                                EntityType::Warp => MouseCursorState::Warp,
                                EntityType::Monster => MouseCursorState::Attack,
                                _ => MouseCursorState::Default,
                            })
                            .unwrap_or(MouseCursorState::Default)
                    }
                }
                _ => MouseCursorState::Default,
            };
            self.mouse_cursor.set_state(cursor_state, client_tick);

            if let Some(mouse_button) = input_report.mouse_click {
                if armed_skill.is_some() && is_skill_target_cancellation(mouse_button) {
                    interface_frame.unfocus();
                    armed_skill = None;
                    clear_armed_skill = true;
                } else if is_interface_hovered {
                    interface_frame.click(&self.client_state, mouse_button);
                } else if armed_skill.is_some() && is_skill_target_confirmation(mouse_button) {
                    interface_frame.unfocus();

                    if let Some((armed_skill, target)) = take_resolved_armed_skill(&mut armed_skill, armed_skill_target) {
                        self.input_event_buffer.push(InputEvent::CastSkillAt {
                            skill_id: armed_skill.skill_id,
                            skill_level: armed_skill.skill_level,
                            target,
                        });
                        clear_armed_skill = true;
                    }
                } else {
                    interface_frame.unfocus();

                    if mouse_button == MouseButton::Left {
                        match input_report.mouse_target {
                            PickerTarget::Nothing => {}
                            PickerTarget::Entity(entity_id) => {
                                let is_ground_item = self
                                    .client_state
                                    .follow(client_state().ground_items())
                                    .iter()
                                    .any(|item| item.get_entity_id() == entity_id);

                                if is_ground_item {
                                    self.input_event_buffer.push(InputEvent::PickUpItem { entity_id })
                                } else {
                                    self.input_event_buffer.push(InputEvent::PlayerInteract { entity_id })
                                }
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

        let is_interface_hovered = interface_frame.is_interface_hovered();
        let skill_target_highlight = match (is_interface_hovered, armed_skill, input_report.mouse_target, armed_skill_target) {
            (false, Some(_), PickerTarget::Entity(entity_id), Some(_)) => Some((entity_id, Color::rgb_u8(255, 130, 130))),
            _ => None,
        };

        if clear_armed_skill {
            self.armed_skill = None;
        }

        {
            let mut render_context = MapRenderContext {
                map: &map,
                current_camera,
                point_light_set: &point_light_set,
                client_state: &self.client_state,
                library: &self.library,
                mouse_position: input_report.mouse_position,
                mouse_target: input_report.mouse_target,
                screen_size,
                scaling,
                client_tick,
                animation_timer_ms,
                currently_playing,
                is_mouse_mode_default,
                is_interface_hovered,
                last_walking_destination,
                skill_target_highlight,
                buffered_action: *self.client_state.follow(client_state().buffered_action()),
                #[cfg(feature = "debug")]
                render_options: &render_options,
                #[cfg(feature = "debug")]
                hovered_marker_identifier,
                #[cfg(feature = "debug")]
                pathing_texture_set: &self.pathing_texture_set,
                #[cfg(feature = "debug")]
                tile_texture_set: &self.tile_texture_set,
                #[cfg(feature = "debug")]
                player_camera: &self.player_camera,
                #[cfg(feature = "debug")]
                start_camera: &self.start_camera,
                model_batches: &mut self.model_batches,
                model_instructions: &mut self.model_instructions,
                entity_instructions: &mut self.entity_instructions,
                directional_shadow_camera: &mut self.directional_shadow_camera,
                directional_shadow_model_batches: &mut self.directional_shadow_model_batches,
                directional_shadow_model_instructions: &mut self.directional_shadow_model_instructions,
                directional_shadow_entity_instructions: &mut self.directional_shadow_entity_instructions,
                point_shadow_camera: &mut self.point_shadow_camera,
                point_shadow_model_instructions: &mut self.point_shadow_model_instructions,
                point_light_with_shadow_instructions: &mut self.point_light_with_shadow_instructions,
                point_light_instructions: &mut self.point_light_instructions,
                directional_shadow_object_set_buffer: &mut self.directional_shadow_object_set_buffer,
                point_shadow_object_set_buffer: &mut self.point_shadow_object_set_buffer,
                deferred_object_set_buffer: &mut self.deferred_object_set_buffer,
                indicator_instruction: &mut indicator_instruction,
                water_instruction: &mut water_instruction,
                particle_holder: &mut self.particle_holder,
                effect_holder: &mut self.effect_holder,
                effect_renderer: &mut self.effect_renderer,
                bottom_interface_renderer: &self.bottom_interface_renderer,
                middle_interface_renderer: &mut self.middle_interface_renderer,
                #[cfg(feature = "debug")]
                aabb_instructions: &mut self.aabb_instructions,
                #[cfg(feature = "debug")]
                circle_instructions: &mut self.circle_instructions,
                #[cfg(feature = "debug")]
                rectangle_instructions: &mut self.rectangle_instructions,
                #[cfg(feature = "debug")]
                bounding_box_object_set_buffer: &mut self.bounding_box_object_set_buffer,
                #[cfg(feature = "debug")]
                debug_marker_renderer: &mut self.debug_marker_renderer,
            };

            #[cfg(feature = "debug")]
            render_context.render_markers();
            render_context.render_directional_shadows();
            render_context.render_point_lights();
            render_context.render_geometry();
            #[cfg(feature = "debug")]
            render_context.render_bounding_boxes();
            render_context.render_world_overlays();
        }

        let in_game_theme_path = client_state().in_game_theme().tooltip();
        let menu_theme_path = client_state().menu_theme().tooltip();
        let tooltip_theme = match currently_playing {
            true => self.client_state.follow(in_game_theme_path),
            false => self.client_state.follow(menu_theme_path),
        };

        interface_frame.render(
            &self.client_state,
            &self.interface_renderer,
            tooltip_theme,
            input_report.mouse_position,
        );

        drop(interface_frame);

        self.render_ui_overlays(
            &input_report,
            scaling,
            #[cfg(feature = "debug")]
            &render_options,
        );

        let picker_position = ScreenPosition {
            left: input_report.mouse_position.left.clamp(0.0, screen_size.width),
            top: input_report.mouse_position.top.clamp(0.0, screen_size.height),
        };

        let uniforms = Uniforms {
            view_matrix,
            projection_matrix,
            camera_position,
            animation_timer_ms,
            ambient_light_color: map.ambient_light_color(),
            enhanced_lighting: lighting_mode == LightingMode::Enhanced,
            shadow_method,
            shadow_detail,
            use_sdsm,
            sdsm_enabled,
        };

        let interface_instructions = self.interface_renderer.get_instructions();
        let bottom_layer_instructions = self.bottom_interface_renderer.get_instructions();
        let middle_layer_instructions = self.middle_interface_renderer.get_instructions();
        let top_layer_instructions = self.top_interface_renderer.get_instructions();

        let directional_light = DirectionalLightInstruction {
            view_projection_matrix: self.directional_shadow_camera.view_projection_matrix(),
            direction: directional_light_direction,
            color: directional_light_color,
        };

        let render_instruction = RenderInstruction {
            show_interface: self.show_interface,
            picker_position,
            uniforms,
            indicator: indicator_instruction,
            interface: interface_instructions.as_slice(),
            bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
            middle_layer_rectangles: middle_layer_instructions.as_slice(),
            top_layer_rectangles: top_layer_instructions.as_slice(),
            directional_light,
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

        if let Some(frame) = maybe_frame {
            self.graphics_engine.render_next_frame(frame, render_instruction);
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
                self.armed_skill = None;
                self.stop_active_continuous_skill();
                self.input_system.clear_hotbar_key_ownership();
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
                    self.armed_skill = None;
                    self.stop_active_continuous_skill();
                    self.input_system.clear_hotbar_key_ownership();
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
                #[cfg(feature = "debug")]
                let _measurement = threads::Main::start_frame();

                self.update_and_render(event_loop);

                if let Some(window) = self.window.as_mut() {
                    window.request_redraw();
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

/// Bundles all the borrows needed by the per-frame rendering pipeline so that
/// they can be passed as a single `self` argument.
struct MapRenderContext<'a, 'm: 'a> {
    map: &'m Map,
    current_camera: &'a (dyn Camera + Send + Sync),
    point_light_set: &'a PointLightSet<'a>,
    client_state: &'a State<ClientState>,
    library: &'a Library,
    mouse_position: ScreenPosition,
    mouse_target: PickerTarget,
    screen_size: ScreenSize,
    scaling: Scaling,
    client_tick: ClientTick,
    animation_timer_ms: f32,
    currently_playing: bool,
    is_mouse_mode_default: bool,
    is_interface_hovered: bool,
    last_walking_destination: Option<TilePosition>,
    skill_target_highlight: Option<(EntityId, Color)>,
    buffered_action: Option<BufferedAction>,
    #[cfg(feature = "debug")]
    render_options: &'a RenderOptions,
    #[cfg(feature = "debug")]
    hovered_marker_identifier: Option<MarkerIdentifier>,
    #[cfg(feature = "debug")]
    pathing_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    tile_texture_set: &'a Arc<TextureSet>,
    #[cfg(feature = "debug")]
    player_camera: &'a PlayerCamera,
    #[cfg(feature = "debug")]
    start_camera: &'a StartCamera,

    // Mutable rendering state
    model_batches: &'a mut Vec<ModelBatch>,
    model_instructions: &'a mut Vec<ModelInstruction>,
    entity_instructions: &'a mut Vec<EntityInstruction>,
    directional_shadow_camera: &'a mut DirectionalShadowCamera,
    directional_shadow_model_batches: &'a mut [Vec<ModelBatch>; PARTITION_COUNT],
    directional_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    directional_shadow_entity_instructions: &'a mut [Vec<EntityInstruction>; PARTITION_COUNT],
    point_shadow_camera: &'a mut PointShadowCamera,
    point_shadow_model_instructions: &'a mut Vec<ModelInstruction>,
    point_light_with_shadow_instructions: &'a mut Vec<PointLightWithShadowInstruction>,
    point_light_instructions: &'a mut Vec<PointLightInstruction>,
    directional_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    indicator_instruction: &'a mut Option<IndicatorInstruction>,
    water_instruction: &'a mut Option<WaterInstruction<'m>>,
    particle_holder: &'a mut ParticleHolder,
    effect_holder: &'a mut EffectHolder,
    effect_renderer: &'a mut EffectRenderer,
    bottom_interface_renderer: &'a GameInterfaceRenderer,
    middle_interface_renderer: &'a mut GameInterfaceRenderer,
    #[cfg(feature = "debug")]
    aabb_instructions: &'a mut Vec<DebugAabbInstruction>,
    #[cfg(feature = "debug")]
    circle_instructions: &'a mut Vec<DebugCircleInstruction>,
    #[cfg(feature = "debug")]
    rectangle_instructions: &'a mut Vec<DebugRectangleInstruction>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: &'a mut ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    debug_marker_renderer: &'a mut DebugMarkerRenderer,
}

impl<'a, 'm: 'a> MapRenderContext<'a, 'm> {
    #[inline(always)]
    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    fn render_markers(&mut self) {
        let entities = self.client_state.follow(client_state().entities());

        self.map.render_markers(
            self.debug_marker_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );

        self.map.render_markers(
            self.middle_interface_renderer,
            self.current_camera,
            self.render_options,
            entities,
            self.point_light_set,
            self.hovered_marker_identifier,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_directional_shadows(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        for partition_index in 0..PARTITION_COUNT {
            let partition_camera = self.directional_shadow_camera.get_partition_camera(partition_index);

            let object_set = self.map.cull_objects_with_frustum(
                &partition_camera,
                self.directional_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                self.render_options.frustum_culling,
            );

            let offset = self.directional_shadow_model_instructions.len();
            let model_batches = &mut self.directional_shadow_model_batches[partition_index];
            let entity_instructions = &mut self.directional_shadow_entity_instructions[partition_index];

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
            self.map.render_objects(
                self.directional_shadow_model_instructions,
                &object_set,
                self.animation_timer_ms,
                &partition_camera,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map))]
            self.map.render_ground(self.directional_shadow_model_instructions);

            let count = self.directional_shadow_model_instructions.len() - offset;

            model_batches.push(ModelBatch {
                offset,
                count,
                texture_set: self.map.get_texture_set().clone(),
                vertex_buffer: self.map.get_model_vertex_buffer().clone(),
                index_buffer: self.map.get_model_index_buffer().clone(),
            });

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map_tiles))]
            self.map
                .render_overlay_tiles(self.directional_shadow_model_instructions, model_batches, self.tile_texture_set);

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_pathing))]
            self.map.render_entity_pathing(
                self.directional_shadow_model_instructions,
                model_batches,
                entities,
                self.pathing_texture_set,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_ground_items))]
            self.map
                .render_ground_items(entity_instructions, ground_items, &partition_camera, self.client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_entities(entity_instructions, entities, &partition_camera, self.client_tick, None);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
            self.map
                .render_dead_entities(entity_instructions, dead_entities, &partition_camera, self.client_tick);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_point_lights(&mut self) {
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights(self.point_light_instructions);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.enable_point_lights))]
        self.point_light_set.render_point_lights_with_shadows(
            self.map,
            self.point_shadow_camera,
            self.point_shadow_object_set_buffer,
            self.point_shadow_model_instructions,
            self.point_light_with_shadow_instructions,
            self.animation_timer_ms,
            #[cfg(feature = "debug")]
            self.render_options,
        );
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_geometry(&mut self) {
        let entities = self.client_state.follow(client_state().entities());
        let dead_entities = self.client_state.follow(client_state().dead_entities());
        let ground_items = self.client_state.follow(client_state().ground_items());

        let offset = self.model_instructions.len();
        let object_set = self.map.cull_objects_with_frustum(
            self.current_camera,
            self.deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            self.render_options.frustum_culling,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_objects))]
        self.map.render_objects(
            self.model_instructions,
            &object_set,
            self.animation_timer_ms,
            self.current_camera,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map))]
        self.map.render_ground(self.model_instructions);

        self.model_batches.push(ModelBatch {
            offset,
            count: self.model_instructions.len() - offset,
            texture_set: self.map.get_texture_set().clone(),
            vertex_buffer: self.map.get_model_vertex_buffer().clone(),
            index_buffer: self.map.get_model_index_buffer().clone(),
        });

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_map_tiles))]
        self.map
            .render_overlay_tiles(self.model_instructions, self.model_batches, self.tile_texture_set);

        #[cfg(feature = "debug")]
        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_pathing))]
        self.map
            .render_entity_pathing(self.model_instructions, self.model_batches, entities, self.pathing_texture_set);

        let entity_camera: &dyn Camera = match true {
            #[cfg(feature = "debug")]
            _ if self.render_options.show_entities_paper => self.player_camera,
            _ => self.current_camera,
        };

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_ground_items))]
        self.map
            .render_ground_items(self.entity_instructions, ground_items, entity_camera, self.client_tick);

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.map.render_entities(
            self.entity_instructions,
            entities,
            entity_camera,
            self.client_tick,
            self.skill_target_highlight,
        );

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_entities))]
        self.map
            .render_dead_entities(self.entity_instructions, dead_entities, entity_camera, self.client_tick);

        #[cfg(feature = "debug")]
        if self.render_options.show_entities_debug {
            self.map.render_entities_debug(self.rectangle_instructions, entities, entity_camera);
            self.map
                .render_entities_debug(self.rectangle_instructions, dead_entities, entity_camera);
        }

        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_water))]
        self.map.render_water(self.water_instruction, self.animation_timer_ms);
    }

    #[inline(always)]
    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    fn render_bounding_boxes(&mut self) {
        if self.render_options.show_bounding_boxes {
            let culling_camera: &dyn Camera = match self.currently_playing {
                true => self.player_camera,
                false => self.start_camera,
            };

            let object_set = self.map.cull_objects_with_frustum(
                culling_camera,
                self.bounding_box_object_set_buffer,
                self.render_options.frustum_culling,
            );

            self.map
                .render_bounding(self.aabb_instructions, self.render_options.frustum_culling, &object_set);
        }
    }

    #[inline(always)]
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    fn render_world_overlays(&mut self) {
        #[cfg(feature = "debug")]
        if let Some(marker_identifier) = self.hovered_marker_identifier {
            self.map.render_marker_overlay(
                self.aabb_instructions,
                self.circle_instructions,
                self.current_camera,
                marker_identifier,
                self.point_light_set,
                self.animation_timer_ms,
            );
        }

        self.particle_holder.render(
            self.bottom_interface_renderer,
            self.current_camera,
            self.screen_size,
            self.scaling,
            self.client_state.follow(client_state().entities()),
        );

        self.effect_holder.render(self.effect_renderer, self.current_camera);

        let world_theme = self.client_state.follow(client_state().world_theme());
        for entity in self.client_state.follow(client_state().entities()).iter() {
            entity.render_cast_bar(
                self.middle_interface_renderer,
                self.current_camera,
                world_theme,
                self.screen_size,
            );
        }

        if let Some(player) = self.client_state.try_follow(this_entity()) {
            #[cfg(feature = "debug")]
            profile_block!("render player status");

            player.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                world_theme,
                self.screen_size,
            );
        }

        if let Some(BufferedAction::AttackEntity { entity_id }) = self.buffered_action
            && let Some(entity) = self
                .client_state
                .follow(client_state().entities())
                .iter()
                .find(|entity| entity.get_entity_id() == entity_id)
        {
            entity.render_status(
                self.middle_interface_renderer,
                self.current_camera,
                world_theme,
                self.screen_size,
            );
        }

        match self.mouse_target {
            PickerTarget::Tile { x, y } => {
                // Only show if the mouse mode is default or walking.
                if self.currently_playing
                    && !self.is_interface_hovered
                    && (self.is_mouse_mode_default || self.last_walking_destination.is_some())
                {
                    let walk_indicator_color = *self.client_state.follow(client_state().world_theme().indicator().walking());

                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(self.render_options.show_indicators))]
                    self.map
                        .render_walk_indicator(self.indicator_instruction, walk_indicator_color, TilePosition { x, y });
                }
            }
            PickerTarget::Entity(entity_id) => {
                if !self.is_interface_hovered && self.is_mouse_mode_default {
                    if let Some(entity) = self
                        .client_state
                        .follow(client_state().entities())
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id)
                    {
                        // Since the buffered attack entity will render its status anyway,
                        // we make sure not to render it here again if it's the same.
                        if !self
                            .buffered_action
                            .is_some_and(|buffered_action| buffered_action.is_attack_entity(entity_id))
                        {
                            entity.render_status(
                                self.middle_interface_renderer,
                                self.current_camera,
                                world_theme,
                                self.screen_size,
                            );
                        }

                        if let Some(name) = &entity.get_details() {
                            let name = name.split('#').next().unwrap();
                            self.middle_interface_renderer
                                .render_hover_text(name, self.scaling, self.mouse_position);
                        }
                    } else if let Some(item) = self
                        .client_state
                        .follow(client_state().ground_items())
                        .iter()
                        .find(|item| item.get_entity_id() == entity_id)
                    {
                        let name = self.library.get::<ItemName>(ItemNameKey {
                            item_id: item.item_id,
                            is_identified: item.is_identified,
                        });

                        // TODO: Don't allocate every frame
                        let text = format!("{name}: {}ea", item.quantity);
                        self.middle_interface_renderer
                            .render_hover_text(&text, self.scaling, self.mouse_position);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod skill_casting_tests {
    use super::*;

    const PLAYER_ID: EntityId = EntityId(1);
    const ENTITY_ID: EntityId = EntityId(2);
    const ITEM_ID: EntityId = EntityId(3);
    const STALE_ID: EntityId = EntityId(4);
    const ENTITY_POSITION: TilePosition = TilePosition { x: 10, y: 20 };
    const ITEM_POSITION: TilePosition = TilePosition { x: 30, y: 40 };

    fn entity_position(entity_id: EntityId) -> Option<TilePosition> {
        match entity_id {
            PLAYER_ID => Some(TilePosition { x: 1, y: 1 }),
            ENTITY_ID => Some(ENTITY_POSITION),
            _ => None,
        }
    }

    fn ground_item_position(entity_id: EntityId) -> Option<TilePosition> {
        (entity_id == ITEM_ID).then_some(ITEM_POSITION)
    }

    fn resolve(skill_type: SkillType, picker_target: PickerTarget) -> Option<SkillCastTarget> {
        resolve_skill_cast_target(
            skill_type,
            picker_target,
            Some(PLAYER_ID),
            entity_position,
            ground_item_position,
        )
    }

    fn update_sound_timing(timing: &mut SkillSoundSequenceTiming, delta_time: f32, ready: bool, playback_count: &mut usize) -> bool {
        let Some(wait_elapsed) = timing.wait_elapsed_if_due(delta_time) else {
            return true;
        };

        if ready {
            *playback_count += 1;
            timing.playback_succeeded()
        } else {
            timing.playback_unavailable(wait_elapsed)
        }
    }

    #[test]
    fn targeted_skills_only_accept_live_entities() {
        assert_eq!(
            resolve(SkillType::Attack, PickerTarget::Entity(ENTITY_ID)),
            Some(SkillCastTarget::Entity(ENTITY_ID))
        );
        assert_eq!(resolve(SkillType::Attack, PickerTarget::Entity(STALE_ID)), None);
        assert_eq!(resolve(SkillType::Attack, PickerTarget::Tile { x: 5, y: 6 }), None);
    }

    #[test]
    fn support_skills_fall_back_to_the_player() {
        assert_eq!(
            resolve(SkillType::Support, PickerTarget::Entity(ENTITY_ID)),
            Some(SkillCastTarget::Entity(ENTITY_ID))
        );
        assert_eq!(
            resolve(SkillType::Support, PickerTarget::Nothing),
            Some(SkillCastTarget::Entity(PLAYER_ID))
        );
        assert_eq!(
            resolve(SkillType::Support, PickerTarget::Tile { x: 5, y: 6 }),
            Some(SkillCastTarget::Entity(PLAYER_ID))
        );
        assert_eq!(resolve(SkillType::Support, PickerTarget::Entity(ITEM_ID)), None);
    }

    #[test]
    fn ground_skills_resolve_tiles_entities_and_ground_items() {
        assert_eq!(
            resolve(SkillType::Ground, PickerTarget::Tile { x: 5, y: 6 }),
            Some(SkillCastTarget::Ground(TilePosition { x: 5, y: 6 }))
        );
        assert_eq!(
            resolve(SkillType::Ground, PickerTarget::Entity(ENTITY_ID)),
            Some(SkillCastTarget::Ground(ENTITY_POSITION))
        );
        assert_eq!(
            resolve(SkillType::Trap, PickerTarget::Entity(ITEM_ID)),
            Some(SkillCastTarget::Ground(ITEM_POSITION))
        );
        assert_eq!(resolve(SkillType::Ground, PickerTarget::Entity(STALE_ID)), None);
    }

    #[test]
    fn passive_and_self_cast_skills_never_resolve_world_targets() {
        assert_eq!(resolve(SkillType::Passive, PickerTarget::Entity(ENTITY_ID)), None);
        assert_eq!(resolve(SkillType::SelfCast, PickerTarget::Entity(ENTITY_ID)), None);
    }

    #[test]
    fn invalid_confirmation_keeps_the_skill_armed() {
        let armed = ArmedSkill {
            skill_id: SkillId(10),
            skill_level: SkillLevel(3),
            skill_type: SkillType::Attack,
        };
        let mut armed_skill = Some(armed);

        assert_eq!(take_resolved_armed_skill(&mut armed_skill, None), None);
        assert_eq!(armed_skill, Some(armed));

        let target = SkillCastTarget::Entity(ENTITY_ID);
        assert_eq!(take_resolved_armed_skill(&mut armed_skill, Some(target)), Some((armed, target)));
        assert_eq!(armed_skill, None);
    }

    #[test]
    fn single_and_double_mouse_buttons_confirm_or_cancel_targeting() {
        assert!(is_skill_target_confirmation(MouseButton::Left));
        assert!(is_skill_target_confirmation(MouseButton::DoubleLeft));
        assert!(is_skill_target_cancellation(MouseButton::Right));
        assert!(is_skill_target_cancellation(MouseButton::DoubleRight));
    }

    #[test]
    fn held_continuous_skill_stops_only_for_its_owner() {
        let skill_id = ROLLING_CUTTER_ID;
        let mut active_skill = None;

        assert_eq!(
            activate_continuous_skill(&mut active_skill, skill_id, SkillActivation::Hold, Some(HotbarSlot(2)),),
            (None, true)
        );
        assert_eq!(release_continuous_skill(&mut active_skill, HotbarSlot(1)), None);
        assert_eq!(release_continuous_skill(&mut active_skill, HotbarSlot(2)), Some(skill_id));
        assert_eq!(active_skill, None);
    }

    #[test]
    fn continuous_skill_ownership_transfers_between_inputs() {
        let skill_id = ROLLING_CUTTER_ID;
        let mut active_skill = None;

        assert_eq!(
            activate_continuous_skill(&mut active_skill, skill_id, SkillActivation::Hold, Some(HotbarSlot(0)),),
            (None, true)
        );
        assert_eq!(
            activate_continuous_skill(&mut active_skill, skill_id, SkillActivation::Hold, Some(HotbarSlot(1)),),
            (Some(skill_id), true)
        );
        assert_eq!(release_continuous_skill(&mut active_skill, HotbarSlot(0)), None);
        assert_eq!(release_continuous_skill(&mut active_skill, HotbarSlot(1)), Some(skill_id));

        assert_eq!(
            activate_continuous_skill(&mut active_skill, skill_id, SkillActivation::Toggle, None),
            (None, true)
        );
        assert_eq!(
            activate_continuous_skill(&mut active_skill, skill_id, SkillActivation::Toggle, None),
            (Some(skill_id), false)
        );
        assert_eq!(active_skill, None);
    }

    #[test]
    fn ready_multi_hit_sound_plays_first_hit_immediately() {
        let mut timing = SkillSoundSequenceTiming::new(0.0, 0.12, 3);
        let mut playback_count = 0;

        assert!(update_sound_timing(&mut timing, 0.0, true, &mut playback_count));
        assert_eq!(playback_count, 1);
        assert_eq!(timing.hits_remaining, 2);
        assert!((timing.next_delay - 0.12).abs() < f32::EPSILON);
    }

    #[test]
    fn unavailable_multi_hit_sound_preserves_hits_and_spacing_after_load() {
        let mut timing = SkillSoundSequenceTiming::new(0.0, 0.12, 3);
        let mut playback_count = 0;

        assert!(update_sound_timing(&mut timing, 0.0, false, &mut playback_count));
        assert_eq!(timing.hits_remaining, 3);

        // The asset becomes ready at t=0.50. Subsequent calls at t=0.61 and
        // t=0.62 demonstrate that the second hit is not emitted in the same
        // frame as the delayed first hit.
        assert!(update_sound_timing(&mut timing, 0.50, true, &mut playback_count));
        assert_eq!(playback_count, 1);
        assert!(update_sound_timing(&mut timing, 0.11, true, &mut playback_count));
        assert_eq!(playback_count, 1);
        assert!(update_sound_timing(&mut timing, 0.01, true, &mut playback_count));
        assert_eq!(playback_count, 2);
        assert!(!update_sound_timing(&mut timing, 0.12, true, &mut playback_count));
        assert_eq!(playback_count, 3);
    }

    #[test]
    fn unavailable_skill_sound_expires_after_one_second() {
        let mut timing = SkillSoundSequenceTiming::new(0.0, 0.12, 3);
        let mut playback_count = 0;

        assert!(update_sound_timing(&mut timing, 0.5, false, &mut playback_count));
        assert!(!update_sound_timing(&mut timing, 0.5, false, &mut playback_count));
        assert_eq!(playback_count, 0);
        assert_eq!(timing.hits_remaining, 3);
    }

    #[test]
    fn large_frame_delta_emits_at_most_one_sequence_hit() {
        let mut timing = SkillSoundSequenceTiming::new(0.0, 0.12, 3);
        let mut playback_count = 0;

        assert!(update_sound_timing(&mut timing, 0.0, true, &mut playback_count));
        assert!(update_sound_timing(&mut timing, 10.0, true, &mut playback_count));
        assert_eq!(playback_count, 2);
        assert_eq!(timing.hits_remaining, 1);
    }
}
