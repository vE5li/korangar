#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(allocator_api)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
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
mod system;
mod world;

use std::cell::RefCell;
use std::io::Cursor;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use cgmath::{Vector2, Vector3};
use image::{EncodableLayout, ImageFormat, ImageReader};
use korangar_audio::{AudioEngine, SoundEffectKey};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
#[cfg(feature = "debug")]
use korangar_interface::application::{Application, FontSizeTraitExt, PositionTraitExt};
use korangar_interface::application::{FocusState, FontSizeTrait};
use korangar_interface::state::{
    MappedRemote, PlainTrackedState, Remote, TrackedState, TrackedStateExt, TrackedStateTake, TrackedStateVec,
};
use korangar_interface::Interface;
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkEventBuffer, NetworkingSystem, SellItem,
    ShopItem,
};
#[cfg(feature = "debug")]
use korangar_util::texture_atlas::AtlasAllocation;
#[cfg(not(feature = "debug"))]
use ragnarok_packets::handler::NoPacketCallback;
use ragnarok_packets::{
    BuyShopItemsResult, CharacterId, CharacterInformation, CharacterServerInformation, DisappearanceReason, Friend, HotbarSlot,
    SellItemsResult, SkillId, SkillType, TilePosition, UnitId, WorldPosition,
};
use renderer::InterfaceRenderer;
#[cfg(feature = "debug")]
use wgpu::{Device, Queue};
use wgpu::{Dx12Compiler, Instance, InstanceFlags, MemoryHints};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
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
use crate::renderer::{EffectRenderer, GameInterfaceRenderer};
use crate::system::GameTimer;
use crate::world::*;

const CLIENT_NAME: &str = "Korangar";
const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
const DEFAULT_MAP: &str = "geffen";
const DEFAULT_BACKGROUND_MUSIC: Option<&str> = Some("bgm\\01.mp3");
const MAIN_MENU_CLICK_SOUND_EFFECT: &str = "¹öÆ°¼Ò¸®.wav";
// TODO: The number of point lights that can cast shadows should be configurable
// through the graphics settings. For now I just chose an arbitrary smaller
// number that should be playable on most devices.
const NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS: usize = 6;

static ICON_DATA: &[u8] = include_bytes!("../archive/data/icon.png");

// Create the `threads` module.
#[cfg(feature = "debug")]
korangar_debug::create_profiler_threads!(threads, {
    Main,
    Picker,
    Shadow,
    PointShadow,
    Deferred,
});

fn main() {
    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = threads::Main::start_frame();

    time_phase!("create global thread pool", {
        rayon::ThreadPoolBuilder::new().num_threads(4).build_global().unwrap();
    });

    let mut client = Client::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let _ = event_loop.run_app(&mut client);
}

struct Client {
    window: Option<Arc<Window>>,
    #[cfg(feature = "debug")]
    device: Arc<Device>,
    #[cfg(feature = "debug")]
    queue: Arc<Queue>,
    graphics_engine: GraphicsEngine,
    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    #[cfg(feature = "debug")]
    packet_history_callback: PacketHistoryCallback,
    #[cfg(feature = "debug")]
    networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    networking_system: NetworkingSystem<NoPacketCallback>,

    model_loader: ModelLoader,
    texture_loader: Arc<TextureLoader>,
    font_loader: Rc<RefCell<FontLoader>>,
    map_loader: MapLoader,
    sprite_loader: SpriteLoader,
    script_loader: ScriptLoader,
    action_loader: ActionLoader,
    effect_loader: EffectLoader,
    animation_loader: AnimationLoader,

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
    vsync: MappedRemote<GraphicsSettings, bool>,
    limit_framerate: MappedRemote<GraphicsSettings, LimitFramerate>,
    triple_buffering: MappedRemote<GraphicsSettings, bool>,
    texture_filtering: MappedRemote<GraphicsSettings, TextureSamplerType>,
    shadow_detail: MappedRemote<GraphicsSettings, ShadowDetail>,
    msaa: MappedRemote<GraphicsSettings, Msaa>,
    screen_space_anti_aliasing: MappedRemote<GraphicsSettings, ScreenSpaceAntiAliasing>,
    #[cfg(feature = "debug")]
    render_settings: PlainTrackedState<RenderSettings>,

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

    point_light_set_buffer: ResourceSetBuffer<LightSourceKey>,
    directional_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    point_shadow_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    deferred_object_set_buffer: ResourceSetBuffer<ObjectKey>,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: ResourceSetBuffer<ObjectKey>,

    #[cfg(feature = "debug")]
    pathing_texture_mapping: Vec<AtlasAllocation>,
    #[cfg(feature = "debug")]
    pathing_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    tile_texture: Arc<Texture>,
    #[cfg(feature = "debug")]
    tile_texture_mapping: Vec<AtlasAllocation>,

    chat_messages: PlainTrackedState<Vec<ChatMessage>>,
    main_menu_click_sound_effect: SoundEffectKey,

    map: Map,
}

impl Client {
    fn init() -> Self {
        // We don't know the window size yet, so these values are dummy values.
        let initial_screen_size = ScreenSize {
            width: 800.0,
            height: 600.0,
        };

        time_phase!("create adapter", {
            let trace_dir = std::env::var("WGPU_TRACE");
            let backends = wgpu::util::backend_bits_from_env().unwrap_or_default();
            let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or(Dx12Compiler::Dxc {
                dxil_path: None,
                dxc_path: None,
            });
            let gles_minor_version = wgpu::util::gles_minor_version_from_env().unwrap_or_default();
            let flags = InstanceFlags::from_build_config().with_env();

            let instance = Instance::new(wgpu::InstanceDescriptor {
                backends,
                flags,
                dx12_shader_compiler,
                gles_minor_version,
            });

            let adapter = pollster::block_on(async { wgpu::util::initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });
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
                        &wgpu::DeviceDescriptor {
                            label: None,
                            required_features: capabilities.get_required_features(),
                            required_limits: capabilities.get_required_limits(),
                            memory_hints: MemoryHints::Performance,
                        },
                        trace_dir.ok().as_ref().map(std::path::Path::new),
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

        time_phase!("create audio engine", {
            let audio_engine = Arc::new(AudioEngine::new(game_file_loader.clone()));
        });

        time_phase!("create resource managers", {
            std::fs::create_dir_all("client/themes").unwrap();
            let font_loader = Rc::new(RefCell::new(FontLoader::new(&device, queue.clone(), &game_file_loader)));

            let mut model_loader = ModelLoader::new(game_file_loader.clone());
            let texture_loader = Arc::new(TextureLoader::new(device.clone(), queue.clone(), game_file_loader.clone()));
            let mut map_loader = MapLoader::new(device.clone(), queue.clone(), game_file_loader.clone(), audio_engine.clone());
            let mut sprite_loader = SpriteLoader::new(device.clone(), queue.clone(), game_file_loader.clone());
            let mut action_loader = ActionLoader::new(game_file_loader.clone());
            let effect_loader = EffectLoader::new(game_file_loader.clone());
            let animation_loader = AnimationLoader::new();

            let script_loader = ScriptLoader::new(&game_file_loader).unwrap_or_else(|_| {
                // The scrip loader not being created correctly means that the lua files were
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

                ScriptLoader::new(&game_file_loader).unwrap()
            });

            let interface_renderer = InterfaceRenderer::new(initial_screen_size, font_loader.clone(), &texture_loader);
            let bottom_interface_renderer = GameInterfaceRenderer::new(initial_screen_size, &texture_loader);
            let middle_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let top_interface_renderer = GameInterfaceRenderer::from_renderer(&bottom_interface_renderer);
            let effect_renderer = EffectRenderer::new(initial_screen_size);
            #[cfg(feature = "debug")]
            let debug_marker_renderer = DebugMarkerRenderer::new();

            #[cfg(feature = "debug")]
            let aabb_instructions = Vec::default();
            #[cfg(feature = "debug")]
            let circle_instructions = Vec::default();
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
            let picker_value = Arc::new(AtomicU64::new(0));
            let graphics_engine = GraphicsEngine::initialize(GraphicsEngineDescriptor {
                capabilities,
                adapter,
                instance,
                device: device.clone(),
                queue: queue.clone(),
                texture_loader: texture_loader.clone(),
                picker_value: picker_value.clone(),
            });
        });

        time_phase!("load settings", {
            let input_system = InputSystem::new(picker_value);
            let graphics_settings = PlainTrackedState::new(GraphicsSettings::new());

            let vsync = graphics_settings.mapped(|settings| &settings.vsync).new_remote();
            let limit_framerate = graphics_settings.mapped(|settings| &settings.limit_framerate).new_remote();
            let triple_buffering = graphics_settings.mapped(|settings| &settings.triple_buffering).new_remote();
            let texture_filtering = graphics_settings.mapped(|settings| &settings.texture_filtering).new_remote();
            let shadow_detail = graphics_settings.mapped(|settings| &settings.shadow_detail).new_remote();
            let msaa = graphics_settings.mapped(|settings| &settings.msaa).new_remote();
            let screen_space_anti_aliasing = graphics_settings
                .mapped(|settings| &settings.screen_space_anti_aliasing)
                .new_remote();

            #[cfg(feature = "debug")]
            let render_settings = PlainTrackedState::new(RenderSettings::new());
        });

        time_phase!("initialize interface", {
            let application = InterfaceSettings::load_or_default();
            let mut interface = Interface::new(initial_screen_size);
            let mut focus_state = FocusState::default();
            let mouse_cursor = MouseCursor::new(&mut sprite_loader, &mut action_loader);
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

            start_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
            directional_shadow_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
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

            let point_light_set_buffer = ResourceSetBuffer::default();
            let directional_shadow_object_set_buffer = ResourceSetBuffer::default();
            let point_shadow_object_set_buffer = ResourceSetBuffer::default();
            let deferred_object_set_buffer = ResourceSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ResourceSetBuffer::default();

            #[cfg(feature = "debug")]
            let (pathing_texture_mapping, pathing_texture) =
                TextureAtlasFactory::create_from_group(texture_loader.clone(), "pathing", false, &[
                    "pathing_goal.png",
                    "pathing_straight.png",
                    "pathing_diagonal.png",
                ]);

            #[cfg(feature = "debug")]
            let (tile_texture_mapping, tile_texture) = TextureAtlasFactory::create_from_group(texture_loader.clone(), "tile", false, &[
                "tile_0.png",
                "tile_1.png",
                "tile_2.png",
                "tile_3.png",
                "tile_4.png",
                "tile_5.png",
                "tile_6.png",
            ]);

            let welcome_string = format!(
                "Welcome to ^ffff00★^000000 ^ff8800Korangar^000000 ^ffff00★^000000 version ^ff8800{}^000000!",
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
                .load(
                    DEFAULT_MAP.to_string(),
                    &mut model_loader,
                    texture_loader.clone(),
                    #[cfg(feature = "debug")]
                    &tile_texture_mapping,
                )
                .expect("failed to load initial map");

            map.set_ambient_sound_sources(&audio_engine);
            audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);
        });

        Self {
            window: None,
            #[cfg(feature = "debug")]
            device,
            #[cfg(feature = "debug")]
            queue,
            graphics_engine,
            audio_engine,
            #[cfg(feature = "debug")]
            packet_history_callback,
            networking_system,
            model_loader,
            texture_loader,
            font_loader,
            map_loader,
            sprite_loader,
            script_loader,
            action_loader,
            effect_loader,
            animation_loader,
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
            vsync,
            limit_framerate,
            triple_buffering,
            texture_filtering,
            shadow_detail,
            msaa,
            screen_space_anti_aliasing,
            #[cfg(feature = "debug")]
            render_settings,
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
            point_light_set_buffer,
            directional_shadow_object_set_buffer,
            point_shadow_object_set_buffer,
            deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            bounding_box_object_set_buffer,
            #[cfg(feature = "debug")]
            pathing_texture_mapping,
            #[cfg(feature = "debug")]
            pathing_texture,
            #[cfg(feature = "debug")]
            tile_texture_mapping,
            #[cfg(feature = "debug")]
            tile_texture,
            chat_messages,
            main_menu_click_sound_effect,
            map,
        }
    }

    fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
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

                    self.entities.clear();
                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    self.audio_engine.play_background_music_track(None);

                    self.map = self
                        .map_loader
                        .load(
                            DEFAULT_MAP.to_string(),
                            &mut self.model_loader,
                            self.texture_loader.clone(),
                            #[cfg(feature = "debug")]
                            &self.tile_texture_mapping,
                        )
                        .expect("failed to load initial map");

                    self.map.set_ambient_sound_sources(&self.audio_engine);
                    self.audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);

                    self.interface.close_all_windows_except(&mut self.focus_state);

                    let character_selection_window = CharacterSelectionWindow::new(
                        self.saved_characters.new_remote(),
                        self.move_request.new_remote(),
                        self.saved_slot_count,
                    );
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &character_selection_window);

                    self.start_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
                    self.directional_shadow_camera
                        .set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
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
                NetworkEvent::CharacterSelected { login_data, map_name } => {
                    self.audio_engine.play_sound_effect(self.main_menu_click_sound_effect);

                    let saved_login_data = self.saved_login_data.as_ref().unwrap();
                    self.networking_system.disconnect_from_character_server();
                    self.networking_system.connect_to_map_server(saved_login_data, login_data);

                    let character_information = self
                        .saved_characters
                        .get()
                        .iter()
                        .find(|character| character.character_id == login_data.character_id)
                        .cloned()
                        .unwrap();

                    self.map = self
                        .map_loader
                        .load(
                            map_name,
                            &mut self.model_loader,
                            self.texture_loader.clone(),
                            #[cfg(feature = "debug")]
                            &self.tile_texture_mapping,
                        )
                        .unwrap();

                    self.map.set_ambient_sound_sources(&self.audio_engine);
                    self.audio_engine
                        .play_background_music_track(self.map.background_music_track_name());

                    self.saved_player_name = character_information.name.clone();

                    let player = Player::new(
                        &mut self.sprite_loader,
                        &mut self.action_loader,
                        &mut self.animation_loader,
                        &self.script_loader,
                        &self.map,
                        saved_login_data.account_id,
                        character_information,
                        WorldPosition { x: 0, y: 0 },
                        client_tick,
                    );
                    let player = Entity::Player(player);

                    self.player_camera.set_focus_point(player.get_position());
                    self.entities.push(player);

                    // TODO: this will do one unnecessary restore_focus. check if
                    // that will be problematic
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

                    self.particle_holder.clear();
                    let _ = self.networking_system.map_loaded();
                    // TODO: This is just a workaround until I find a better solution to make the
                    // cursor always look correct.
                    self.mouse_cursor.set_start_time(client_tick);
                    self.game_timer.set_client_tick(client_tick);
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
                NetworkEvent::AddEntity(entity_appeared_data) => {
                    // Sometimes (like after a job change) the server will tell the client
                    // that a new entity appeared, even though it was already on screen. So
                    // to prevent the entity existing twice, we remove the old one.
                    self.entities
                        .retain(|entity| entity.get_entity_id() != entity_appeared_data.entity_id);

                    let npc = Npc::new(
                        &mut self.sprite_loader,
                        &mut self.action_loader,
                        &mut self.animation_loader,
                        &self.script_loader,
                        &self.map,
                        entity_appeared_data,
                        client_tick,
                    );

                    let npc = Entity::Npc(npc);
                    self.entities.push(npc);
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

                    if let Some(entity) = entity {
                        let position_from = Vector2::new(position_from.x, position_from.y);
                        let position_to = Vector2::new(position_to.x, position_to.y);

                        entity.move_from_to(&self.map, position_from, position_to, starting_timestamp);
                        #[cfg(feature = "debug")]
                        entity.generate_pathing_mesh(&self.device, &self.queue, &self.map, &self.pathing_texture_mapping);
                    }
                }
                NetworkEvent::PlayerMove(position_from, position_to, starting_timestamp) => {
                    let position_from = Vector2::new(position_from.x, position_from.y);
                    let position_to = Vector2::new(position_to.x, position_to.y);
                    self.entities[0].move_from_to(&self.map, position_from, position_to, starting_timestamp);

                    #[cfg(feature = "debug")]
                    self.entities[0].generate_pathing_mesh(&self.device, &self.queue, &self.map, &self.pathing_texture_mapping);
                }
                NetworkEvent::ChangeMap(map_name, player_position) => {
                    self.entities.truncate(1);

                    self.map = self
                        .map_loader
                        .load(
                            map_name,
                            &mut self.model_loader,
                            self.texture_loader.clone(),
                            #[cfg(feature = "debug")]
                            &self.tile_texture_mapping,
                        )
                        .unwrap();

                    self.map.set_ambient_sound_sources(&self.audio_engine);
                    self.audio_engine
                        .play_background_music_track(self.map.background_music_track_name());

                    let player_position = Vector2::new(player_position.x as usize, player_position.y as usize);
                    self.entities[0].set_position(&self.map, player_position, client_tick);
                    self.player_camera.set_focus_point(self.entities[0].get_position());

                    self.particle_holder.clear();
                    self.effect_holder.clear();
                    self.point_light_manager.clear();
                    let _ = self.networking_system.map_loaded();

                    // TODO: This is just a workaround until I find a better solution to make the
                    // cursor always look correct.
                    self.mouse_cursor.set_start_time(client_tick);
                }
                NetworkEvent::SetPlayerPosition(player_position) => {
                    let player_position = Vector2::new(player_position.x, player_position.y);
                    self.entities[0].set_position(&self.map, player_position, client_tick);
                    self.player_camera.set_focus_point(self.entities[0].get_position());
                }
                NetworkEvent::UpdateClientTick(client_tick) => {
                    self.game_timer.set_client_tick(client_tick);
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
                    self.particle_holder.add_quest_icon(&self.texture_loader, &self.map, quest_effect)
                }
                NetworkEvent::RemoveQuestEffect(entity_id) => self.particle_holder.remove_quest_icon(entity_id),
                NetworkEvent::SetInventory { items } => {
                    self.player_inventory.fill(&self.texture_loader, &self.script_loader, items);
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.player_inventory.add_item(&self.texture_loader, &self.script_loader, item);

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
                        .fill(&mut self.sprite_loader, &mut self.action_loader, skill_information);
                }
                NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                    self.player_inventory.update_equipped_position(index, equipped_position);
                }
                NetworkEvent::ChangeJob(account_id, job_id) => {
                    let entity = self
                        .entities
                        .iter_mut()
                        .find(|entity| entity.get_entity_id().0 == account_id.0)
                        .unwrap();

                    // FIX: A job change does not automatically send packets for the
                    // inventory and for unequipping items. We should probably manually
                    // request a full list of items and the hotbar.

                    entity.set_job(job_id as usize);
                    entity.reload_sprite(
                        &mut self.sprite_loader,
                        &mut self.action_loader,
                        &mut self.animation_loader,
                        &self.script_loader,
                    );
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
                    let effect = self.effect_loader.get(path, &self.texture_loader).unwrap();
                    let frame_timer = effect.new_frame_timer();

                    self.effect_holder.add_effect(Box::new(EffectWithLight::new(
                        effect,
                        frame_timer,
                        EffectCenter::Entity(entity_id, cgmath::Point3::new(0.0, 0.0, 0.0)),
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
                NetworkEvent::AddSkillUnit(entity_id, unit_id, position) => match unit_id {
                    UnitId::Firewall => {
                        let position = Vector2::new(position.x as usize, position.y as usize);
                        let position = self.map.get_world_position(position);
                        let effect = self.effect_loader.get("firewall.str", &self.texture_loader).unwrap();
                        let frame_timer = effect.new_frame_timer();

                        self.effect_holder.add_unit(
                            Box::new(EffectWithLight::new(
                                effect,
                                frame_timer,
                                EffectCenter::Position(position),
                                Vector3::new(0.0, 0.0, 0.0),
                                PointLightId::new(unit_id as u32),
                                Vector3::new(0.0, 3.0, 0.0),
                                Color::rgb_u8(255, 30, 0),
                                60.0,
                                true,
                            )),
                            entity_id,
                        );
                    }
                    UnitId::Pneuma => {
                        let position = Vector2::new(position.x as usize, position.y as usize);
                        let position = self.map.get_world_position(position);
                        let effect = self.effect_loader.get("pneuma1.str", &self.texture_loader).unwrap();
                        let frame_timer = effect.new_frame_timer();

                        self.effect_holder.add_unit(
                            Box::new(EffectWithLight::new(
                                effect,
                                frame_timer,
                                EffectCenter::Position(position),
                                Vector3::new(0.0, 0.0, 0.0),
                                PointLightId::new(unit_id as u32),
                                Vector3::new(0.0, 3.0, 0.0),
                                Color::rgb_u8(83, 220, 108),
                                40.0,
                                false,
                            )),
                            entity_id,
                        );
                    }
                    _ => {}
                },
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
                            .map(|item| self.script_loader.load_market_item_metadata(&self.texture_loader, item))
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
                        self.vsync.clone_state(),
                        self.limit_framerate.clone_state(),
                        self.triple_buffering.clone_state(),
                        self.texture_filtering.clone_state(),
                        self.msaa.clone_state(),
                        self.screen_space_anti_aliasing.clone_state(),
                        self.shadow_detail.clone_state(),
                    ),
                ),
                UserEvent::OpenAudioSettingsWindow => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, &AudioSettingsWindow)
                }
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
                UserEvent::OpenMarkerDetails(marker_identifier) => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    self.map.resolve_marker(&self.entities, marker_identifier),
                ),
                #[cfg(feature = "debug")]
                UserEvent::OpenRenderSettingsWindow => self.interface.open_window(
                    &self.application,
                    &mut self.focus_state,
                    &RenderSettingsWindow::new(self.render_settings.clone()),
                ),
                #[cfg(feature = "debug")]
                UserEvent::OpenMapDataWindow => {
                    self.interface
                        .open_window(&self.application, &mut self.focus_state, self.map.to_prototype_window())
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
                UserEvent::SetDawn => self.game_timer.set_day_timer(0.0),
                #[cfg(feature = "debug")]
                UserEvent::SetNoon => self.game_timer.set_day_timer(std::f32::consts::FRAC_PI_2),
                #[cfg(feature = "debug")]
                UserEvent::SetDusk => self.game_timer.set_day_timer(std::f32::consts::PI),
                #[cfg(feature = "debug")]
                UserEvent::SetMidnight => self.game_timer.set_day_timer(-std::f32::consts::FRAC_PI_2),
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
        let update_entities_measurement = Profiler::start_measurement("update entities");

        self.entities
            .iter_mut()
            .for_each(|entity| entity.update(&self.map, delta_time as f32, client_tick));

        #[cfg(feature = "debug")]
        update_entities_measurement.stop();

        if !self.entities.is_empty() {
            let player_position = self.entities[0].get_position();
            self.player_camera.set_smoothed_focus_point(player_position);
            self.directional_shadow_camera.set_focus_point(self.player_camera.focus_point());
        }

        #[cfg(feature = "debug")]
        let update_cameras_measurement = Profiler::start_measurement("update cameras");

        self.start_camera.update(delta_time);
        self.player_camera.update(delta_time);
        self.directional_shadow_camera.update(day_timer);

        #[cfg(feature = "debug")]
        update_cameras_measurement.stop();

        self.particle_holder.update(delta_time as f32);
        self.effect_holder.update(&self.entities, delta_time as f32);

        let (clear_interface, render_interface) = self
            .interface
            .update(&self.application, self.font_loader.clone(), &mut self.focus_state);
        self.mouse_cursor.update(client_tick);

        #[cfg(feature = "debug")]
        let matrices_measurement = Profiler::start_measurement("generate view and projection matrices");

        let window_size = self.graphics_engine.get_window_size();
        let screen_size: ScreenSize = window_size.into();

        if self.entities.is_empty() {
            self.start_camera.generate_view_projection(window_size);
        }

        self.player_camera.generate_view_projection(window_size);
        self.directional_shadow_camera.generate_view_projection(window_size);
        #[cfg(feature = "debug")]
        if self.render_settings.get().use_debug_camera {
            self.debug_camera.generate_view_projection(window_size);
        }

        #[cfg(feature = "debug")]
        matrices_measurement.stop();

        let current_camera: &(dyn Camera + Send + Sync) = match self.entities.is_empty() {
            #[cfg(feature = "debug")]
            _ if self.render_settings.get().use_debug_camera => &self.debug_camera,
            true => &self.start_camera,
            false => &self.player_camera,
        };

        #[cfg(feature = "debug")]
        let frame_measurement = Profiler::start_measurement("update audio engine");

        // We set the listener roughly at ear height.
        const EAR_HEIGHT: Vector3<f32> = Vector3::new(0.0, 5.0, 0.0);
        let listener = current_camera.focus_point() + EAR_HEIGHT;

        self.audio_engine
            .set_ambient_listener(listener, current_camera.view_direction(), current_camera.look_up_vector());
        self.audio_engine.update();

        #[cfg(feature = "debug")]
        frame_measurement.stop();

        #[cfg(feature = "debug")]
        let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

        #[cfg(feature = "debug")]
        let render_settings = &*self.render_settings.get();
        let walk_indicator_color = self.application.get_game_theme().indicator.walking.get();
        let entities = &self.entities[..];
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
            self.map
                .register_point_lights(&mut self.point_light_manager, &mut self.point_light_set_buffer, current_camera);

            self.point_light_manager.create_point_light_set(NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS)
        };

        #[cfg(feature = "debug")]
        point_light_manager_measurement.stop();

        #[cfg(feature = "debug")]
        prepare_frame_measurement.stop();

        #[cfg(feature = "debug")]
        let collect_instructions_measurement = Profiler::start_measurement("collect instructions");

        let (view_matrix, projection_matrix) = current_camera.view_projection_matrices();
        let water_level = self.map.get_water_light();
        let ambient_light_color = self.map.get_ambient_light_color(day_timer);
        let (directional_light_view_matrix, directional_light_projection_matrix) =
            self.directional_shadow_camera.view_projection_matrices();
        let directional_light_matrix = directional_light_projection_matrix * directional_light_view_matrix;
        let (directional_light_direction, directional_light_color) = self.map.get_directional_light(day_timer);
        let picker_position = ScreenPosition {
            left: mouse_position.left.clamp(0.0, window_size.x as f32),
            top: mouse_position.top.clamp(0.0, window_size.y as f32),
        };
        let mut indicator_instruction = None;
        let mut map_water_vertex_buffer = None;

        // Marker
        {
            #[cfg(feature = "debug")]
            self.map.render_markers(
                &mut self.debug_marker_renderer,
                current_camera,
                render_settings,
                entities,
                &point_light_set,
                hovered_marker_identifier,
            );

            #[cfg(feature = "debug")]
            self.map.render_markers(
                &mut self.middle_interface_renderer,
                current_camera,
                render_settings,
                entities,
                &point_light_set,
                hovered_marker_identifier,
            );
        }

        // Directional Shadows
        {
            let object_set = self.map.cull_objects_with_frustum(
                &self.directional_shadow_camera,
                &mut self.directional_shadow_object_set_buffer,
                #[cfg(feature = "debug")]
                render_settings.frustum_culling,
            );

            let offset = self.directional_shadow_model_instructions.len();

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
            self.map
                .render_objects(&mut self.directional_shadow_model_instructions, &object_set, client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
            self.map.render_ground(&mut self.directional_shadow_model_instructions);

            let count = self.directional_shadow_model_instructions.len() - offset;

            self.directional_shadow_model_batches.push(ModelBatch {
                offset,
                count,
                texture: self.map.get_texture().clone(),
                vertex_buffer: self.map.get_model_vertex_buffer().clone(),
            });

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map_tiles))]
            self.map.render_overlay_tiles(
                &mut self.directional_shadow_model_instructions,
                &mut self.directional_shadow_model_batches,
                &self.tile_texture,
            );

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_pathing))]
            self.map.render_entity_pathing(
                &mut self.directional_shadow_model_instructions,
                &mut self.directional_shadow_model_batches,
                &self.entities,
                &self.pathing_texture,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
            self.map.render_entities(
                &mut self.directional_shadow_entity_instructions,
                entities,
                &self.directional_shadow_camera,
                true,
            );
        }

        // Point Lights and Shadows
        {
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_point_lights))]
            point_light_set.render_point_lights(&mut self.point_light_instructions);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_point_lights))]
            point_light_set.render_point_lights_with_shadows(
                &self.map,
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
            let object_set = self.map.cull_objects_with_frustum(
                current_camera,
                &mut self.deferred_object_set_buffer,
                #[cfg(feature = "debug")]
                render_settings.frustum_culling,
            );

            let offset = self.model_instructions.len();

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
            self.map.render_objects(&mut self.model_instructions, &object_set, client_tick);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
            self.map.render_ground(&mut self.model_instructions);

            let count = self.model_instructions.len() - offset;

            self.model_batches.push(ModelBatch {
                offset,
                count,
                texture: self.map.get_texture().clone(),
                vertex_buffer: self.map.get_model_vertex_buffer().clone(),
            });

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map_tiles))]
            self.map
                .render_overlay_tiles(&mut self.model_instructions, &mut self.model_batches, &self.tile_texture);

            #[cfg(feature = "debug")]
            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_pathing))]
            self.map.render_entity_pathing(
                &mut self.model_instructions,
                &mut self.model_batches,
                &self.entities,
                &self.pathing_texture,
            );

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
            self.map
                .render_entities(&mut self.entity_instructions, entities, current_camera, true);

            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_water))]
            self.map.render_water(&mut map_water_vertex_buffer);

            #[cfg(feature = "debug")]
            if render_settings.show_bounding_boxes {
                let object_set = self.map.cull_objects_with_frustum(
                    &self.player_camera,
                    &mut self.bounding_box_object_set_buffer,
                    #[cfg(feature = "debug")]
                    render_settings.frustum_culling,
                );

                self.map
                    .render_bounding(&mut self.aabb_instructions, render_settings.frustum_culling, &object_set);
            }
        }

        //  Sprites and Interface
        {
            #[cfg(feature = "debug")]
            if let Some(marker_identifier) = hovered_marker_identifier {
                self.map.render_marker_overlay(
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
                self.application.get_scaling_factor(),
                entities,
            );

            self.effect_holder.render(&mut self.effect_renderer, current_camera);

            if let Some(PickerTarget::Tile { x, y }) = mouse_target
                && !entities.is_empty()
            {
                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators))]
                self.map.render_walk_indicator(
                    &mut indicator_instruction,
                    walk_indicator_color,
                    Vector2::new(x as usize, y as usize),
                );
            } else if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                let entity = entities.iter().find(|entity| entity.get_entity_id() == entity_id);

                if let Some(entity) = entity {
                    entity.render_status(
                        &self.middle_interface_renderer,
                        current_camera,
                        self.application.get_game_theme(),
                        screen_size,
                    );

                    if let Some(name) = &entity.get_details() {
                        let name = name.split('#').next().unwrap();

                        let offset = ScreenPosition {
                            left: name.len() as f32 * -3.0,
                            top: 20.0,
                        };

                        // TODO: move variables into theme
                        self.middle_interface_renderer.render_text(
                            name,
                            mouse_position + offset + ScreenPosition::uniform(1.0),
                            Color::BLACK,
                            FontSize::new(12.0),
                        );

                        self.middle_interface_renderer
                            .render_text(name, mouse_position + offset, Color::WHITE, FontSize::new(12.0));
                    }
                }
            }

            if !entities.is_empty() {
                #[cfg(feature = "debug")]
                profile_block!("render player status");

                entities[0].render_status(
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
                    game_theme.overlay.text_offset.get().scaled(self.application.get_scaling()),
                    game_theme.overlay.foreground_color.get(),
                    game_theme.overlay.font_size.get().scaled(self.application.get_scaling()),
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
        let font_atlas = self.font_loader.borrow();

        let render_instruction = RenderInstruction {
            clear_interface,
            show_interface: self.show_interface,
            picker_position,
            uniforms: Uniforms {
                view_matrix,
                projection_matrix,
                animation_timer,
                day_timer,
                water_level,
                ambient_light_color,
            },
            indicator: indicator_instruction,
            interface: interface_instructions.as_slice(),
            bottom_layer_rectangles: bottom_layer_instructions.as_slice(),
            middle_layer_rectangles: middle_layer_instructions.as_slice(),
            top_layer_rectangles: top_layer_instructions.as_slice(),
            directional_light_with_shadow: DirectionalShadowCasterInstruction {
                view_projection_matrix: directional_light_matrix,
                direction: directional_light_direction,
                color: directional_light_color,
            },
            point_light_shadow_caster: &self.point_light_with_shadow_instructions,
            point_light: &self.point_light_instructions,
            model_batches: &self.model_batches,
            models: &self.model_instructions,
            entities: &self.entity_instructions,
            directional_model_batches: &self.directional_shadow_model_batches,
            directional_shadow_models: &self.directional_shadow_model_instructions,
            directional_shadow_entities: &self.directional_shadow_entity_instructions,
            point_shadow_models: &self.point_shadow_model_instructions,
            point_shadow_entities: &self.point_shadow_entity_instructions,
            effects: self.effect_renderer.get_instructions(),
            map_picker_tile_vertex_buffer: self.map.get_tile_picker_vertex_buffer(),
            map_water_vertex_buffer,
            font_atlas_texture: font_atlas.get_font_atlas(),
            #[cfg(feature = "debug")]
            render_settings: *self.render_settings.get(),
            #[cfg(feature = "debug")]
            aabb: &self.aabb_instructions,
            #[cfg(feature = "debug")]
            circles: &self.circle_instructions,
            #[cfg(feature = "debug")]
            marker: self.debug_marker_renderer.get_instructions(),
        };

        self.graphics_engine.render_next_frame(frame, &render_instruction);

        #[cfg(feature = "debug")]
        render_frame_measurement.stop();
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

        if self.screen_space_anti_aliasing.consume_changed() {
            self.graphics_engine
                .set_screen_space_anti_aliasing(*self.screen_space_anti_aliasing.get());
        }

        if self.shadow_detail.consume_changed() {
            self.graphics_engine.set_shadow_detail(*self.shadow_detail.get());
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

                let window_attributes = Window::default_attributes().with_title(CLIENT_NAME).with_window_icon(Some(icon));
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
        if let Some(window) = self.window.clone() {
            self.graphics_engine.on_resume(
                window,
                *self.triple_buffering.get(),
                *self.vsync.get(),
                *self.limit_framerate.get(),
                *self.shadow_detail.get(),
                *self.texture_filtering.get(),
                *self.msaa.get(),
                *self.screen_space_anti_aliasing.get(),
            )
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
    }
}
