#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(adt_const_params)]
#![feature(auto_traits)]
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
mod network;
mod world;

use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::Arc;

use cgmath::{Vector2, Vector3, Zero};
use image::io::Reader as ImageReader;
use image::{EncodableLayout, ImageFormat};
use korangar_interface::application::{Application, FocusState, FontSizeTrait, FontSizeTraitExt, PositionTraitExt};
use korangar_interface::state::{PlainTrackedState, Remote, RemoteClone, TrackedState, TrackedStateVec};
use korangar_interface::Interface;
use ragnarok_networking::{SkillId, SkillType, UnitId};
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
use vulkano::instance::debug::DebugUtilsMessengerCallback;
#[cfg(feature = "debug")]
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCreateInfo};
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
use crate::interface::resource::{ItemSource, Move, SkillSource};
use crate::interface::windows::{
    AudioSettingsWindow, CharacterCreationWindow, CharacterOverviewWindow, CharacterSelectionWindow, ChatWindow, DialogWindow,
    EquipmentWindow, ErrorWindow, FriendRequestWindow, GraphicsSettingsWindow, HotbarWindow, InventoryWindow, LoginWindow, MenuWindow,
    SelectServerWindow, SkillTreeWindow,
};
#[cfg(feature = "debug")]
use crate::interface::windows::{CommandsWindow, MapsWindow, ProfilerWindow, RenderSettingsWindow, TimeWindow};
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::*;
use crate::network::{ChatMessage, NetworkEvent, NetworkingSystem};
#[cfg(feature = "debug")]
use crate::system::vulkan_message_callback;
use crate::system::{choose_physical_device, get_device_extensions, get_layers, GameTimer};
use crate::world::*;

const ROLLING_CUTTER_ID: SkillId = SkillId(2036);

fn main() {
    const DEFAULT_MAP: &str = "geffen";

    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = korangar_debug::profiler_start_main_thread();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("create device");

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
    korangar_debug::print_debug!("created {}instance{}", korangar_debug::MAGENTA, korangar_debug::NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("create window");

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
    korangar_debug::print_debug!("created {}window{}", korangar_debug::MAGENTA, korangar_debug::NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("choose physical device");

    let desired_device_extensions = get_device_extensions();
    let (physical_device, queue_family_index) = choose_physical_device(&instance, &surface, &desired_device_extensions);

    let present_mode_info = PresentModeInfo::from_device(&physical_device, &surface);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("create device");

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
    korangar_debug::print_debug!("created {}vulkan device{}", korangar_debug::MAGENTA, korangar_debug::NONE);

    let queue = queues.next().unwrap();

    #[cfg(feature = "debug")]
    korangar_debug::print_debug!(
        "received {}queue{} from {}device{}",
        korangar_debug::MAGENTA,
        korangar_debug::NONE,
        korangar_debug::MAGENTA,
        korangar_debug::NONE
    );

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("create resource managers");

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
    let timer = korangar_debug::Timer::new("load resources");

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
    let timer = korangar_debug::Timer::new("create swapchain");

    let mut swapchain_holder = SwapchainHolder::new(&physical_device, device.clone(), queue.clone(), surface.clone());
    let viewport = swapchain_holder.viewport();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("create renderers");

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

    fn handle_result<T>(
        application: &InterfaceSettings,
        interface: &mut Interface<InterfaceSettings>,
        focus_state: &mut FocusState<InterfaceSettings>,
        result: Result<T, String>,
    ) {
        if let Err(message) = result {
            interface.open_window(application, focus_state, &ErrorWindow::new(message));
        }
    }

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
    let timer = korangar_debug::Timer::new("load settings");

    let mut input_system = InputSystem::new();
    let graphics_settings = PlainTrackedState::new(GraphicsSettings::new());

    let mut shadow_detail = graphics_settings.mapped(|settings| &settings.shadow_detail).new_remote();
    let mut framerate_limit = graphics_settings.mapped(|settings| &settings.frame_limit).new_remote();

    #[cfg(feature = "debug")]
    let render_settings = PlainTrackedState::new(RenderSettings::new());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("create render targets");

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
    let timer = korangar_debug::Timer::new("initialize interface");

    let mut application = InterfaceSettings::load_or_default();
    let mut interface = korangar_interface::Interface::new(swapchain_holder.window_screen_size());
    let mut focus_state = FocusState::default();
    let mut mouse_cursor = MouseCursor::new(&mut game_file_loader, &mut sprite_loader, &mut action_loader);
    let mut dialog_system = DialogSystem::default();
    let mut show_interface = true;

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("initialize timer");

    let mut game_timer = GameTimer::new();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = korangar_debug::Timer::new("initialize camera");

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
    let timer = korangar_debug::Timer::new("initialize networking");

    let client_info = load_client_info(&mut game_file_loader);
    let mut networking_system = NetworkingSystem::new();

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
    let welcome_message = ChatMessage::new(welcome_string, Color::monochrome_u8(255));
    let mut chat_messages = PlainTrackedState::new(vec![welcome_message]);

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
                let _measurement = korangar_debug::profiler_start_main_thread();

                #[cfg(feature = "debug")]
                let timer_measuremen = korangar_debug::start_measurement("update timers");

                input_system.update_delta();

                let delta_time = game_timer.update();
                let day_timer = game_timer.get_day_timer();
                let animation_timer = game_timer.get_animation_timer();
                let client_tick = game_timer.get_client_tick();

                #[cfg(feature = "debug")]
                timer_measuremen.stop();

                networking_system.keep_alive(delta_time, client_tick);
                let network_events = networking_system.network_events();

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
                let picker_measuremen = korangar_debug::start_measurement("update picker target");

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                    if let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                        if entity.are_details_unavailable() {
                            networking_system.request_entity_details(entity_id);
                            entity.set_details_requested();
                        }

                        match entity.get_entity_type() {
                            EntityType::Npc => mouse_cursor.set_state(MouseCursorState::Dialog, client_tick),
                            EntityType::Warp => mouse_cursor.set_state(MouseCursorState::Warp, client_tick),
                            EntityType::Monster => mouse_cursor.set_state(MouseCursorState::Attack, client_tick),
                            _ => {} // TODO: fill other entity types
                        }
                    }
                }

                #[cfg(feature = "debug")]
                picker_measuremen.stop();

                #[cfg(feature = "debug")]
                let network_event_measuremen = korangar_debug::start_measurement("process network events");

                for event in network_events {
                    match event {
                        NetworkEvent::AddEntity(entity_appeared_data) => {
                            // Sometimes (like after a job change) the server will tell the client
                            // that a new entity appeared, even though it was already on screen. So
                            // to prevent this we remove the old entity.
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
                                entity.move_from_to(&map, position_from, position_to, starting_timestamp);
                                /*#[cfg(feature = "debug")]
                                entity.generate_steps_vertex_buffer(device.clone(), &map);*/
                            }
                        }
                        NetworkEvent::PlayerMove(position_from, position_to, starting_timestamp) => {
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

                            entities[0].set_position(&map, player_position, client_tick);
                            player_camera.set_focus_point(entities[0].get_position());

                            particle_holder.clear();
                            effect_holder.clear();
                            networking_system.map_loaded();
                            // TODO: This is just a workaround until I find a better solution to make the
                            // cursor always look correct.
                            mouse_cursor.set_start_time(client_tick);
                        }
                        NetworkEvent::SetPlayerPosition(player_position) => {
                            entities[0].set_position(&map, player_position, client_tick);
                            player_camera.set_focus_point(entities[0].get_position());
                        }
                        NetworkEvent::UpdateClientTick(client_tick) => {
                            game_timer.set_client_tick(client_tick);
                        }
                        NetworkEvent::ChatMessage(message) => {
                            chat_messages.push(message);
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
                        NetworkEvent::Inventory(item_data) => {
                            player_inventory.fill(&mut game_file_loader, &mut texture_loader, &script_loader, item_data);
                        }
                        NetworkEvent::AddIventoryItem(item_index, item_data, equip_position, equipped_position) => {
                            player_inventory.add_item(
                                &mut game_file_loader,
                                &mut texture_loader,
                                &script_loader,
                                item_index,
                                item_data,
                                equip_position,
                                equipped_position,
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

                            entity.set_job(job_id as usize);
                            entity.reload_sprite(&mut game_file_loader, &mut sprite_loader, &mut action_loader, &script_loader);
                        }
                        NetworkEvent::Disconnect => {
                            networking_system.disconnect_from_map_server();
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

                            let character_selection_window = networking_system.character_selection_window();
                            interface.open_window(&application, &mut focus_state, &character_selection_window);

                            start_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
                            directional_shadow_camera.set_focus_point(cgmath::Point3::new(600.0, 0.0, 240.0));
                        }
                        NetworkEvent::FriendRequest(friend) => {
                            interface.open_window(&application, &mut focus_state, &FriendRequestWindow::new(friend))
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
                    }
                }

                #[cfg(feature = "debug")]
                network_event_measuremen.stop();

                #[cfg(feature = "debug")]
                let user_event_measuremen = korangar_debug::start_measurement("process user events");

                for event in user_events {
                    match event {
                        UserEvent::LogIn {
                            service_id,
                            username,
                            password,
                        } => {
                            match networking_system.log_in(&client_info, service_id, username, password) {
                                Ok(servers) => {
                                    // TODO: this will do one unnecessary restore_focus. check if
                                    // that will be problematic
                                    interface.close_window_with_class(&mut focus_state, LoginWindow::WINDOW_CLASS);

                                    interface.open_window(&application, &mut focus_state, &SelectServerWindow::new(servers));
                                }
                                Err(message) => interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message)),
                            }
                        }
                        UserEvent::SelectServer(server) => {
                            match networking_system.select_server(server) {
                                Ok(()) => {
                                    // TODO: this will do one unnecessary restore_focus. check if
                                    // that will be problematic
                                    interface.close_window_with_class(&mut focus_state, SelectServerWindow::WINDOW_CLASS);

                                    let character_selection_window = networking_system.character_selection_window();
                                    interface.open_window(&application, &mut focus_state, &character_selection_window);
                                }
                                Err(message) => interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message)),
                            }
                        }
                        UserEvent::LogOut => networking_system.log_out().unwrap(),
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
                                    &InventoryWindow::new(player_inventory.get_items()),
                                )
                            }
                        }
                        UserEvent::OpenEquipmentWindow => {
                            if !entities.is_empty() {
                                interface.open_window(
                                    &application,
                                    &mut focus_state,
                                    &EquipmentWindow::new(player_inventory.get_items()),
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
                            interface.open_window(&application, &mut focus_state, &networking_system.friends_window())
                        }
                        UserEvent::ToggleShowInterface => show_interface = !show_interface,
                        UserEvent::SetThemeFile { theme_file, theme_kind } => application.set_theme_file(theme_file, theme_kind),
                        UserEvent::SaveTheme { theme_kind } => application.save_theme(theme_kind),
                        UserEvent::ReloadTheme { theme_kind } => application.reload_theme(theme_kind),
                        UserEvent::SelectCharacter(character_slot) => {
                            match networking_system.select_character(character_slot) {
                                Ok((account_id, character_information, map_name)) => {
                                    map = map_loader
                                        .get(
                                            map_name,
                                            &mut game_file_loader,
                                            &mut buffer_allocator,
                                            &mut model_loader,
                                            &mut texture_loader,
                                        )
                                        .unwrap();

                                    let player = Player::new(
                                        &mut game_file_loader,
                                        &mut sprite_loader,
                                        &mut action_loader,
                                        &script_loader,
                                        &map,
                                        account_id,
                                        character_information,
                                        Vector2::zero(),
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

                                    particle_holder.clear();
                                    networking_system.map_loaded();
                                    // TODO: This is just a workaround until I find a better solution to make the
                                    // cursor always look correct.
                                    mouse_cursor.set_start_time(client_tick);
                                    game_timer.set_client_tick(client_tick);
                                }
                                Err(message) => interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message)),
                            }
                        }
                        UserEvent::OpenCharacterCreationWindow(character_slot) => {
                            interface.open_window(&application, &mut focus_state, &CharacterCreationWindow::new(character_slot))
                        }
                        UserEvent::CreateCharacter(character_slot, name) => {
                            match networking_system.create_character(character_slot, name) {
                                Ok(..) => interface.close_window_with_class(&mut focus_state, CharacterCreationWindow::WINDOW_CLASS),
                                Err(message) => interface.open_window(&application, &mut focus_state, &ErrorWindow::new(message)),
                            }
                        }
                        UserEvent::DeleteCharacter(character_id) => handle_result(
                            &application,
                            &mut interface,
                            &mut focus_state,
                            networking_system.delete_character(character_id),
                        ),
                        UserEvent::RequestSwitchCharacterSlot(origin_slot) => networking_system.request_switch_character_slot(origin_slot),
                        UserEvent::CancelSwitchCharacterSlot => networking_system.cancel_switch_character_slot(),
                        UserEvent::SwitchCharacterSlot(destination_slot) => handle_result(
                            &application,
                            &mut interface,
                            &mut focus_state,
                            networking_system.switch_character_slot(destination_slot),
                        ),
                        UserEvent::RequestPlayerMove(destination) => {
                            if !entities.is_empty() {
                                networking_system.request_player_move(destination)
                            }
                        }
                        UserEvent::RequestPlayerInteract(entity_id) => {
                            let entity = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                match entity.get_entity_type() {
                                    EntityType::Npc => networking_system.start_dialog(entity_id),
                                    EntityType::Monster => networking_system.request_player_attack(entity_id),
                                    EntityType::Warp => networking_system.request_player_move(entity.get_grid_position()),
                                    _ => {} // TODO: add other interactions
                                }
                            }
                        }
                        UserEvent::RequestWarpToMap(map_name, position) => networking_system.request_warp_to_map(map_name, position),
                        UserEvent::SendMessage(message) => {
                            networking_system.send_message(message);
                            // TODO: maybe find a better solution for unfocusing the message box if
                            // this becomes problematic
                            focus_state.remove_focus();
                        }
                        UserEvent::NextDialog(npc_id) => networking_system.next_dialog(npc_id),
                        UserEvent::CloseDialog(npc_id) => {
                            networking_system.close_dialog(npc_id);
                            dialog_system.close_dialog();
                            interface.close_window_with_class(&mut focus_state, DialogWindow::WINDOW_CLASS);
                        }
                        UserEvent::ChooseDialogOption(npc_id, option) => {
                            networking_system.choose_dialog_option(npc_id, option);

                            if option == -1 {
                                dialog_system.close_dialog();
                                interface.close_window_with_class(&mut focus_state, DialogWindow::WINDOW_CLASS);
                            }
                        }
                        UserEvent::MoveResource(r#move) => match r#move {
                            Move::Item { source, destination, item } => match (source, destination) {
                                (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                                    networking_system.request_item_equip(item.index, position);
                                }
                                (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                                    networking_system.request_item_unequip(item.index);
                                }
                                _ => {}
                            },
                            Move::Skill {
                                source,
                                destination,
                                skill,
                            } => match (source, destination) {
                                (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                                    hotbar.set_slot(skill, slot);
                                }
                                (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                                    hotbar.swap_slot(source_slot, destination_slot);
                                }
                                _ => {}
                            },
                        },
                        UserEvent::CastSkill(slot) => {
                            if let Some(skill) = hotbar.get_skill_in_slot(slot).as_ref() {
                                match skill.skill_type {
                                    SkillType::Passive => {}
                                    SkillType::Attack => {
                                        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                            networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                        }
                                    }
                                    SkillType::Ground | SkillType::Trap => {
                                        if let Some(PickerTarget::Tile { x, y }) = mouse_target {
                                            networking_system.cast_ground_skill(skill.skill_id, skill.skill_level, Vector2::new(x, y));
                                        }
                                    }
                                    SkillType::SelfCast => match skill.skill_id == ROLLING_CUTTER_ID {
                                        true => networking_system.cast_channeling_skill(
                                            skill.skill_id,
                                            skill.skill_level,
                                            entities[0].get_entity_id(),
                                        ),
                                        false => {
                                            networking_system.cast_skill(skill.skill_id, skill.skill_level, entities[0].get_entity_id())
                                        }
                                    },
                                    SkillType::Support => {
                                        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                            networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                        } else {
                                            networking_system.cast_skill(skill.skill_id, skill.skill_level, entities[0].get_entity_id());
                                        }
                                    }
                                }
                            }
                        }
                        UserEvent::StopSkill(slot) => {
                            if let Some(skill) = hotbar.get_skill_in_slot(slot).as_ref() {
                                if skill.skill_id == ROLLING_CUTTER_ID {
                                    networking_system.stop_channeling_skill(skill.skill_id);
                                }
                            }
                        }
                        UserEvent::AddFriend(name) => {
                            networking_system.add_friend(name);
                        }
                        UserEvent::RemoveFriend { account_id, character_id } => {
                            networking_system.remove_friend(account_id, character_id);
                        }
                        UserEvent::RejectFriendRequest { account_id, character_id } => {
                            networking_system.reject_friend_request(account_id, character_id);
                            interface.close_window_with_class(&mut focus_state, FriendRequestWindow::WINDOW_CLASS);
                        }
                        UserEvent::AcceptFriendRequest { account_id, character_id } => {
                            networking_system.accept_friend_request(account_id, character_id);
                            interface.close_window_with_class(&mut focus_state, FriendRequestWindow::WINDOW_CLASS);
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
                            interface.open_window(&application, &mut focus_state, &networking_system.packet_window())
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::ClearPacketHistory => networking_system.clear_packet_history(),
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
                user_event_measuremen.stop();

                let buffer_fence = buffer_allocator.submit_load_buffer();
                let texture_fence = texture_loader.submit_load_buffer();
                let sprite_fence = sprite_loader.submit_load_buffer();

                #[cfg(feature = "debug")]
                let update_entities_measuremen = korangar_debug::start_measurement("update entities");

                entities
                    .iter_mut()
                    .for_each(|entity| entity.update(&map, delta_time as f32, client_tick));

                #[cfg(feature = "debug")]
                update_entities_measuremen.stop();

                if !entities.is_empty() {
                    let player_position = entities[0].get_position();
                    player_camera.set_smoothed_focus_point(player_position);
                    directional_shadow_camera.set_focus_point(player_camera.get_focus_point());
                }

                #[cfg(feature = "debug")]
                let update_cameras_measuremen = korangar_debug::start_measurement("update cameras");

                start_camera.update(delta_time);
                player_camera.update(delta_time);
                directional_shadow_camera.update(day_timer);

                #[cfg(feature = "debug")]
                update_cameras_measuremen.stop();

                particle_holder.update(delta_time as f32);
                effect_holder.update(&entities, delta_time as f32);

                let (clear_interface, render_interface) = interface.update(&application, font_loader.clone(), &mut focus_state);
                mouse_cursor.update(client_tick);

                if swapchain_holder.is_swapchain_invalid() {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("re-create buffers");

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
                    korangar_debug::print_debug!(
                        "re-creating {}directional shadow targets{}",
                        korangar_debug::MAGENTA,
                        korangar_debug::NONE
                    );

                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("re-create shadow maps");

                    let new_shadow_detail = shadow_detail.get();

                    directional_shadow_targets = swapchain_holder
                        .get_swapchain_images()
                        .into_iter()
                        .map(|_| shadow_renderer.create_render_target(new_shadow_detail.into_resolution()))
                        .collect::<Vec<<ShadowRenderer as Renderer>::Target>>();
                }

                if framerate_limit.consume_changed() {
                    swapchain_holder.set_frame_limit(present_mode_info, framerate_limit.cloned());

                    // NOTE: For some reason the interface buffer becomes messed up when
                    // recreating the swapchain, so we need to render it again.
                    interface.schedule_render();
                }

                #[cfg(feature = "debug")]
                let matrices_measuremen = korangar_debug::start_measurement("generate view and projection matrices");

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
                matrices_measuremen.stop();

                let current_camera: &(dyn Camera + Send + Sync) = match entities.is_empty() {
                    #[cfg(feature = "debug")]
                    _ if render_settings.get().use_debug_camera => &debug_camera,
                    true => &start_camera,
                    false => &player_camera,
                };

                if let Some(mut fence) = screen_targets[swapchain_holder.get_image_number()].state.try_take_fence() {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("wait for frame in current slot");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = buffer_fence {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("wait for buffers");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = texture_fence {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("wait for textures");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = sprite_fence {
                    #[cfg(feature = "debug")]
                    korangar_debug::profile_block!("wait for sprites");

                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                #[cfg(feature = "debug")]
                let prepare_frame_measuremen = korangar_debug::start_measurement("prepare frame");

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
                prepare_frame_measuremen.stop();

                thread_pool.in_place_scope(|scope| {
                    scope.spawn(|_| {
                        #[cfg(feature = "debug")]
                        let _measurement = korangar_debug::profiler_start_picker_thread();

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
                        let _measurement = korangar_debug::profiler_start_shadow_thread();

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
                        let _measurement = korangar_debug::profiler_start_deferred_thread();

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
                        korangar_debug::profile_block!("render user interface");

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
                    korangar_debug::profile_block!("render hovered entity status");

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
                    korangar_debug::profile_block!("render player status");

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
                        Color::rgb_u8(200, 100, 50),
                        &application,
                    );
                }

                #[cfg(feature = "debug")]
                let finalize_frame_measuremen = korangar_debug::start_measurement("finalize frame");

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
                finalize_frame_measuremen.stop();
            }
            _ignored => (),
        }
    });
}
