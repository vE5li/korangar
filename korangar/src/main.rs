#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![feature(iter_next_chunk)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(proc_macro_hygiene)]
#![feature(type_changing_struct_update)]
#![feature(unsized_const_params)]
#![feature(variant_count)]

mod graphics;
mod input;
#[macro_use]
mod interface;
mod inventory;
mod loaders;
mod system;
mod world;

use std::cell::RefCell;
use std::io::Cursor;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;
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
    MappedRemote, PlainTrackedState, Remote, RemoteClone, TrackedState, TrackedStateExt, TrackedStateTake, TrackedStateVec,
};
use korangar_interface::Interface;
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkingSystem, SellItem, ShopItem,
};
use korangar_util::collision::Sphere;
use num::Zero;
#[cfg(not(feature = "debug"))]
use ragnarok_packets::handler::NoPacketCallback;
use ragnarok_packets::{
    BuyShopItemsResult, CharacterId, CharacterInformation, CharacterServerInformation, Friend, HotbarSlot, SellItemsResult, SkillId,
    SkillType, TilePosition, UnitId, WorldPosition,
};
use rayon::in_place_scope;
use wgpu::{
    Adapter, CommandEncoderDescriptor, Device, Features, Instance, InstanceFlags, Limits, Maintain, MemoryHints, Queue, TextureFormat,
    TextureViewDescriptor,
};
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
use crate::system::GameTimer;
use crate::world::*;

const CLIENT_NAME: &str = "Korangar";
const ROLLING_CUTTER_ID: SkillId = SkillId(2036);
// The real limiting factor is WGPUs
// "Limit::max_sampled_textures_per_shader_stage".
const MAX_BINDING_TEXTURE_ARRAY_COUNT: usize = 30;
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
    instance: Instance,
    adapter: Adapter,
    device: Arc<Device>,
    queue: Arc<Queue>,
    window: Option<Arc<Window>>,
    surface: Option<Surface>,
    previous_surface_texture_format: Option<TextureFormat>,
    max_frame_count: usize,

    audio_engine: Arc<AudioEngine<GameFileLoader>>,
    #[cfg(feature = "debug")]
    packet_history_callback: PacketHistoryCallback,
    #[cfg(feature = "debug")]
    networking_system: NetworkingSystem<PacketHistoryCallback>,
    #[cfg(not(feature = "debug"))]
    networking_system: NetworkingSystem<NoPacketCallback>,
    render_context: Option<RenderContext>,

    model_loader: ModelLoader,
    texture_loader: TextureLoader,
    font_loader: Rc<RefCell<FontLoader>>,
    map_loader: MapLoader,
    sprite_loader: SpriteLoader,
    script_loader: ScriptLoader,
    action_loader: ActionLoader,
    effect_loader: EffectLoader,

    input_system: InputSystem,
    shadow_detail: MappedRemote<GraphicsSettings, ShadowDetail>,
    framerate_limit: MappedRemote<GraphicsSettings, bool>,
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

    directional_shadow_object_set_buffer: ObjectSetBuffer,
    point_shadow_object_set_buffer: ObjectSetBuffer,
    deferred_object_set_buffer: ObjectSetBuffer,
    #[cfg(feature = "debug")]
    bounding_box_object_set_buffer: ObjectSetBuffer,

    chat_messages: PlainTrackedState<Vec<ChatMessage>>,
    main_menu_click_sound_effect: SoundEffectKey,

    map: Arc<Map>,
}

struct RenderContext {
    deferred_renderer: DeferredRenderer,
    interface_renderer: InterfaceRenderer,
    picker_renderer: PickerRenderer,
    directional_shadow_renderer: DirectionalShadowRenderer,
    point_shadow_renderer: PointShadowRenderer,

    screen_targets: Vec<<DeferredRenderer as Renderer>::Target>,
    interface_target: <InterfaceRenderer as Renderer>::Target,
    picker_targets: Vec<<PickerRenderer as Renderer>::Target>,
    directional_shadow_targets: Vec<<DirectionalShadowRenderer as Renderer>::Target>,
    point_shadow_targets: Vec<Vec<<PointShadowRenderer as Renderer>::Target>>,
    point_shadow_maps: Vec<Vec<Arc<CubeTexture>>>,
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
            let dx12_shader_compiler = wgpu::util::dx12_shader_compiler_from_env().unwrap_or_default();
            let gles_minor_version = wgpu::util::gles_minor_version_from_env().unwrap_or_default();
            let flags = InstanceFlags::from_build_config().with_env();

            let instance = Instance::new(wgpu::InstanceDescriptor {
                backends,
                flags,
                dx12_shader_compiler,
                gles_minor_version,
            });

            let adapter = pollster::block_on(async { wgpu::util::initialize_adapter_from_env_or_default(&instance, None).await.unwrap() });

            #[cfg(feature = "debug")]
            {
                let adapter_info = adapter.get_info();
                print_debug!("using adapter {} ({})", adapter_info.name, adapter_info.backend);
                print_debug!("using device {} ({})", adapter_info.device, adapter_info.vendor);
                print_debug!("using driver {} ({})", adapter_info.driver, adapter_info.driver_info);
            }
        });

        time_phase!("create device", {
            let required_features = Features::PUSH_CONSTANTS | Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING;
            #[cfg(feature = "debug")]
            let required_features = required_features | Features::POLYGON_MODE_LINE;

            let adapter_features = adapter.features();
            assert!(
                adapter_features.contains(required_features),
                "Adapter does not support required features: {:?}",
                required_features - adapter_features
            );
            set_supported_features(adapter_features);

            #[cfg(feature = "debug")]
            {
                let supported = match features_supported(Features::PARTIALLY_BOUND_BINDING_ARRAY) {
                    true => "supported".green(),
                    false => "unsupported".yellow(),
                };
                print_debug!("PARTIALLY_BOUND_BINDING_ARRAY: {}", supported);
            }

            let required_limits = Limits {
                max_push_constant_size: 128,
                max_sampled_textures_per_shader_stage: u32::try_from(MAX_BINDING_TEXTURE_ARRAY_COUNT).unwrap(),
                ..Default::default()
            }
            .using_resolution(adapter.limits());

            let (device, queue) = pollster::block_on(async {
                adapter
                    .request_device(
                        &wgpu::DeviceDescriptor {
                            label: None,
                            required_features: adapter_features | required_features,
                            required_limits,
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

            let mut model_loader = ModelLoader::new(device.clone(), queue.clone(), game_file_loader.clone());
            let mut texture_loader = TextureLoader::new(device.clone(), queue.clone(), game_file_loader.clone());
            let mut map_loader = MapLoader::new(device.clone(), queue.clone(), game_file_loader.clone(), audio_engine.clone());
            let mut sprite_loader = SpriteLoader::new(device.clone(), queue.clone(), game_file_loader.clone());
            let mut action_loader = ActionLoader::new(game_file_loader.clone());
            let effect_loader = EffectLoader::new(game_file_loader.clone());

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
        });

        time_phase!("load settings", {
            // TODO: NHA We should make double buffering optional and selectable in the
            //       settings. Since WGPU uses staging buffers and we record changed
            //       in advance, double buffering is not really needed anymore (especially
            //       with FIFO).
            let max_frame_count = 1;

            let input_system = InputSystem::new();
            let graphics_settings = PlainTrackedState::new(GraphicsSettings::new());

            let shadow_detail = graphics_settings.mapped(|settings| &settings.shadow_detail).new_remote();
            let framerate_limit = graphics_settings.mapped(|settings| &settings.frame_limit).new_remote();

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
            let networking_system = NetworkingSystem::spawn();
            #[cfg(feature = "debug")]
            let packet_history_callback = PacketHistoryCallback::get_static_instance();
            #[cfg(feature = "debug")]
            let networking_system = NetworkingSystem::spawn_with_callback(packet_history_callback.clone());

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

            let directional_shadow_object_set_buffer = ObjectSetBuffer::default();
            let point_shadow_object_set_buffer = ObjectSetBuffer::default();
            let deferred_object_set_buffer = ObjectSetBuffer::default();
            #[cfg(feature = "debug")]
            let bounding_box_object_set_buffer = ObjectSetBuffer::default();

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
                .get(DEFAULT_MAP.to_string(), &mut model_loader, &mut texture_loader)
                .expect("failed to load initial map");

            map.set_ambient_sound_sources(&audio_engine);
            audio_engine.play_background_music_track(DEFAULT_BACKGROUND_MUSIC);
        });

        Self {
            instance,
            adapter,
            device,
            queue,
            window: None,
            surface: None,
            previous_surface_texture_format: None,
            max_frame_count,
            audio_engine,
            #[cfg(feature = "debug")]
            packet_history_callback,
            networking_system,
            render_context: None,
            model_loader,
            texture_loader,
            font_loader,
            map_loader,
            sprite_loader,
            script_loader,
            action_loader,
            effect_loader,
            input_system,
            shadow_detail,
            framerate_limit,
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
            directional_shadow_object_set_buffer,
            point_shadow_object_set_buffer,
            deferred_object_set_buffer,
            #[cfg(feature = "debug")]
            bounding_box_object_set_buffer,
            chat_messages,
            main_menu_click_sound_effect,
            map,
        }
    }

    fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(feature = "debug")]
        let _measurement = threads::Main::start_frame();

        #[cfg(feature = "debug")]
        let timer_measurement = Profiler::start_measurement("update timers");

        self.input_system.update_delta();

        let delta_time = self.game_timer.update();
        let day_timer = self.game_timer.get_day_timer();
        let animation_timer = self.game_timer.get_animation_timer();
        let client_tick = self.game_timer.get_client_tick();

        #[cfg(feature = "debug")]
        timer_measurement.stop();

        let network_events = self.networking_system.get_events();

        let (user_events, hovered_element, focused_element, mouse_target, mouse_position) = self.input_system.user_events(
            &mut self.interface,
            &self.application,
            &mut self.focus_state,
            &mut self.render_context.as_mut().unwrap().picker_targets[self.surface.as_ref().unwrap().frame_number()],
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

        for event in network_events {
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
                        .get(crate::DEFAULT_MAP.to_string(), &mut self.model_loader, &mut self.texture_loader)
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
                        .get(map_name, &mut self.model_loader, &mut self.texture_loader)
                        .unwrap();

                    self.map.set_ambient_sound_sources(&self.audio_engine);
                    self.audio_engine
                        .play_background_music_track(self.map.background_music_track_name());

                    self.saved_player_name = character_information.name.clone();

                    let player = Player::new(
                        &mut self.sprite_loader,
                        &mut self.action_loader,
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
                        &self.script_loader,
                        &self.map,
                        entity_appeared_data,
                        client_tick,
                    );

                    let npc = Entity::Npc(npc);
                    self.entities.push(npc);
                }
                NetworkEvent::RemoveEntity(entity_id) => {
                    self.entities.retain(|entity| entity.get_entity_id() != entity_id);
                }
                NetworkEvent::EntityMove(entity_id, position_from, position_to, starting_timestamp) => {
                    let entity = self.entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        let position_from = Vector2::new(position_from.x, position_from.y);
                        let position_to = Vector2::new(position_to.x, position_to.y);

                        entity.move_from_to(&self.map, position_from, position_to, starting_timestamp);
                        /*#[cfg(feature = "debug")]
                        entity.generate_steps_vertex_buffer(device.clone(), &map);*/
                    }
                }
                NetworkEvent::PlayerMove(position_from, position_to, starting_timestamp) => {
                    let position_from = Vector2::new(position_from.x, position_from.y);
                    let position_to = Vector2::new(position_to.x, position_to.y);
                    self.entities[0].move_from_to(&self.map, position_from, position_to, starting_timestamp);

                    /*#[cfg(feature = "debug")]
                    entities[0].generate_steps_vertex_buffer(device.clone(), &map);*/
                }
                NetworkEvent::ChangeMap(map_name, player_position) => {
                    self.entities.truncate(1);

                    self.map = self
                        .map_loader
                        .get(map_name, &mut self.model_loader, &mut self.texture_loader)
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
                NetworkEvent::DamageEffect(entity_id, damage_amount) => {
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
                    self.particle_holder
                        .add_quest_icon(&mut self.texture_loader, &self.map, quest_effect)
                }
                NetworkEvent::RemoveQuestEffect(entity_id) => self.particle_holder.remove_quest_icon(entity_id),
                NetworkEvent::SetInventory { items } => {
                    self.player_inventory.fill(&mut self.texture_loader, &self.script_loader, items);
                }
                NetworkEvent::IventoryItemAdded { item } => {
                    self.player_inventory.add_item(&mut self.texture_loader, &self.script_loader, item);

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
                    entity.reload_sprite(&mut self.sprite_loader, &mut self.action_loader, &self.script_loader);
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
                    let effect = self.effect_loader.get(path, &mut self.texture_loader).unwrap();
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
                        Color::monochrome_u8(255),
                        50.0,
                        false,
                    )));
                }
                NetworkEvent::AddSkillUnit(entity_id, unit_id, position) => match unit_id {
                    UnitId::Firewall => {
                        let position = Vector2::new(position.x as usize, position.y as usize);
                        let position = self.map.get_world_position(position);
                        let effect = self.effect_loader.get("firewall.str", &mut self.texture_loader).unwrap();
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
                                20.0,
                                true,
                            )),
                            entity_id,
                        );
                    }
                    UnitId::Pneuma => {
                        let position = Vector2::new(position.x as usize, position.y as usize);
                        let position = self.map.get_world_position(position);
                        let effect = self.effect_loader.get("pneuma1.str", &mut self.texture_loader).unwrap();
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
                            .map(|item| self.script_loader.load_market_item_metadata(&mut self.texture_loader, item))
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
                        self.surface.as_ref().unwrap().present_mode_info(),
                        self.shadow_detail.clone_state(),
                        self.framerate_limit.clone_state(),
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

        if let Some(surface) = self.surface.as_mut()
            && surface.is_invalid()
        {
            #[cfg(feature = "debug")]
            profile_block!("re-create buffers");

            surface.reconfigure();
            let dimensions = surface.window_screen_size();

            if let Some(context) = self.render_context.as_mut() {
                context.deferred_renderer.reconfigure_pipeline(
                    surface.format(),
                    dimensions,
                    #[cfg(feature = "debug")]
                    self.render_settings.get().show_wireframe,
                );
                context.interface_renderer.reconfigure_pipeline(dimensions);
                context.picker_renderer.reconfigure_pipeline(dimensions);
                context.screen_targets = (0..self.max_frame_count)
                    .map(|_| context.deferred_renderer.create_render_target())
                    .collect();
                context.interface_target = context.interface_renderer.create_render_target();
                context.picker_targets = (0..self.max_frame_count)
                    .map(|_| context.picker_renderer.create_render_target())
                    .collect();
            }
        }

        if self.shadow_detail.consume_changed() {
            #[cfg(feature = "debug")]
            print_debug!("re-creating {}", "directional shadow targets".magenta());

            #[cfg(feature = "debug")]
            profile_block!("re-create shadow maps");

            let new_shadow_detail = self.shadow_detail.get();

            if let Some(context) = self.render_context.as_mut() {
                context.directional_shadow_targets = (0..self.max_frame_count)
                    .map(|_| {
                        context
                            .directional_shadow_renderer
                            .create_render_target(new_shadow_detail.directional_shadow_resolution())
                    })
                    .collect::<Vec<<DirectionalShadowRenderer as Renderer>::Target>>();

                context.point_shadow_targets = (0..self.max_frame_count)
                    .map(|_| {
                        (0..NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS)
                            .map(|_| {
                                context
                                    .point_shadow_renderer
                                    .create_render_target(new_shadow_detail.point_shadow_resolution())
                            })
                            .collect::<Vec<<PointShadowRenderer as Renderer>::Target>>()
                    })
                    .collect::<Vec<Vec<_>>>();

                context.point_shadow_maps = context
                    .point_shadow_targets
                    .iter()
                    .map(|target| target.iter().map(|target| target.texture.clone()).collect())
                    .collect();
            }
        }

        if self.framerate_limit.consume_changed() {
            self.surface.as_mut().unwrap().set_frame_limit(self.framerate_limit.cloned());

            // For some reason the interface buffer becomes messed up when
            // recreating the surface, so we need to render it again.
            self.interface.schedule_render();
        }

        #[cfg(feature = "debug")]
        let matrices_measurement = Profiler::start_measurement("generate view and projection matrices");

        let window_size = self.surface.as_ref().unwrap().window_size();
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
        let frame_measurement = Profiler::start_measurement("get next frame");

        let (frame_number, frame) = self.surface.as_mut().unwrap().acquire();
        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

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

        let context = self.render_context.as_mut().unwrap();

        let picker_target = &mut context.picker_targets[frame_number];
        let directional_shadow_target = &mut context.directional_shadow_targets[frame_number];
        let point_shadow_target = &mut context.point_shadow_targets[frame_number];
        let deferred_target = &mut context.screen_targets[frame_number];

        // TODO: NHA Lifetime limitation
        let directional_shadow_map = directional_shadow_target.texture.clone();

        #[cfg(feature = "debug")]
        let command_buffer_measurement = Profiler::start_measurement("allocate command buffer");

        let mut picker_render_command_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Picker render"),
        });
        let mut picker_render_pass = picker_target.start_render_pass(&mut picker_render_command_encoder);

        let mut picker_compute_command_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Picker compute"),
        });
        let mut picker_compute_pass = picker_target.start_compute_pass(&mut picker_compute_command_encoder);

        let mut interface_command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("Interface") });
        let mut interface_render_pass = context.interface_target.start(&mut interface_command_encoder, clear_interface);

        let mut directional_shadow_command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("Shadow") });
        let mut directional_shadow_render_pass = directional_shadow_target.start(&mut directional_shadow_command_encoder);

        let mut point_shadow_command_encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Point Light"),
        });

        let mut geometry_command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("Geometry") });
        let mut geometry_render_pass = deferred_target.start_geometry_pass(&mut geometry_command_encoder);

        let mut screen_command_encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("Screen") });
        let mut screen_render_pass = deferred_target.start_screen_pass(&frame_view, &mut screen_command_encoder);

        #[cfg(feature = "debug")]
        command_buffer_measurement.stop();

        #[cfg(feature = "debug")]
        prepare_frame_measurement.stop();

        let point_light_set = {
            self.point_light_manager.prepare();

            self.effect_holder
                .register_point_lights(&mut self.point_light_manager, current_camera);
            self.map.register_point_lights(&mut self.point_light_manager, current_camera);

            self.point_light_manager
                .create_point_light_set(crate::NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS)
        };

        in_place_scope(|scope| {
            scope.spawn(|_| {
                #[cfg(feature = "debug")]
                let _measurement = threads::Picker::start_frame();

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map
                ))]
                self.map
                    .render_tiles(picker_target, &mut picker_render_pass, &context.picker_renderer, current_camera);

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities
                ))]
                self.map.render_entities(
                    entities,
                    picker_target,
                    &mut picker_render_pass,
                    &context.picker_renderer,
                    current_camera,
                    false,
                );

                #[cfg(feature = "debug")]
                self.map.render_markers(
                    picker_target,
                    &mut picker_render_pass,
                    &context.picker_renderer,
                    current_camera,
                    render_settings,
                    entities,
                    &point_light_set,
                    hovered_marker_identifier,
                );

                context
                    .picker_renderer
                    .dispatch_selector(picker_target, &mut picker_compute_pass, screen_size, mouse_position);
            });

            scope.spawn(|_| {
                #[cfg(feature = "debug")]
                let _measurement = threads::Shadow::start_frame();

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map
                ))]
                self.map.render_ground(
                    directional_shadow_target,
                    &mut directional_shadow_render_pass,
                    &context.directional_shadow_renderer,
                    &self.directional_shadow_camera,
                    animation_timer,
                );

                let object_set = self.map.cull_objects_with_frustum(
                    &self.directional_shadow_camera,
                    &mut self.directional_shadow_object_set_buffer,
                    #[cfg(feature = "debug")]
                    render_settings.frustum_culling,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects
                ))]
                self.map.render_objects(
                    directional_shadow_target,
                    &mut directional_shadow_render_pass,
                    &context.directional_shadow_renderer,
                    &self.directional_shadow_camera,
                    client_tick,
                    animation_timer,
                    &object_set,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities
                ))]
                self.map.render_entities(
                    entities,
                    directional_shadow_target,
                    &mut directional_shadow_render_pass,
                    &context.directional_shadow_renderer,
                    &self.directional_shadow_camera,
                    true,
                );

                if let Some(PickerTarget::Tile { x, y }) = mouse_target
                    && !entities.is_empty()
                {
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators
                    ))]
                    self.map.render_walk_indicator(
                        directional_shadow_target,
                        &mut directional_shadow_render_pass,
                        &context.directional_shadow_renderer,
                        &self.directional_shadow_camera,
                        walk_indicator_color,
                        Vector2::new(x as usize, y as usize),
                    );
                }
            });

            scope.spawn(|_| {
                #[cfg(feature = "debug")]
                let _measurement = threads::PointShadow::start_frame();

                for (light_index, point_light) in point_light_set.with_shadow_iterator().enumerate() {
                    let Some(point_shadow_target) = point_shadow_target.get_mut(light_index) else {
                        break;
                    };

                    self.point_shadow_camera.set_camera_position(point_light.position);
                    context.point_shadow_renderer.set_light_position(point_light.position);

                    let extent = point_light_extent(point_light.color, point_light.range);
                    let object_set = self.map.cull_objects_in_sphere(
                        Sphere::new(point_light.position, extent),
                        &mut self.point_shadow_object_set_buffer,
                        #[cfg(feature = "debug")]
                        render_settings.frustum_culling,
                    );
                    // TODO: Create an entity set, similar to the object set for better
                    // performance.

                    for index in 0..6 {
                        self.point_shadow_camera.change_direction(index);
                        self.point_shadow_camera.generate_view_projection(Vector2::zero());

                        let (view_matrix, projection_matrix) = self.point_shadow_camera.view_projection_matrices();
                        let mut point_shadow_render_pass = point_shadow_target.start(
                            &self.queue,
                            &mut point_shadow_command_encoder,
                            index,
                            projection_matrix * view_matrix,
                        );

                        /* #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
                        self.entity_set.render(
                            context.point_shadow_target,
                            &mut point_shadow_render_pass,
                            &context.point_shadow_renderer,
                            &self.point_shadow_camera,
                            true,
                        ); */

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects
                        ))]
                        self.map.render_objects(
                            point_shadow_target,
                            &mut point_shadow_render_pass,
                            &context.point_shadow_renderer,
                            &self.point_shadow_camera,
                            client_tick,
                            animation_timer,
                            &object_set,
                        );

                        if let Some(PickerTarget::Tile { x, y }) = mouse_target
                            && !entities.is_empty()
                        {
                            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators
                            ))]
                            self.map.render_walk_indicator(
                                point_shadow_target,
                                &mut point_shadow_render_pass,
                                &context.point_shadow_renderer,
                                &self.point_shadow_camera,
                                walk_indicator_color,
                                Vector2::new(x as usize, y as usize),
                            );
                        }

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map
                        ))]
                        self.map.render_ground(
                            point_shadow_target,
                            &mut point_shadow_render_pass,
                            &context.point_shadow_renderer,
                            &self.point_shadow_camera,
                            animation_timer,
                        );
                    }
                }
            });

            scope.spawn(|_| {
                #[cfg(feature = "debug")]
                let _measurement = threads::Deferred::start_frame();

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map
                ))]
                self.map.render_ground(deferred_target, &mut geometry_render_pass, &context.deferred_renderer, current_camera, animation_timer);

                #[cfg(feature = "debug")]
                if render_settings.show_map_tiles {
                    self.map.render_overlay_tiles(deferred_target, &mut geometry_render_pass, &context.deferred_renderer, current_camera);
                }

                let object_set = self.map.cull_objects_with_frustum(
                    current_camera,
                    &mut self.deferred_object_set_buffer,
                    #[cfg(feature = "debug")] render_settings.frustum_culling
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects
                ))]
                self.map.render_objects(
                    deferred_target,
                    &mut geometry_render_pass,
                    &context.deferred_renderer,
                    current_camera,
                    client_tick,
                    animation_timer,
                    &object_set,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities
                ))]
                self.map.render_entities(entities, deferred_target, &mut geometry_render_pass, &context.deferred_renderer, current_camera, true);

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_water
                ))]
                self.map.render_water(deferred_target, &mut geometry_render_pass, &context.deferred_renderer, current_camera, animation_timer);

                if let Some(PickerTarget::Tile { x, y }) = mouse_target
                    && !entities.is_empty()
                {
                    #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators
                    ))]
                    self.map.render_walk_indicator(
                        deferred_target,
                        &mut geometry_render_pass,
                        &context.deferred_renderer,
                        current_camera,
                        walk_indicator_color,
                        Vector2::new(x as usize, y as usize),
                    );
                }

                // We switch to record the screen render pass.

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_ambient_light && !render_settings.show_buffers()
                ))]
                self.map.ambient_light(deferred_target, &mut screen_render_pass, &context.deferred_renderer, day_timer);

                let (view_matrix, projection_matrix) = self.directional_shadow_camera.view_projection_matrices();
                let light_matrix = projection_matrix * view_matrix;

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_directional_light && !render_settings.show_buffers()
                ))]
                self.map.directional_light(
                    deferred_target,
                    &mut screen_render_pass,
                    &context.deferred_renderer,
                    current_camera,
                    &directional_shadow_map,
                    light_matrix,
                    day_timer,
                );

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_point_lights && !render_settings.show_buffers()
                ))]
                point_light_set.render_point_lights(deferred_target, &mut screen_render_pass, &context.deferred_renderer, current_camera, &context.point_shadow_maps[frame_number]);

                #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_water && !render_settings.show_buffers()
                ))]
                self.map.water_light(deferred_target, &mut screen_render_pass, &context.deferred_renderer, current_camera);

                #[cfg(feature = "debug")]
                self.map.render_markers(
                    deferred_target,
                    &mut screen_render_pass,
                    &context.deferred_renderer,
                    current_camera,
                    render_settings,
                    entities,
                    &point_light_set,
                    hovered_marker_identifier,
                );

                #[cfg(feature = "debug")]
                if render_settings.show_bounding_boxes {
                    let object_set = self.map.cull_objects_with_frustum(
                        &self.player_camera,
                        &mut self.bounding_box_object_set_buffer,
                        #[cfg(feature = "debug")] render_settings.frustum_culling
                    );

                    self.map.render_bounding(
                        deferred_target,
                        &mut screen_render_pass,
                        &context.deferred_renderer,
                        current_camera,
                        render_settings.frustum_culling,
                        &object_set,
                    );
                }

                #[cfg(feature = "debug")]
                if let Some(marker_identifier) = hovered_marker_identifier {
                    self.map.render_marker_overlay(deferred_target, &mut screen_render_pass, &context.deferred_renderer, current_camera, marker_identifier, &point_light_set);
                }

                self.particle_holder.render(deferred_target, &mut screen_render_pass, &context.deferred_renderer, current_camera, screen_size, self.application.get_scaling_factor(), entities);
                self.effect_holder.render(deferred_target, &mut screen_render_pass, &context.deferred_renderer, current_camera);
            });

            if render_interface {
                #[cfg(feature = "debug")]
                profile_block!("render user interface");

                self.interface.render(
                    &mut context.interface_target,
                    &mut interface_render_pass,
                    &context.interface_renderer,
                    &self.application,
                    hovered_element,
                    focused_element,
                    self.input_system.get_mouse_mode(),
                );
            }
        });

        #[cfg(feature = "debug")]
        if render_settings.show_buffers() {
            // TODO: Make configurable through the UI
            let point_light_id = 0;

            context.deferred_renderer.overlay_buffers(
                deferred_target,
                &mut screen_render_pass,
                &picker_target.texture,
                &directional_shadow_map,
                self.font_loader.borrow().get_font_atlas(),
                &context.point_shadow_maps[frame_number][point_light_id],
                render_settings,
            );
        }

        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
            #[cfg(feature = "debug")]
            profile_block!("render hovered entity status");

            let entity = entities.iter().find(|entity| entity.get_entity_id() == entity_id);

            if let Some(entity) = entity {
                entity.render_status(
                    deferred_target,
                    &mut screen_render_pass,
                    &context.deferred_renderer,
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
                    context.deferred_renderer.render_text(
                        deferred_target,
                        &mut screen_render_pass,
                        name,
                        mouse_position + offset + ScreenPosition::uniform(1.0),
                        Color::monochrome_u8(0),
                        FontSize::new(12.0),
                    );

                    context.deferred_renderer.render_text(
                        deferred_target,
                        &mut screen_render_pass,
                        name,
                        mouse_position + offset,
                        Color::monochrome_u8(255),
                        FontSize::new(12.0),
                    );
                }
            }
        }

        if !entities.is_empty() {
            #[cfg(feature = "debug")]
            profile_block!("render player status");

            entities[0].render_status(
                deferred_target,
                &mut screen_render_pass,
                &context.deferred_renderer,
                current_camera,
                self.application.get_game_theme(),
                screen_size,
            );
        }

        #[cfg(feature = "debug")]
        if render_settings.show_frames_per_second {
            let game_theme = self.application.get_game_theme();

            context.deferred_renderer.render_text(
                deferred_target,
                &mut screen_render_pass,
                &self.game_timer.last_frames_per_second().to_string(),
                game_theme.overlay.text_offset.get().scaled(self.application.get_scaling()),
                game_theme.overlay.foreground_color.get(),
                game_theme.overlay.font_size.get().scaled(self.application.get_scaling()),
            );
        }

        if self.show_interface {
            context
                .deferred_renderer
                .overlay_interface(deferred_target, &mut screen_render_pass, &context.interface_target.texture);

            self.mouse_cursor.render(
                deferred_target,
                &mut screen_render_pass,
                &context.deferred_renderer,
                mouse_position,
                self.input_system.get_mouse_mode().grabbed(),
                self.application.get_game_theme().cursor.color.get(),
                &self.application,
            );
        }

        #[cfg(feature = "debug")]
        let finalize_frame_measurement = Profiler::start_measurement("finishing command encoders");

        drop(picker_render_pass);
        drop(picker_compute_pass);
        drop(interface_render_pass);
        drop(directional_shadow_render_pass);
        drop(geometry_render_pass);
        drop(screen_render_pass);

        let (picker_render_command_buffer, picker_compute_command_buffer) =
            picker_target.finish(picker_render_command_encoder, picker_compute_command_encoder);
        let interface_command_buffer = context.interface_target.finish(interface_command_encoder);
        let directional_shadow_command_buffer = directional_shadow_target.finish(directional_shadow_command_encoder);
        // HACK: `point_shadow_target[0].finish` internally only calls
        // `point_shadow_command_encoder.finish()`. The fact that we use index `0` here
        // is completely arbitrary and a bit ugly.
        let point_shadow_command_buffer = point_shadow_target[0].finish(point_shadow_command_encoder);
        let (deferred_command_buffer, screen_command_buffer) = deferred_target.finish(geometry_command_encoder, screen_command_encoder);

        #[cfg(feature = "debug")]
        finalize_frame_measurement.stop();

        #[cfg(feature = "debug")]
        let queue_measurement = Profiler::start_measurement("queue command buffers");

        // We need to wait for the last submission to finish to be able to resolve all
        // outstanding mapping callback.
        self.device.poll(Maintain::Wait);
        self.queue.submit([
            picker_render_command_buffer,
            picker_compute_command_buffer,
            interface_command_buffer,
            directional_shadow_command_buffer,
            point_shadow_command_buffer,
            deferred_command_buffer,
            screen_command_buffer,
        ]);

        #[cfg(feature = "debug")]
        queue_measurement.stop();

        #[cfg(feature = "debug")]
        let present_measurement = Profiler::start_measurement("present frame");

        frame.present();

        #[cfg(feature = "debug")]
        present_measurement.stop();
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

                let adapter_info = self.adapter.get_info();
                let backend = adapter_info.backend.to_string();
                window.set_title(&format!("{CLIENT_NAME} ({})", str::to_uppercase(&backend)));
                window.set_cursor_visible(false);

                self.window = Some(window);

                #[cfg(feature = "debug")]
                print_debug!("created {}", "window".magenta());
            });
        }

        // Android devices need to drop the surface on suspend, so we might need to
        // re-create it.
        if self.surface.is_none()
            && let Some(window) = self.window.clone()
        {
            time_phase!("create surface", {
                let inner_size: ScreenSize = window.inner_size().max(PhysicalSize::new(1, 1)).into();
                let raw_surface = self.instance.create_surface(window).unwrap();
                let surface = Surface::new(
                    &self.adapter,
                    self.device.clone(),
                    raw_surface,
                    inner_size.width as u32,
                    inner_size.height as u32,
                );
                let surface_texture_format = surface.format();

                if self.previous_surface_texture_format != Some(surface_texture_format) {
                    self.previous_surface_texture_format = Some(surface_texture_format);
                    self.render_context = None;

                    time_phase!("create renderers", {
                        let deferred_renderer = DeferredRenderer::new(
                            self.device.clone(),
                            self.queue.clone(),
                            &mut self.texture_loader,
                            surface_texture_format,
                            inner_size,
                        );
                        let interface_renderer = InterfaceRenderer::new(
                            self.device.clone(),
                            &mut self.texture_loader,
                            self.font_loader.clone(),
                            inner_size,
                        );
                        let picker_renderer = PickerRenderer::new(self.device.clone(), self.queue.clone(), inner_size);
                        let directional_shadow_renderer =
                            DirectionalShadowRenderer::new(self.device.clone(), self.queue.clone(), &mut self.texture_loader);
                        let point_shadow_renderer = PointShadowRenderer::new(self.device.clone(), &mut self.texture_loader);
                    });

                    time_phase!("create render targets", {
                        let screen_targets = (0..self.max_frame_count)
                            .map(|_| deferred_renderer.create_render_target())
                            .collect();
                        let interface_target = interface_renderer.create_render_target();
                        let picker_targets = (0..self.max_frame_count).map(|_| picker_renderer.create_render_target()).collect();
                        let directional_shadow_targets = (0..self.max_frame_count)
                            .map(|_| {
                                directional_shadow_renderer.create_render_target(self.shadow_detail.get().directional_shadow_resolution())
                            })
                            .collect();
                        let point_shadow_targets = (0..self.max_frame_count)
                            .map(|_| {
                                (0..NUMBER_OF_POINT_LIGHTS_WITH_SHADOWS)
                                    .map(|_| point_shadow_renderer.create_render_target(self.shadow_detail.get().point_shadow_resolution()))
                                    .collect::<Vec<<PointShadowRenderer as Renderer>::Target>>()
                            })
                            .collect::<Vec<Vec<_>>>();
                        let point_shadow_maps = point_shadow_targets
                            .iter()
                            .map(|target| target.iter().map(|target| target.texture.clone()).collect())
                            .collect();
                    });

                    self.render_context = Some(RenderContext {
                        deferred_renderer,
                        interface_renderer,
                        picker_renderer,
                        directional_shadow_renderer,
                        point_shadow_renderer,
                        screen_targets,
                        interface_target,
                        picker_targets,
                        directional_shadow_targets,
                        point_shadow_targets,
                        point_shadow_maps,
                    })
                }

                self.surface = Some(surface);

                #[cfg(feature = "debug")]
                print_debug!("created {}", "surface".magenta());
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_) => {
                if let Some(window) = self.window.as_ref() {
                    let screen_size: ScreenSize = window.inner_size().max(PhysicalSize::new(1, 1)).into();
                    self.interface.update_window_size(screen_size);
                    if let Some(surface) = self.surface.as_mut() {
                        surface.update_window_size(screen_size);
                    }
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
                if self.window.is_some() && self.surface.is_some() {
                    self.render_frame(event_loop);
                }
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            _ignored => {}
        }
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // Android devices are expected to drop their surface view.
        if cfg!(target_os = "android") {
            self.surface = None;
        }
    }
}
