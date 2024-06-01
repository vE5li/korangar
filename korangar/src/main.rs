#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(auto_traits)]
#![feature(lazy_cell)]
#![feature(const_trait_impl)]
#![feature(decl_macro)]
#![feature(div_duration)]
#![feature(effects)]
#![feature(generic_const_exprs)]
#![feature(iter_intersperse)]
#![feature(iter_next_chunk)]
#![feature(let_chains)]
#![feature(negative_impls)]
#![feature(option_zip)]
#![feature(proc_macro_hygiene)]
#![feature(type_changing_struct_update)]
#![feature(variant_count)]

mod input;
#[macro_use]
mod system;
mod graphics;
mod interface;
mod inventory;
mod loaders;
mod world;

use std::cell::RefCell;
use std::io::Cursor;
use std::net::ToSocketAddrs;
use std::rc::Rc;
use std::sync::Arc;

use cgmath::{Vector2, Vector3};
use image::io::Reader as ImageReader;
use image::{EncodableLayout, ImageFormat};
#[cfg(feature = "debug")]
use korangar_debug::logging::{print_debug, Colorize, Timer};
#[cfg(feature = "debug")]
use korangar_debug::profile_block;
#[cfg(feature = "debug")]
use korangar_debug::profiling::Profiler;
use korangar_interface::application::{Application, FocusState, FontSizeTrait, FontSizeTraitExt, PositionTraitExt};
use korangar_interface::state::{PlainTrackedState, Remote, RemoteClone, TrackedState, TrackedStateExt, TrackedStateTake, TrackedStateVec};
use korangar_interface::Interface;
use korangar_networking::{
    DisconnectReason, HotkeyState, LoginServerLoginData, MessageColor, NetworkEvent, NetworkingSystem, SellItem, ShopItem,
};
use ragnarok_packets::{
    BuyShopItemsResult, CharacterId, CharacterInformation, CharacterServerInformation, Friend, HotbarSlot, SellItemsResult, SkillId,
    SkillType, TilePosition, UnitId, WorldPosition,
};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
#[cfg(feature = "debug")]
use vulkano::instance::debug::{
    DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCallback, DebugUtilsMessengerCreateInfo,
};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo, InstanceExtensions};
use vulkano::swapchain::Surface;
use vulkano::sync::{now, GpuFuture};
use vulkano::VulkanLibrary;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Icon, WindowBuilder};

use crate::graphics::*;
use crate::input::{InputSystem, UserEvent};
use crate::interface::application::InterfaceSettings;
use crate::interface::cursor::{MouseCursor, MouseCursorState};
use crate::interface::dialog::DialogSystem;
use crate::interface::layout::{ScreenPosition, ScreenSize};
use crate::interface::linked::LinkedElement;
use crate::interface::resource::{ItemSource, Move, SkillSource};
use crate::interface::windows::*;
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::*;
#[cfg(feature = "debug")]
use crate::system::vulkan_message_callback;
use crate::system::{choose_physical_device, get_device_extensions, get_layers, GameTimer};
use crate::world::*;

const ROLLING_CUTTER_ID: SkillId = SkillId(2036);

// Create the `threads` module.
#[cfg(feature = "debug")]
korangar_debug::create_profiler_threads!(threads, {
    Main,
    Picker,
    Shadow,
    Deferred,
});

fn main() {
    const DEFAULT_MAP: &str = "geffen";

    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = threads::Main::start_frame();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let library = VulkanLibrary::new().unwrap();
    let event_loop = EventLoop::new();
    let create_info = InstanceCreateInfo {
        enabled_extensions: InstanceExtensions {
            ext_debug_utils: true,
            ..Surface::required_extensions(&event_loop)
        },
        enabled_layers: get_layers(&library),
        flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
        ..Default::default()
    };

    let instance = Instance::new(library, create_info).expect("failed to create instance");

    #[cfg(feature = "debug")]
    let _debug_callback = DebugUtilsMessenger::new(instance.clone(), DebugUtilsMessengerCreateInfo {
        message_severity: DebugUtilsMessageSeverity::ERROR
            | DebugUtilsMessageSeverity::WARNING
            | DebugUtilsMessageSeverity::INFO
            | DebugUtilsMessageSeverity::VERBOSE,
        message_type: DebugUtilsMessageType::GENERAL | DebugUtilsMessageType::VALIDATION | DebugUtilsMessageType::PERFORMANCE,
        ..DebugUtilsMessengerCreateInfo::user_callback(unsafe { DebugUtilsMessengerCallback::new(vulkan_message_callback) })
    })
    .ok();

    #[cfg(feature = "debug")]
    print_debug!("created {}", "instance".magenta());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create window");

    // TODO: move this somewhere else
    let file_data = include_bytes!("../archive/data/icon.png");

    let reader = ImageReader::with_format(Cursor::new(file_data), ImageFormat::Png);
    let image_buffer = reader.decode().unwrap().to_rgba8();
    let image_data = image_buffer.as_bytes().to_vec();

    assert_eq!(image_buffer.width(), image_buffer.height(), "icon must be square");
    let icon = Icon::from_rgba(image_data, image_buffer.width(), image_buffer.height()).unwrap();
    //

    let window = WindowBuilder::new()
        .with_title("Korangar".to_string())
        .with_window_icon(Some(icon))
        .build(&event_loop)
        .unwrap();
    window.set_cursor_visible(false);
    let window = Arc::new(window);

    let surface = Surface::from_window(instance.clone(), window).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}", "window".magenta());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("choose physical device");

    let desired_device_extensions = get_device_extensions();
    let (physical_device, queue_family_index) = choose_physical_device(&instance, &surface, &desired_device_extensions);

    let present_mode_info = PresentModeInfo::from_device(&physical_device, &surface);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let (device, mut queues) = Device::new(physical_device.clone(), DeviceCreateInfo {
        enabled_extensions: get_device_extensions(),
        enabled_features: vulkano::device::Features {
            sampler_anisotropy: true,
            #[cfg(feature = "debug")]
            fill_mode_non_solid: true,
            ..Default::default()
        },
        queue_create_infos: vec![QueueCreateInfo {
            queue_family_index,
            ..Default::default()
        }],
        ..Default::default()
    })
    .expect("failed to create device");

    #[cfg(feature = "debug")]
    print_debug!("created {}", "vulkan device".magenta());

    let queue = queues.next().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("received {} from {}", "queue".magenta(), "device".magenta());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resource managers");

    std::fs::create_dir_all("client/themes").unwrap();

    let mut game_file_loader = GameFileLoader::default();

    game_file_loader.load_archives_from_settings();
    game_file_loader.load_patched_lua_files();

    let memory_allocator = Arc::new(MemoryAllocator::new(device.clone()));

    let font_loader = Rc::new(RefCell::new(FontLoader::new(
        memory_allocator.clone(),
        queue.clone(),
        &mut game_file_loader,
    )));

    let mut buffer_allocator = BufferAllocator::new(memory_allocator.clone(), queue.clone());
    let mut model_loader = ModelLoader::new();
    let mut texture_loader = TextureLoader::new(memory_allocator.clone(), queue.clone());
    let mut map_loader = MapLoader::new();
    let mut sprite_loader = SpriteLoader::new(memory_allocator.clone(), queue.clone());
    let mut action_loader = ActionLoader::default();
    let mut effect_loader = EffectLoader::default();
    let script_loader = ScriptLoader::new(&mut game_file_loader);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    let mut map = map_loader
        .get(
            DEFAULT_MAP.to_string(),
            &mut game_file_loader,
            &mut buffer_allocator,
            &mut model_loader,
            &mut texture_loader,
        )
        .expect("failed to load initial map");

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create swapchain");

    let mut swapchain_holder = SwapchainHolder::new(&physical_device, device.clone(), queue.clone(), surface.clone());
    let viewport = swapchain_holder.viewport();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create renderers");

    let mut deferred_renderer = DeferredRenderer::new(
        memory_allocator.clone(),
        &mut buffer_allocator,
        &mut game_file_loader,
        &mut texture_loader,
        queue.clone(),
        swapchain_holder.swapchain_format(),
        viewport.clone(),
        swapchain_holder.window_size_u32(),
    );

    let mut interface_renderer = InterfaceRenderer::new(
        memory_allocator.clone(),
        &mut game_file_loader,
        &mut texture_loader,
        font_loader.clone(),
        queue.clone(),
        viewport.clone(),
        swapchain_holder.window_size_u32(),
    );

    let mut picker_renderer = PickerRenderer::new(
        memory_allocator.clone(),
        queue.clone(),
        viewport,
        swapchain_holder.window_size_u32(),
    );

    let shadow_renderer = ShadowRenderer::new(memory_allocator, &mut game_file_loader, &mut texture_loader, queue);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load settings");

    let mut input_system = InputSystem::new();
    let graphics_settings = PlainTrackedState::new(GraphicsSettings::new());

    let mut shadow_detail = graphics_settings.mapped(|settings| &settings.shadow_detail).new_remote();
    let mut framerate_limit = graphics_settings.mapped(|settings| &settings.frame_limit).new_remote();

    #[cfg(feature = "debug")]
    let render_settings = PlainTrackedState::new(RenderSettings::new());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create render targets");

    let mut screen_targets = swapchain_holder
        .get_swapchain_images()
        .into_iter()
        .map(|swapchain_image| deferred_renderer.create_render_target(swapchain_image))
        .collect::<Vec<<DeferredRenderer as Renderer>::Target>>();

    let mut interface_target = interface_renderer.create_render_target();

    let mut picker_targets = swapchain_holder
        .get_swapchain_images()
        .into_iter()
        .map(|_| picker_renderer.create_render_target())
        .collect::<Vec<<PickerRenderer as Renderer>::Target>>();

    let mut directional_shadow_targets = swapchain_holder
        .get_swapchain_images()
        .into_iter()
        .map(|_| shadow_renderer.create_render_target(shadow_detail.get().into_resolution()))
        .collect::<Vec<<ShadowRenderer as Renderer>::Target>>();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize interface");

    let mut application = InterfaceSettings::load_or_default();
    let mut interface = Interface::new(swapchain_holder.window_screen_size());
    let mut focus_state = FocusState::default();
    let mut mouse_cursor = MouseCursor::new(&mut game_file_loader, &mut sprite_loader, &mut action_loader);
    let mut dialog_system = DialogSystem::default();
    let mut show_interface = true;

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize timer");

    let mut game_timer = GameTimer::new();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize camera");

    #[cfg(feature = "debug")]
    let mut debug_camera = DebugCamera::new();
    let mut start_camera = StartCamera::new();
    let mut player_camera = PlayerCamera::new();
    let mut directional_shadow_camera = ShadowCamera::new();

    start_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
    directional_shadow_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize networking");

    let client_info = load_client_info(&mut game_file_loader);

    #[cfg(not(feature = "debug"))]
    let mut networking_system = NetworkingSystem::spawn();
    #[cfg(feature = "debug")]
    let packet_callback = {
        // SAFETY: This function leaks memory, but it's fine since we only call
        // it once.
        unsafe { interface::elements::PacketHistoryCallback::new() }
    };
    #[cfg(feature = "debug")]
    let mut networking_system = NetworkingSystem::spawn_with_callback(packet_callback.clone());

    let mut friend_list: PlainTrackedState<Vec<(Friend, LinkedElement)>> = PlainTrackedState::default();
    let mut saved_login_data: Option<LoginServerLoginData> = None;
    let mut saved_character_server: Option<CharacterServerInformation> = None;
    let mut saved_characters: PlainTrackedState<Vec<CharacterInformation>> = PlainTrackedState::default();
    let mut shop_items: PlainTrackedState<Vec<ShopItem<ResourceMetadata>>> = PlainTrackedState::default();
    let mut sell_items: PlainTrackedState<Vec<SellItem<(ResourceMetadata, u16)>>> = PlainTrackedState::default();
    let mut currently_deleting: Option<CharacterId> = None;
    let mut saved_player_name = String::new();
    let mut move_request: PlainTrackedState<Option<usize>> = PlainTrackedState::default();
    let mut saved_login_server_address = None;
    let mut saved_password = String::new();
    let mut saved_username = String::new();
    let mut saved_slot_count = 0;

    interface.open_window(&application, &mut focus_state, &LoginWindow::new(&client_info));

    #[cfg(feature = "debug")]
    timer.stop();

    let mut particle_holder = ParticleHolder::default();
    let mut effect_holder = EffectHolder::default();
    let mut entities = Vec::<Entity>::new();
    let mut player_inventory = Inventory::default();
    let mut player_skill_tree = SkillTree::default();
    let mut hotbar = Hotbar::default();

    let welcome_string = format!(
        "Welcome to ^ffff00★^000000 ^ff8800Korangar^000000 ^ffff00★^000000 version ^ff8800{}^000000!",
        env!("CARGO_PKG_VERSION")
    );
    let mut chat_messages = PlainTrackedState::new(vec![ChatMessage {
        text: welcome_string,
        color: MessageColor::Server,
    }]);

    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(3).build().unwrap();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                // FIX: For some reason GraphicsSettings is not dropped unless we use it in this
                // scope. This fixes it.
                let _ = &graphics_settings;
                control_flow.set_exit()
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                let window_size = surface
                    .object()
                    .unwrap()
                    .downcast_ref::<winit::window::Window>()
                    .unwrap()
                    .inner_size();
                interface.update_window_size(ScreenSize {
                    width: window_size.width as f32,
                    height: window_size.height as f32,
                });
                swapchain_holder.update_window_size(window_size.into());
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(focused),
                ..
            } => {
                if !focused {
                    input_system.reset();
                    focus_state.remove_focus();
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                ..
            } => mouse_cursor.hide(),
            Event::WindowEvent {
                event: WindowEvent::CursorEntered { .. },
                ..
            } => mouse_cursor.show(),
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => input_system.update_mouse_position(position),
            Event::WindowEvent {
                event: WindowEvent::MouseInput { button, state, .. },
                ..
            } => input_system.update_mouse_buttons(button, state),
            Event::WindowEvent {
                event: WindowEvent::MouseWheel { delta, .. },
                ..
            } => input_system.update_mouse_wheel(delta),
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => {
                if let Some(keycode) = input.virtual_keycode {
                    input_system.update_keyboard(keycode, input.state)
                }
            }
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(character),
                ..
            } => input_system.buffer_character(character),
            Event::MainEventsCleared => {
                #[cfg(feature = "debug")]
                let _measurement = threads::Main::start_frame();

                #[cfg(feature = "debug")]
                let timer_measurement = Profiler::start_measurement("update timers");

                input_system.update_delta();

                let delta_time = game_timer.update();
                let day_timer = game_timer.get_day_timer();
                let animation_timer = game_timer.get_animation_timer();
                let client_tick = game_timer.get_client_tick();

                #[cfg(feature = "debug")]
                timer_measurement.stop();

                let network_events = networking_system.get_events();

                let (user_events, hovered_element, focused_element, mouse_target) = input_system.user_events(
                    &mut interface,
                    &application,
                    &mut focus_state,
                    &mut picker_targets[swapchain_holder.get_image_number()],
                    &mut mouse_cursor,
                    #[cfg(feature = "debug")]
                    &render_settings,
                    swapchain_holder.window_size(),
                    client_tick,
                );

                #[cfg(feature = "debug")]
                let picker_measurement = Profiler::start_measurement("update picker target");

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                    if let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                        if entity.are_details_unavailable() && networking_system.entity_details(entity_id).is_ok() {
                            entity.set_details_requested();
                        }

                        match entity.get_entity_type() {
                            EntityType::Npc => mouse_cursor.set_state(MouseCursorState::Dialog, client_tick),
                            EntityType::Warp => mouse_cursor.set_state(MouseCursorState::Warp, client_tick),
                            EntityType::Monster => mouse_cursor.set_state(MouseCursorState::Attack, client_tick),
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
                        NetworkEvent::LoginServerConnected { character_servers, login_data } => {
                            saved_login_data = Some(login_data);

                            interface.close_all_windows_except(&mut focus_state);
                            interface.open_window(&application, &mut focus_state, &SelectServerWindow::new(character_servers));
                        }
                        NetworkEvent::LoginServerConnectionFailed { message, .. } => {
                            interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message.to_owned()))
                        }
                        NetworkEvent::LoginServerDisconnected { reason } => {
                            if reason != DisconnectReason::ClosedByClient {
                                // TODO: Make this an on-screen popup.
                                #[cfg(feature = "debug")]
                                print_debug!("Disconnection from the character server with error");

                                let socket_address = saved_login_server_address.unwrap();
                                networking_system.connect_to_login_server(socket_address, &saved_username, &saved_password);
                            }
                        },
                        NetworkEvent::CharacterServerConnected { normal_slot_count } => {
                            saved_slot_count = normal_slot_count;
                            let _ = networking_system.request_character_list();
                        },
                        NetworkEvent::CharacterServerConnectionFailed { message, .. } => {
                            interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message.to_owned()))
                        },
                        NetworkEvent::CharacterServerDisconnected { reason } => {
                            if reason != DisconnectReason::ClosedByClient {
                                // TODO: Make this an on-screen popup.
                                #[cfg(feature = "debug")]
                                print_debug!("Disconnection from the character server with error");

                                let login_data = saved_login_data.as_ref().unwrap();
                                let server = saved_character_server.clone().unwrap();
                                networking_system.connect_to_character_server(login_data, server);
                            }
                        },
                        NetworkEvent::MapServerDisconnected {  reason } => {
                            if reason != DisconnectReason::ClosedByClient {
                                // TODO: Make this an on-screen popup.
                                #[cfg(feature = "debug")]
                                print_debug!("Disconnection from the map server with error");
                            }

                            let login_data = saved_login_data.as_ref().unwrap();
                            let server = saved_character_server.clone().unwrap();
                            networking_system.connect_to_character_server(login_data, server);

                            entities.clear();
                            particle_holder.clear();
                            effect_holder.clear();

                            map = map_loader
                                .get(
                                    DEFAULT_MAP.to_string(),
                                    &mut game_file_loader,
                                    &mut buffer_allocator,
                                    &mut model_loader,
                                    &mut texture_loader,
                                )
                                .expect("failed to load initial map");

                            interface.close_all_windows_except(&mut focus_state);

                            let character_selection_window = CharacterSelectionWindow::new(saved_characters.new_remote(), move_request.new_remote(), saved_slot_count);
                            interface.open_window(&application, &mut focus_state, &character_selection_window);

                            start_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
                            directional_shadow_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));

                        },
                        NetworkEvent::AccountId(..) => {},
                        NetworkEvent::CharacterList { characters } => {
                            saved_characters.set(characters);
                            let character_selection_window = CharacterSelectionWindow::new(saved_characters.new_remote(), move_request.new_remote(), saved_slot_count);

                            // TODO: this will do one unnecessary restore_focus. check if
                            // that will be problematic
                            interface.close_all_windows_except(&mut focus_state);
                            interface.open_window(&application, &mut focus_state, &character_selection_window);
                        }
                        NetworkEvent::CharacterSelectionFailed { message, .. } => {
                            interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message.to_owned()))
                        }
                        NetworkEvent::CharacterDeleted => {
                            let character_id = currently_deleting.take().unwrap();

                            saved_characters.retain(|character| character.character_id != character_id);
                        },
                        NetworkEvent::CharacterDeletionFailed { message, .. } => {
                            currently_deleting = None;
                            interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message.to_owned()))
                        }
                        NetworkEvent::CharacterSelected { login_data, map_name } => {
                            let saved_login_data = saved_login_data.as_ref().unwrap();
                            networking_system.connect_to_map_server(saved_login_data, login_data);

                            let character_information = saved_characters
                                .get()
                                .iter()
                                .find(|character| character.character_id == login_data.character_id)
                                .cloned()
                                .unwrap();

                            map = map_loader
                                .get(
                                    map_name,
                                    &mut game_file_loader,
                                    &mut buffer_allocator,
                                    &mut model_loader,
                                    &mut texture_loader,
                                )
                                .unwrap();

                            saved_player_name = character_information.name.clone();

                            let player = Player::new(
                                &mut game_file_loader,
                                &mut sprite_loader,
                                &mut action_loader,
                                &script_loader,
                                &map,
                                saved_login_data.account_id,
                                character_information,
                                WorldPosition { x: 0, y: 0 },
                                client_tick,
                            );
                            let player = Entity::Player(player);

                            player_camera.set_focus_point(player.get_position());
                            entities.push(player);

                            // TODO: this will do one unnecessary restore_focus. check if
                            // that will be problematic
                            interface.close_window_with_class(&mut focus_state, CharacterSelectionWindow::WINDOW_CLASS);
                            interface.open_window(&application, &mut focus_state, &CharacterOverviewWindow::new());
                            interface.open_window(
                                &application,
                                &mut focus_state,
                                &ChatWindow::new(chat_messages.new_remote(), font_loader.clone()),
                            );
                            interface.open_window(&application, &mut focus_state, &HotbarWindow::new(hotbar.get_skills()));

                            // Put the dialog system in a well-defined state.
                            dialog_system.close_dialog();

                            particle_holder.clear();
                            let _ = networking_system.map_loaded();
                            // TODO: This is just a workaround until I find a better solution to make the
                            // cursor always look correct.
                            mouse_cursor.set_start_time(client_tick);
                            game_timer.set_client_tick(client_tick);
                        },
                        NetworkEvent::CharacterCreated { character_information } => {
                            saved_characters.push(character_information);

                            interface.close_window_with_class(&mut focus_state, CharacterCreationWindow::WINDOW_CLASS);
                        },
                        NetworkEvent::CharacterCreationFailed { message, .. } => {
                            interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message.to_owned()));
                        },
                        NetworkEvent::CharacterSlotSwitched => {},
                        NetworkEvent::CharacterSlotSwitchFailed => {
                            interface.open_window(&application, &mut focus_state, &ErrorWindow::new("Failed to switch character slots".to_owned()));
                        },
                        NetworkEvent::AddEntity(entity_appeared_data) => {
                            // Sometimes (like after a job change) the server will tell the client
                            // that a new entity appeared, even though it was already on screen. So
                            // to prevent the entity existing twice, we remove the old one.
                            entities.retain(|entity| entity.get_entity_id() != entity_appeared_data.entity_id);

                            let npc = Npc::new(
                                &mut game_file_loader,
                                &mut sprite_loader,
                                &mut action_loader,
                                &script_loader,
                                &map,
                                entity_appeared_data,
                                client_tick,
                            );

                            let npc = Entity::Npc(npc);
                            entities.push(npc);
                        }
                        NetworkEvent::RemoveEntity(entity_id) => {
                            entities.retain(|entity| entity.get_entity_id() != entity_id);
                        }
                        NetworkEvent::EntityMove(entity_id, position_from, position_to, starting_timestamp) => {
                            let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                let position_from = Vector2::new(position_from.x, position_from.y);
                                let position_to = Vector2::new(position_to.x, position_to.y);

                                entity.move_from_to(&map, position_from, position_to, starting_timestamp);
                                /*#[cfg(feature = "debug")]
                                entity.generate_steps_vertex_buffer(device.clone(), &map);*/
                            }
                        }
                        NetworkEvent::PlayerMove(position_from, position_to, starting_timestamp) => {
                            let position_from = Vector2::new(position_from.x, position_from.y);
                            let position_to = Vector2::new(position_to.x, position_to.y);
                            entities[0].move_from_to(&map, position_from, position_to, starting_timestamp);

                            /*#[cfg(feature = "debug")]
                            entities[0].generate_steps_vertex_buffer(device.clone(), &map);*/
                        }
                        NetworkEvent::ChangeMap(map_name, player_position) => {
                            entities.truncate(1);

                            map = map_loader
                                .get(
                                    map_name,
                                    &mut game_file_loader,
                                    &mut buffer_allocator,
                                    &mut model_loader,
                                    &mut texture_loader,
                                )
                                .unwrap();

                            let player_position = Vector2::new(player_position.x as usize, player_position.y as usize);
                            entities[0].set_position(&map, player_position, client_tick);
                            player_camera.set_focus_point(entities[0].get_position());

                            particle_holder.clear();
                            effect_holder.clear();
                            let _ = networking_system.map_loaded();

                            // TODO: This is just a workaround until I find a better solution to make the
                            // cursor always look correct.
                            mouse_cursor.set_start_time(client_tick);
                        }
                        NetworkEvent::SetPlayerPosition(player_position) => {
                            let player_position = Vector2::new(player_position.x, player_position.y);
                            entities[0].set_position(&map, player_position, client_tick);
                            player_camera.set_focus_point(entities[0].get_position());
                        }
                        NetworkEvent::UpdateClientTick(client_tick) => {
                            game_timer.set_client_tick(client_tick);
                        }
                        NetworkEvent::ChatMessage { text, color } => {
                            chat_messages.push(ChatMessage { text, color });
                        }
                        NetworkEvent::UpdateEntityDetails(entity_id, name) => {
                            let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                entity.set_details(name);
                            }
                        }
                        NetworkEvent::DamageEffect(entity_id, damage_amount) => {
                            let entity = entities
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id)
                                .unwrap_or(&entities[0]);

                            particle_holder.spawn_particle(Box::new(DamageNumber::new(entity.get_position(), damage_amount.to_string())));
                        }
                        NetworkEvent::HealEffect(entity_id, damage_amount) => {
                            let entity = entities
                                .iter()
                                .find(|entity| entity.get_entity_id() == entity_id)
                                .unwrap_or(&entities[0]);

                            particle_holder.spawn_particle(Box::new(HealNumber::new(entity.get_position(), damage_amount.to_string())));
                        }
                        NetworkEvent::UpdateEntityHealth(entity_id, health_points, maximum_health_points) => {
                            let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                entity.update_health(health_points, maximum_health_points);
                            }
                        }
                        NetworkEvent::UpdateStatus(status_type) => {
                            let Entity::Player(player) = &mut entities[0] else {
                                panic!();
                            };

                            player.update_status(status_type);
                        }
                        NetworkEvent::OpenDialog(text, npc_id) => {
                            if let Some(dialog_window) = dialog_system.open_dialog_window(text, npc_id) {
                                interface.open_window(&application, &mut focus_state, &dialog_window);
                            }
                        }
                        NetworkEvent::AddNextButton => dialog_system.add_next_button(),
                        NetworkEvent::AddCloseButton => dialog_system.add_close_button(),
                        NetworkEvent::AddChoiceButtons(choices) => dialog_system.add_choice_buttons(choices),
                        NetworkEvent::AddQuestEffect(quest_effect) => {
                            particle_holder.add_quest_icon(&mut game_file_loader, &mut texture_loader, &map, quest_effect)
                        }
                        NetworkEvent::RemoveQuestEffect(entity_id) => particle_holder.remove_quest_icon(entity_id),
                        NetworkEvent::SetInventory { items } => {
                            player_inventory.fill(&mut game_file_loader, &mut texture_loader, &script_loader, items);
                        }
                        NetworkEvent::IventoryItemAdded {
                            item
                        }=> {
                            player_inventory.add_item(
                                &mut game_file_loader,
                                &mut texture_loader,
                                &script_loader,
                                item,
                            );

                            // TODO: Update the selling items. If you pick up an item that you
                            // already have the sell window should allow you to sell the new amount
                            // of items.
                        }
                        NetworkEvent::InventoryItemRemoved { reason: _reason, index, amount } => {
                            player_inventory.remove_item(
                                index, amount,
                            );
                        }
                        NetworkEvent::SkillTree(skill_information) => {
                            player_skill_tree.fill(&mut game_file_loader, &mut sprite_loader, &mut action_loader, skill_information);
                        }
                        NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                            player_inventory.update_equipped_position(index, equipped_position);
                        }
                        NetworkEvent::ChangeJob(account_id, job_id) => {
                            let entity = entities.iter_mut().find(|entity| entity.get_entity_id().0 == account_id.0).unwrap();

                            // FIX: A job change does not automatically send packets for the
                            // inventory and for unequipping items. We should probably manually
                            // request a full list of items and the hotbar.

                            entity.set_job(job_id as usize);
                            entity.reload_sprite(&mut game_file_loader, &mut sprite_loader, &mut action_loader, &script_loader);
                        }
                        NetworkEvent::LoggedOut => {
                            networking_system.disconnect_from_map_server();
                        }
                        NetworkEvent::FriendRequest { requestee } => {
                            interface.open_window(&application, &mut focus_state, &FriendRequestWindow::new(requestee))
                        }
                        NetworkEvent::FriendRemoved { account_id, character_id } => {
                            friend_list.retain(|(friend, _)| !(friend.account_id == account_id && friend.character_id == character_id));
                        }
                        NetworkEvent::FriendAdded { friend } => {
                            friend_list.push((friend, LinkedElement::new()));
                        }
                        NetworkEvent::VisualEffect(path, entity_id) => {
                            let effect = effect_loader.get(path, &mut game_file_loader, &mut texture_loader).unwrap();
                            let frame_timer = effect.new_frame_timer();

                            effect_holder.add_effect(Box::new(EffectWithLight::new(
                                effect,
                                frame_timer,
                                EffectCenter::Entity(entity_id, cgmath::Vector3::new(0.0, 0.0, 0.0)),
                                Vector3::new(0.0, 9.0, 0.0),
                                Vector3::new(0.0, 12.0, 0.0),
                                Color::monochrome_u8(255),
                                50.0,
                                false,
                            )));
                        }
                        NetworkEvent::AddSkillUnit(entity_id, unit_id, position) => match unit_id {
                            UnitId::Firewall => {
                                let position = Vector2::new(position.x as usize, position.y as usize);
                                let position = map.get_world_position(position);
                                let effect = effect_loader
                                    .get("firewall.str", &mut game_file_loader, &mut texture_loader)
                                    .unwrap();
                                let frame_timer = effect.new_frame_timer();

                                effect_holder.add_unit(
                                    Box::new(EffectWithLight::new(
                                        effect,
                                        frame_timer,
                                        EffectCenter::Position(position),
                                        Vector3::new(0.0, 0.0, 0.0),
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
                                let position = map.get_world_position(position);
                                let effect = effect_loader
                                    .get("pneuma1.str", &mut game_file_loader, &mut texture_loader)
                                    .unwrap();
                                let frame_timer = effect.new_frame_timer();

                                effect_holder.add_unit(
                                    Box::new(EffectWithLight::new(
                                        effect,
                                        frame_timer,
                                        EffectCenter::Position(position),
                                        Vector3::new(0.0, 0.0, 0.0),
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
                            effect_holder.remove_unit(entity_id);
                        }
                        NetworkEvent::SetFriendList { friends } => {
                            friend_list.mutate(|friend_list| {
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
                                        let Some(mut skill) = player_skill_tree.find_skill(SkillId(hotkey.skill_id as u16)) else {
                                            hotbar.clear_slot(&mut networking_system, HotbarSlot(index as u16));
                                            continue;
                                        };

                                        skill.skill_level = hotkey.quantity_or_skill_level;
                                        hotbar.set_slot(HotbarSlot(index as u16), skill);
                                    },
                                    HotkeyState::Unbound => hotbar.unset_slot(HotbarSlot(index as u16)),
                                }
                            }
                        }
                        NetworkEvent::OpenShop { items } => {
                            shop_items.mutate(|shop_items| *shop_items = items.into_iter().map(|item| {
                                script_loader.load_market_item_metadata(&mut game_file_loader, &mut texture_loader, item)
                            }).collect());

                            let cart = PlainTrackedState::default();

                            interface.open_window(&application, &mut focus_state, &BuyWindow::new(shop_items.new_remote(), cart.clone()));
                            interface.open_window(&application, &mut focus_state, &BuyCartWindow::new(cart));
                        }
                        NetworkEvent::AskBuyOrSell { shop_id } => {
                            interface.open_window(&application, &mut focus_state, &BuyOrSellWindow::new(shop_id));
                        }
                        NetworkEvent::BuyingCompleted { result } => {
                            match result {
                                BuyShopItemsResult::Success => {
                                    let _ = networking_system.close_shop();

                                    interface.close_window_with_class(&mut focus_state, BuyWindow::WINDOW_CLASS);
                                    interface.close_window_with_class(&mut focus_state, BuyCartWindow::WINDOW_CLASS);
                                }
                                BuyShopItemsResult::Error => {
                                    chat_messages.push(ChatMessage {
                                        text: "Failed to buy items".to_owned(),
                                        color: MessageColor::Error,
                                    });
                                },
                            }
                        },
                        NetworkEvent::SellItemList { items } => {
                            let inventory_items = player_inventory.get_items();

                            sell_items.mutate(|sell_items| *sell_items = items.into_iter().map(|item| {
                                let inventory_item = &inventory_items.iter().find(|inventory_item| inventory_item.index == item.inventory_index).expect("item not in inventory");

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
                            }).collect());

                            let cart = PlainTrackedState::default();

                            interface.open_window(&application, &mut focus_state, &SellWindow::new(sell_items.new_remote(), cart.clone()));
                            interface.open_window(&application, &mut focus_state, &SellCartWindow::new(cart.clone()));
                        }
                        NetworkEvent::SellingCompleted { result } => {
                            match result {
                                SellItemsResult::Success => {
                                    interface.close_window_with_class(&mut focus_state, SellWindow::WINDOW_CLASS);
                                    interface.close_window_with_class(&mut focus_state, SellCartWindow::WINDOW_CLASS);
                                }
                                SellItemsResult::Error => {
                                    chat_messages.push(ChatMessage {
                                        text: "Failed to sell items".to_owned(),
                                        color: MessageColor::Error,
                                    });
                                },
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
                            let service = client_info
                                .services
                                .iter()
                                .find(|service| service.service_id() == service_id)
                                .unwrap();
                            let address = format!("{}:{}", service.address, service.port);
                            let socket_address = address.to_socket_addrs().expect("Failed to resolve IP").next().expect("ill formatted service IP");

                            saved_login_server_address = Some(socket_address);
                            saved_username = username.clone();
                            saved_password = password.clone();

                            networking_system.connect_to_login_server(socket_address, username, password);
                        }
                        UserEvent::SelectServer(server) => {
                            saved_character_server = Some(server.clone());

                            networking_system.disconnect_from_login_server();

                            // Korangar should never attempt to connect to the character
                            // server before it logged in to the login server, so it's fine to
                            // unwrap here.
                            let login_data = saved_login_data.as_ref().unwrap();
                            networking_system.connect_to_character_server(login_data, server);
                        }
                        UserEvent::LogOut => {
                            let _ = networking_system.log_out();
                        },
                        UserEvent::Exit => *control_flow = ControlFlow::Exit,
                        UserEvent::CameraZoom(factor) => player_camera.soft_zoom(factor),
                        UserEvent::CameraRotate(factor) => player_camera.soft_rotate(factor),
                        UserEvent::OpenMenuWindow => {
                            if !entities.is_empty() {
                                interface.open_window(&application, &mut focus_state, &MenuWindow)
                            }
                        }
                        UserEvent::OpenInventoryWindow => {
                            if !entities.is_empty() {
                                interface.open_window(
                                    &application,
                                    &mut focus_state,
                                    &InventoryWindow::new(player_inventory.item_remote()),
                                )
                            }
                        }
                        UserEvent::OpenEquipmentWindow => {
                            if !entities.is_empty() {
                                interface.open_window(
                                    &application,
                                    &mut focus_state,
                                    &EquipmentWindow::new(player_inventory.item_remote()),
                                )
                            }
                        }
                        UserEvent::OpenSkillTreeWindow => {
                            if !entities.is_empty() {
                                interface.open_window(
                                    &application,
                                    &mut focus_state,
                                    &SkillTreeWindow::new(player_skill_tree.get_skills()),
                                )
                            }
                        }
                        UserEvent::OpenGraphicsSettingsWindow => interface.open_window(
                            &application,
                            &mut focus_state,
                            &GraphicsSettingsWindow::new(present_mode_info, shadow_detail.clone_state(), framerate_limit.clone_state()),
                        ),
                        UserEvent::OpenAudioSettingsWindow => interface.open_window(&application, &mut focus_state, &AudioSettingsWindow),
                        UserEvent::OpenFriendsWindow => {
                            interface.open_window(&application, &mut focus_state, &FriendsWindow::new(friend_list.new_remote()));
                        }
                        UserEvent::ToggleShowInterface => show_interface = !show_interface,
                        UserEvent::SetThemeFile { theme_file, theme_kind } => application.set_theme_file(theme_file, theme_kind),
                        UserEvent::SaveTheme { theme_kind } => application.save_theme(theme_kind),
                        UserEvent::ReloadTheme { theme_kind } => application.reload_theme(theme_kind),
                        UserEvent::SelectCharacter(character_slot) => {
                            let _ = networking_system.select_character(character_slot);
                        },
                        UserEvent::OpenCharacterCreationWindow(character_slot) => {
                            interface.open_window(&application, &mut focus_state, &CharacterCreationWindow::new(character_slot))
                        }
                        UserEvent::CreateCharacter(character_slot, name) => {
                            let _ = networking_system.create_character(character_slot, name);
                        },
                        UserEvent::DeleteCharacter(character_id) => {
                            if currently_deleting.is_none() {
                                let _ = networking_system.delete_character(character_id);
                                currently_deleting = Some(character_id);
                            }
                        },
                        UserEvent::RequestSwitchCharacterSlot(origin_slot) => move_request.set(Some(origin_slot)),
                        UserEvent::CancelSwitchCharacterSlot => move_request.set(None),
                        UserEvent::SwitchCharacterSlot(destination_slot) => {
                            let _ = networking_system.switch_character_slot(move_request.take().unwrap(), destination_slot);
                        },
                        UserEvent::RequestPlayerMove(destination) => {
                            if !entities.is_empty() {
                                let _ = networking_system.player_move(WorldPosition { x: destination.x, y: destination.y });
                            }
                        }
                        UserEvent::RequestPlayerInteract(entity_id) => {
                            let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                let _ = match entity.get_entity_type() {
                                    EntityType::Npc => networking_system.start_dialog(entity_id),
                                    EntityType::Monster => networking_system.player_attack(entity_id),
                                    EntityType::Warp => networking_system.player_move({
                                        let position = entity.get_grid_position();
                                        WorldPosition { x: position.x, y: position.y }
                                    }),
                                    _ => Ok(())
                                };
                            }
                        }
                        UserEvent::RequestWarpToMap(map_name, position) => {
                            let _ = networking_system.warp_to_map(map_name, position);
                        },
                        UserEvent::SendMessage(message) => {
                            let _ = networking_system.send_chat_message(&saved_player_name, &message);
                            // TODO: maybe find a better solution for unfocusing the message box if
                            // this becomes problematic
                            focus_state.remove_focus();
                        }
                        UserEvent::NextDialog(npc_id) => {
                            let _ = networking_system.next_dialog(npc_id);
                        },
                        UserEvent::CloseDialog(npc_id) => {
                            let _ = networking_system.close_dialog(npc_id);
                            dialog_system.close_dialog();
                            interface.close_window_with_class(&mut focus_state, DialogWindow::WINDOW_CLASS);
                        }
                        UserEvent::ChooseDialogOption(npc_id, option) => {
                            let _ = networking_system.choose_dialog_option(npc_id, option);

                            if option == -1 {
                                dialog_system.close_dialog();
                                interface.close_window_with_class(&mut focus_state, DialogWindow::WINDOW_CLASS);
                            }
                        }
                        UserEvent::MoveResource(r#move) => {
                            match r#move {
                                Move::Item { source, destination, item } => match (source, destination) {
                                    (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                                        let _ = networking_system.request_item_equip(item.index, position);
                                    }
                                    (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                                        let _ = networking_system.request_item_unequip(item.index);
                                    }
                                    _ => {}
                                },
                                Move::Skill {
                                    source,
                                    destination,
                                    skill,
                                } => match (source, destination) {
                                    (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                                        hotbar.update_slot(&mut networking_system, slot, skill);
                                    }
                                    (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                                        hotbar.swap_slot(&mut networking_system, source_slot, destination_slot);
                                    }
                                    _ => {}
                                },
                            }
                        },
                        UserEvent::CastSkill(slot) => {
                            if let Some(skill) = hotbar.get_skill_in_slot(slot).as_ref() {
                                match skill.skill_type {
                                    SkillType::Passive => {}
                                    SkillType::Attack => {
                                        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                            let _ = networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                        }
                                    }
                                    SkillType::Ground | SkillType::Trap => {
                                        if let Some(PickerTarget::Tile { x, y }) = mouse_target {
                                            let _ = networking_system.cast_ground_skill(skill.skill_id, skill.skill_level, TilePosition { x, y });
                                        }
                                    }
                                    SkillType::SelfCast => match skill.skill_id == ROLLING_CUTTER_ID {
                                        true => {
                                            let _ = networking_system.cast_channeling_skill(
                                                skill.skill_id,
                                                skill.skill_level,
                                                entities[0].get_entity_id(),
                                            );
                                        },
                                        false => {
                                            let _ = networking_system.cast_skill(skill.skill_id, skill.skill_level, entities[0].get_entity_id());
                                        }
                                    },
                                    SkillType::Support => {
                                        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                            let _ = networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                        } else {
                                            let _ = networking_system.cast_skill(skill.skill_id, skill.skill_level, entities[0].get_entity_id());
                                        }
                                    }
                                }
                            }
                        }
                        UserEvent::StopSkill(slot) => {
                            if let Some(skill) = hotbar.get_skill_in_slot(slot).as_ref() {
                                if skill.skill_id == ROLLING_CUTTER_ID {
                                    let _ = networking_system.stop_channeling_skill(skill.skill_id);
                                }
                            }
                        }
                        UserEvent::AddFriend(name) => {
                            if name.len() > 24 {
                                #[cfg(feature = "debug")]
                                print_debug!("[{}] friend name {} is too long", "error".red(), name.magenta());
                            } else {
                                let _ = networking_system.add_friend(name);
                            }
                        }
                        UserEvent::RemoveFriend { account_id, character_id } => {
                            let _ = networking_system.remove_friend(account_id, character_id);
                        }
                        UserEvent::RejectFriendRequest { account_id, character_id } => {
                            let _ = networking_system.reject_friend_request(account_id, character_id);
                            interface.close_window_with_class(&mut focus_state, FriendRequestWindow::WINDOW_CLASS);
                        }
                        UserEvent::AcceptFriendRequest { account_id, character_id } => {
                            let _ = networking_system.accept_friend_request(account_id, character_id);
                            interface.close_window_with_class(&mut focus_state, FriendRequestWindow::WINDOW_CLASS);
                        }
                        UserEvent::BuyItems { items } => {
                            let _ = networking_system.purchase_items(items);
                        }
                        UserEvent::CloseShop => {
                            let _ = networking_system.close_shop();

                            interface.close_window_with_class(&mut focus_state, BuyWindow::WINDOW_CLASS);
                            interface.close_window_with_class(&mut focus_state, BuyCartWindow::WINDOW_CLASS);
                            interface.close_window_with_class(&mut focus_state, SellWindow::WINDOW_CLASS);
                            interface.close_window_with_class(&mut focus_state, SellCartWindow::WINDOW_CLASS);
                        }
                        UserEvent::BuyOrSell { shop_id, buy_or_sell } => {
                            let _ = networking_system.select_buy_or_sell(shop_id, buy_or_sell);
                            interface.close_window_with_class(&mut focus_state, BuyOrSellWindow::WINDOW_CLASS);
                        },
                        UserEvent::SellItems { items } => {
                            let _ = networking_system.sell_items(items);
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMarkerDetails(marker_identifier) => {
                            interface.open_window(&application, &mut focus_state, map.resolve_marker(&entities, marker_identifier))
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::OpenRenderSettingsWindow => interface.open_window(
                            &application,
                            &mut focus_state,
                            &RenderSettingsWindow::new(render_settings.clone()),
                        ),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapDataWindow => interface.open_window(&application, &mut focus_state, map.to_prototype_window()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapsWindow => interface.open_window(&application, &mut focus_state, &MapsWindow),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenCommandsWindow => interface.open_window(&application, &mut focus_state, &CommandsWindow),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenTimeWindow => interface.open_window(&application, &mut focus_state, &TimeWindow),
                        #[cfg(feature = "debug")]
                        UserEvent::SetDawn => game_timer.set_day_timer(0.0),
                        #[cfg(feature = "debug")]
                        UserEvent::SetNoon => game_timer.set_day_timer(std::f32::consts::FRAC_PI_2),
                        #[cfg(feature = "debug")]
                        UserEvent::SetDusk => game_timer.set_day_timer(std::f32::consts::PI),
                        #[cfg(feature = "debug")]
                        UserEvent::SetMidnight => game_timer.set_day_timer(-std::f32::consts::FRAC_PI_2),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenThemeViewerWindow => {
                            interface.open_window(&application, &mut focus_state, application.theme_window())
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::OpenProfilerWindow => interface.open_window(&application, &mut focus_state, &ProfilerWindow::new()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenPacketWindow => {
                            interface.open_window(&application, &mut focus_state, &PacketWindow::new(packet_callback.remote(), PlainTrackedState::default()))
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::ClearPacketHistory => packet_callback.clear_all(),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraLookAround(offset) => debug_camera.look_around(offset),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraMoveForward => debug_camera.move_forward(delta_time as f32),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraMoveBackward => debug_camera.move_backward(delta_time as f32),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraMoveLeft => debug_camera.move_left(delta_time as f32),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraMoveRight => debug_camera.move_right(delta_time as f32),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraMoveUp => debug_camera.move_up(delta_time as f32),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraAccelerate => debug_camera.accelerate(),
                        #[cfg(feature = "debug")]
                        UserEvent::CameraDecelerate => debug_camera.decelerate(),
                    }
                }

                #[cfg(feature = "debug")]
                user_event_measurement.stop();

                let buffer_fence = buffer_allocator.submit_load_buffer();
                let texture_fence = texture_loader.submit_load_buffer();
                let sprite_fence = sprite_loader.submit_load_buffer();

                #[cfg(feature = "debug")]
                let update_entities_measurement = Profiler::start_measurement("update entities");

                entities
                    .iter_mut()
                    .for_each(|entity| entity.update(&map, delta_time as f32, client_tick));

                #[cfg(feature = "debug")]
                update_entities_measurement.stop();

                if !entities.is_empty() {
                    let player_position = entities[0].get_position();
                    player_camera.set_smoothed_focus_point(player_position);
                    directional_shadow_camera.set_focus_point(player_camera.get_focus_point());
                }

                #[cfg(feature = "debug")]
                let update_cameras_measurement = Profiler::start_measurement("update cameras");

                start_camera.update(delta_time);
                player_camera.update(delta_time);
                directional_shadow_camera.update(day_timer);

                #[cfg(feature = "debug")]
                update_cameras_measurement.stop();

                particle_holder.update(delta_time as f32);
                effect_holder.update(&entities, delta_time as f32);

                let (clear_interface, render_interface) = interface.update(&application, font_loader.clone(), &mut focus_state);
                mouse_cursor.update(client_tick);

                if swapchain_holder.is_swapchain_invalid() {
                    #[cfg(feature = "debug")]
                    profile_block!("re-create buffers");

                    let viewport = swapchain_holder.recreate_swapchain();

                    deferred_renderer.recreate_pipeline(
                        viewport.clone(),
                        swapchain_holder.window_size_u32(),
                        #[cfg(feature = "debug")]
                        render_settings.get().show_wireframe,
                    );
                    interface_renderer.recreate_pipeline(viewport.clone(), swapchain_holder.window_size_u32());
                    picker_renderer.recreate_pipeline(viewport, swapchain_holder.window_size_u32());

                    screen_targets = swapchain_holder
                        .get_swapchain_images()
                        .into_iter()
                        .map(|swapchain_image| deferred_renderer.create_render_target(swapchain_image))
                        .collect();

                    interface_target = interface_renderer.create_render_target();

                    picker_targets = swapchain_holder
                        .get_swapchain_images()
                        .into_iter()
                        .map(|_| picker_renderer.create_render_target())
                        .collect();
                }

                if swapchain_holder.acquire_next_image().is_err() {
                    // temporary check?
                    return;
                }

                if shadow_detail.consume_changed() {
                    #[cfg(feature = "debug")]
                    print_debug!("re-creating {}", "directional shadow targets".magenta());

                    #[cfg(feature = "debug")]
                    profile_block!("re-create shadow maps");

                    let new_shadow_detail = shadow_detail.get();

                    directional_shadow_targets = swapchain_holder
                        .get_swapchain_images()
                        .into_iter()
                        .map(|_| shadow_renderer.create_render_target(new_shadow_detail.into_resolution()))
                        .collect::<Vec<<ShadowRenderer as Renderer>::Target>>();
                }

                if framerate_limit.consume_changed() {
                    swapchain_holder.set_frame_limit(present_mode_info, framerate_limit.cloned());

                    // For some reason the interface buffer becomes messed up when
                    // recreating the swapchain, so we need to render it again.
                    interface.schedule_render();
                }

                #[cfg(feature = "debug")]
                let matrices_measurement = Profiler::start_measurement("generate view and projection matrices");

                if entities.is_empty() {
                    start_camera.generate_view_projection(swapchain_holder.window_size());
                }

                player_camera.generate_view_projection(swapchain_holder.window_size());
                directional_shadow_camera.generate_view_projection(swapchain_holder.window_size());
                #[cfg(feature = "debug")]
                if render_settings.get().use_debug_camera {
                    debug_camera.generate_view_projection(swapchain_holder.window_size());
                }

                #[cfg(feature = "debug")]
                matrices_measurement.stop();

                let current_camera: &(dyn Camera + Send + Sync) = match entities.is_empty() {
                    #[cfg(feature = "debug")]
                    _ if render_settings.get().use_debug_camera => &debug_camera,
                    true => &start_camera,
                    false => &player_camera,
                };

                if let Some(mut fence) = screen_targets[swapchain_holder.get_image_number()].state.try_take_fence() {
                    #[cfg(feature = "debug")]
                    profile_block!("wait for frame in current slot");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = buffer_fence {
                    #[cfg(feature = "debug")]
                    profile_block!("wait for buffers");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = texture_fence {
                    #[cfg(feature = "debug")]
                    profile_block!("wait for textures");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = sprite_fence {
                    #[cfg(feature = "debug")]
                    profile_block!("wait for sprites");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                #[cfg(feature = "debug")]
                let prepare_frame_measurement = Profiler::start_measurement("prepare frame");

                #[cfg(feature = "debug")]
                let render_settings = &*render_settings.get();
                let walk_indicator_color = application.get_game_theme().indicator.walking.get();
                let image_number = swapchain_holder.get_image_number();
                let directional_shadow_image = directional_shadow_targets[image_number].image.clone();
                let screen_target = &mut screen_targets[image_number];
                let window_size = swapchain_holder.window_screen_size();
                let window_size_u32 = swapchain_holder.window_size_u32();
                let entities = &entities[..];
                #[cfg(feature = "debug")]
                let hovered_marker_identifier = match mouse_target {
                    Some(PickerTarget::Marker(marker_identifier)) => Some(marker_identifier),
                    _ => None,
                };

                #[cfg(feature = "debug")]
                prepare_frame_measurement.stop();

                thread_pool.in_place_scope(|scope| {
                    scope.spawn(|_| {
                        #[cfg(feature = "debug")]
                        let _measurement = threads::Picker::start_frame();

                        let picker_target = &mut picker_targets[image_number];

                        picker_target.start();

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
                        map.render_tiles(picker_target, &picker_renderer, current_camera);

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
                        map.render_entities(entities, picker_target, &picker_renderer, current_camera, false);

                        #[cfg(feature = "debug")]
                        map.render_markers(
                            picker_target,
                            &picker_renderer,
                            current_camera,
                            render_settings,
                            entities,
                            hovered_marker_identifier,
                        );

                        picker_target.finish();
                    });

                    scope.spawn(|_| {
                        #[cfg(feature = "debug")]
                        let _measurement = threads::Shadow::start_frame();

                        let directional_shadow_target = &mut directional_shadow_targets[image_number];

                        directional_shadow_target.start();

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
                        map.render_ground(
                            directional_shadow_target,
                            &shadow_renderer,
                            &directional_shadow_camera,
                            animation_timer,
                        );

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
                        map.render_objects(
                            directional_shadow_target,
                            &shadow_renderer,
                            &directional_shadow_camera,
                            client_tick,
                            animation_timer,
                            #[cfg(feature = "debug")]
                            render_settings.frustum_culling,
                        );

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
                        map.render_entities(
                            entities,
                            directional_shadow_target,
                            &shadow_renderer,
                            &directional_shadow_camera,
                            true,
                        );

                        if let Some(PickerTarget::Tile { x, y }) = mouse_target
                            && !entities.is_empty()
                        {
                            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators))]
                            map.render_walk_indicator(
                                directional_shadow_target,
                                &shadow_renderer,
                                &directional_shadow_camera,
                                walk_indicator_color,
                                Vector2::new(x as usize, y as usize),
                            );
                        }

                        directional_shadow_target.finish();
                    });

                    scope.spawn(|_| {
                        #[cfg(feature = "debug")]
                        let _measurement = threads::Deferred::start_frame();

                        screen_target.start();

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_map))]
                        map.render_ground(screen_target, &deferred_renderer, current_camera, animation_timer);

                        #[cfg(feature = "debug")]
                        if render_settings.show_map_tiles {
                            map.render_overlay_tiles(screen_target, &deferred_renderer, current_camera);
                        }

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_objects))]
                        map.render_objects(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            client_tick,
                            animation_timer,
                            #[cfg(feature = "debug")]
                            render_settings.frustum_culling,
                        );

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_entities))]
                        map.render_entities(entities, screen_target, &deferred_renderer, current_camera, true);

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_water))]
                        map.render_water(screen_target, &deferred_renderer, current_camera, animation_timer);

                        if let Some(PickerTarget::Tile { x, y }) = mouse_target
                            && !entities.is_empty()
                        {
                            #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_indicators))]
                            map.render_walk_indicator(
                                screen_target,
                                &deferred_renderer,
                                current_camera,
                                walk_indicator_color,
                                Vector2::new(x as usize, y as usize),
                            );
                        }

                        screen_target.lighting_pass();

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_ambient_light && !render_settings.show_buffers()))]
                        map.ambient_light(screen_target, &deferred_renderer, day_timer);

                        let (view_matrix, projection_matrix) = directional_shadow_camera.view_projection_matrices();
                        let light_matrix = projection_matrix * view_matrix;

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_directional_light && !render_settings.show_buffers()))]
                        map.directional_light(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            directional_shadow_image.clone(),
                            light_matrix,
                            day_timer,
                        );

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_point_lights && !render_settings.show_buffers()))]
                        map.point_lights(screen_target, &deferred_renderer, current_camera);

                        #[cfg_attr(feature = "debug", korangar_debug::debug_condition(render_settings.show_water && !render_settings.show_buffers()))]
                        map.water_light(screen_target, &deferred_renderer, current_camera);

                        #[cfg(feature = "debug")]
                        map.render_markers(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            render_settings,
                            entities,
                            hovered_marker_identifier,
                        );

                        #[cfg(feature = "debug")]
                        if render_settings.show_bounding_boxes {
                            map.render_bounding(
                                screen_target,
                                &deferred_renderer,
                                current_camera,
                                &player_camera,
                                render_settings.frustum_culling,
                            );
                        }

                        #[cfg(feature = "debug")]
                        if let Some(marker_identifier) = hovered_marker_identifier {
                            map.render_marker_box(screen_target, &deferred_renderer, current_camera, marker_identifier);
                        }

                        particle_holder.render(screen_target, &deferred_renderer, current_camera, window_size, entities);
                        effect_holder.render(screen_target, &deferred_renderer, current_camera);
                    });

                    if render_interface {
                        #[cfg(feature = "debug")]
                        profile_block!("render user interface");

                        interface_target.start(window_size_u32, clear_interface);

                        interface.render(
                            &mut interface_target,
                            &interface_renderer,
                            &application,
                            hovered_element,
                            focused_element,
                            input_system.get_mouse_mode(),
                        );

                        let font_future = font_loader.borrow_mut().submit_load_buffer();
                        interface_target.finish(font_future);
                    }
                });

                #[cfg(feature = "debug")]
                if render_settings.show_buffers() {
                    let picker_target = &mut picker_targets[image_number];

                    if let Some(fence) = picker_target.state.try_take_fence() {
                        fence.wait(None).unwrap();
                    }

                    deferred_renderer.overlay_buffers(
                        screen_target,
                        picker_target.image.clone(),
                        directional_shadow_image,
                        font_loader.borrow().get_font_atlas(),
                        render_settings,
                    );
                }

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                    #[cfg(feature = "debug")]
                    profile_block!("render hovered entity status");

                    let entity = entities.iter().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.render_status(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            application.get_game_theme(),
                            window_size,
                        );

                        if let Some(name) = &entity.get_details() {
                            let name = name.split('#').next().unwrap();

                            let offset = ScreenPosition {
                                left: name.len() as f32 * -3.0,
                                top: 20.0,
                            };

                            deferred_renderer.render_text(
                                screen_target,
                                name,
                                input_system.get_mouse_position() + offset + ScreenPosition::uniform(1.0),
                                Color::monochrome_u8(0),
                                FontSize::new(12.0),
                            ); // TODO: move variables into theme

                            deferred_renderer.render_text(
                                screen_target,
                                name,
                                input_system.get_mouse_position() + offset,
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
                        screen_target,
                        &deferred_renderer,
                        current_camera,
                        application.get_game_theme(),
                        window_size,
                    );
                }

                #[cfg(feature = "debug")]
                if render_settings.show_frames_per_second {
                    let game_theme = application.get_game_theme();

                    deferred_renderer.render_text(
                        screen_target,
                        &game_timer.last_frames_per_second().to_string(),
                        game_theme.overlay.text_offset.get().scaled(application.get_scaling()),
                        game_theme.overlay.foreground_color.get(),
                        game_theme.overlay.font_size.get().scaled(application.get_scaling()),
                    );
                }

                if show_interface {
                    deferred_renderer.overlay_interface(screen_target, interface_target.image.clone());

                    mouse_cursor.render(
                        screen_target,
                        &deferred_renderer,
                        input_system.get_mouse_position(),
                        input_system.get_mouse_mode().grabbed(),
                        application.get_game_theme().cursor.color.get(),
                        &application,
                    );
                }

                #[cfg(feature = "debug")]
                let finalize_frame_measurement = Profiler::start_measurement("finalize frame");

                let interface_future = interface_target
                    .state
                    .try_take_semaphore()
                    .unwrap_or_else(|| now(device.clone()).boxed());
                let directional_shadow_future = directional_shadow_targets[image_number].state.take_semaphore();
                let swapchain_acquire_future = swapchain_holder.take_acquire_future();

                let combined_future = interface_future
                    .join(directional_shadow_future)
                    .join(swapchain_acquire_future)
                    .boxed();

                screen_target.finish(swapchain_holder.get_swapchain(), combined_future, image_number);

                #[cfg(feature = "debug")]
                finalize_frame_measurement.stop();
            }
            _ignored => {},
        }
    });
}
