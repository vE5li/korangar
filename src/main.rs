#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(option_zip)]
#![feature(adt_const_params)]
#![feature(arc_unwrap_or_clone)]
#![feature(proc_macro_hygiene)]
#![feature(negative_impls)]
#![feature(iter_intersperse)]
#![feature(auto_traits)]
#![feature(let_chains)]
#![feature(variant_count)]
#![feature(const_trait_impl)]
#![feature(decl_macro)]
#![feature(thread_local)]
#![feature(lazy_cell)]
#![feature(div_duration)]
#![feature(iter_next_chunk)]

#[cfg(feature = "debug")]
#[macro_use]
mod debug;
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

use cgmath::{Vector2, Zero};
use image::io::Reader as ImageReader;
use image::{EncodableLayout, ImageFormat};
use network::SkillType;
use procedural::debug_condition;
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
#[cfg(feature = "debug")]
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCreateInfo};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::sync::{now, GpuFuture};
use vulkano::VulkanLibrary;
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Icon, WindowBuilder};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::*;
use crate::input::{FocusState, InputSystem, UserEvent};
use crate::interface::*;
use crate::inventory::{Hotbar, Inventory, SkillTree};
use crate::loaders::*;
use crate::network::{ChatMessage, EntityId, NetworkEvent, NetworkingSystem, SkillId};
use crate::system::{choose_physical_device, get_device_extensions, get_instance_extensions, get_layers, GameTimer};
use crate::world::*;

const ROLLING_CUTTER_ID: SkillId = SkillId(2036);

fn main() {
    const DEFAULT_MAP: &str = "geffen";

    // We start a frame so that functions trying to start a measurement don't panic.
    #[cfg(feature = "debug")]
    let _measurement = profiler_start_main_thread();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let library = VulkanLibrary::new().unwrap();
    let create_info = InstanceCreateInfo {
        enabled_extensions: get_instance_extensions(&library),
        enabled_layers: get_layers(&library),
        enumerate_portability: true,
        ..Default::default()
    };

    let instance = Instance::new(library, create_info).expect("failed to create instance");

    #[cfg(feature = "debug")]
    let _debug_callback = unsafe {
        DebugUtilsMessenger::new(instance.clone(), DebugUtilsMessengerCreateInfo {
            message_severity: DebugUtilsMessageSeverity {
                error: true,
                warning: true,
                information: true,
                verbose: true,
                ..Default::default()
            },
            message_type: DebugUtilsMessageType {
                general: true,
                validation: true,
                performance: true,
                ..Default::default()
            },
            ..DebugUtilsMessengerCreateInfo::user_callback(Arc::new(vulkan_message_callback))
        })
        .ok()
    };

    #[cfg(feature = "debug")]
    print_debug!("created {}instance{}", MAGENTA, NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create window");

    // TODO: move this somewhere else
    let file_data = include_bytes!("../icon.png");

    let reader = ImageReader::with_format(Cursor::new(file_data), ImageFormat::Png);
    let image_buffer = reader.decode().unwrap().to_rgba8();
    let image_data = image_buffer.as_bytes().to_vec();

    assert_eq!(image_buffer.width(), image_buffer.height(), "icon must be square");
    let icon = Icon::from_rgba(image_data, image_buffer.width(), image_buffer.height()).unwrap();
    //

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title("Korangar".to_string())
        .with_window_icon(Some(icon))
        .build_vk_surface(&events_loop, instance.clone())
        .unwrap();

    surface
        .object()
        .unwrap()
        .downcast_ref::<winit::window::Window>()
        .unwrap()
        .set_cursor_visible(false);

    #[cfg(feature = "debug")]
    print_debug!("created {}window{}", MAGENTA, NONE);

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
    print_debug!("created {}vulkan device{}", MAGENTA, NONE);

    let queue = queues.next().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("received {}queue{} from {}device{}", MAGENTA, NONE, MAGENTA, NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resource managers");

    std::fs::create_dir_all("client/themes").unwrap();

    let mut game_file_loader = GameFileLoader::default();

    game_file_loader.add_archives_from_settings();

    // Patch precompiled lua files to lua 5.1 64 bit.
    game_file_loader.patch();

    // Load patched files to overwrite the original ones.
    game_file_loader.add_archive(LUA_GRF_FILE_NAME.to_string());

    let memory_allocator = Arc::new(MemoryAllocator::new(device.clone()));

    let font_loader = Rc::new(RefCell::new(FontLoader::new(
        memory_allocator.clone(),
        queue.clone(),
        &mut game_file_loader,
    )));

    let mut model_loader = ModelLoader::new(memory_allocator.clone());
    let mut texture_loader = TextureLoader::new(memory_allocator.clone(), queue.clone());
    let mut map_loader = MapLoader::new(memory_allocator.clone());
    let mut sprite_loader = SpriteLoader::new(memory_allocator.clone(), queue.clone());
    let mut action_loader = ActionLoader::default();
    let script_loader = ScriptLoader::new(&mut game_file_loader);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    let mut map = map_loader
        .get(
            DEFAULT_MAP.to_string(),
            &mut game_file_loader,
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
        queue.clone(),
        swapchain_holder.swapchain_format(),
        viewport.clone(),
        swapchain_holder.window_size_u32(),
        &mut game_file_loader,
        &mut texture_loader,
    );

    let mut interface_renderer = InterfaceRenderer::new(
        memory_allocator.clone(),
        queue.clone(),
        viewport.clone(),
        swapchain_holder.window_size_u32(),
        &mut game_file_loader,
        &mut texture_loader,
        font_loader.clone(),
    );

    let mut picker_renderer = PickerRenderer::new(
        memory_allocator.clone(),
        queue.clone(),
        viewport,
        swapchain_holder.window_size_u32(),
    );

    let shadow_renderer = ShadowRenderer::new(memory_allocator, queue, &mut game_file_loader, &mut texture_loader);

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
        .map(|_| shadow_renderer.create_render_target(8192))
        .collect::<Vec<<ShadowRenderer as Renderer>::Target>>();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize interface");

    let mut interface = Interface::new(
        &mut game_file_loader,
        &mut sprite_loader,
        &mut action_loader,
        swapchain_holder.window_size_f32(),
    );
    let mut focus_state = FocusState::default();
    let mut input_system = InputSystem::new();
    let mut graphics_settings = GraphicsSettings::new();
    #[cfg(feature = "debug")]
    let mut render_settings = RenderSettings::new();

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

    start_camera.set_focus_point(cgmath::Vector3::new(600.0, 0.0, 240.0));
    directional_shadow_camera.set_focus_point(cgmath::Vector3::new(600.0, 0.0, 240.0));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize networking");

    let mut networking_system = NetworkingSystem::new();

    interface.open_window(
        &mut focus_state,
        &LoginWindow::new(networking_system.get_login_settings().clone()),
    );

    #[cfg(feature = "debug")]
    timer.stop();

    let mut particle_holder = ParticleHolder::default();
    let mut entities = Vec::<Entity>::new();
    let mut player_inventory = Inventory::default();
    let mut player_skill_tree = SkillTree::default();
    let mut hotbar = Hotbar::default();

    let welcome_message = ChatMessage::new("Welcome to Korangar!".to_string(), Color::rgb(220, 170, 220));
    let chat_messages = Rc::new(RefCell::new(vec![welcome_message]));

    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(3).build().unwrap();

    events_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => control_flow.set_exit(),
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
                interface.update_window_size(Size::new(window_size.width as f32, window_size.height as f32));
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
            } => interface.hide_mouse_cursor(),
            Event::WindowEvent {
                event: WindowEvent::CursorEntered { .. },
                ..
            } => interface.show_mouse_cursor(),
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
                let _measurement = profiler_start_main_thread();

                #[cfg(feature = "debug")]
                let timer_measuremen = start_measurement("update timers");

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
                    &mut focus_state,
                    &mut picker_targets[swapchain_holder.get_image_number()],
                    #[cfg(feature = "debug")]
                    &render_settings,
                    swapchain_holder.window_size(),
                    client_tick,
                );

                #[cfg(feature = "debug")]
                let picker_measuremen = start_measurement("update picker target");

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                    if let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                        if entity.are_details_unavailable() {
                            networking_system.request_entity_details(entity_id);
                            entity.set_details_requested();
                        }

                        match entity.get_entity_type() {
                            EntityType::Npc => interface.set_mouse_cursor_state(MouseCursorState::Dialog, client_tick),
                            EntityType::Warp => interface.set_mouse_cursor_state(MouseCursorState::Warp, client_tick),
                            EntityType::Monster => interface.set_mouse_cursor_state(MouseCursorState::Attack, client_tick),
                            _ => {} // TODO: fill other entity types
                        }
                    }
                }

                #[cfg(feature = "debug")]
                picker_measuremen.stop();

                #[cfg(feature = "debug")]
                let network_event_measuremen = start_measurement("process network events");

                for event in network_events {
                    match event {
                        NetworkEvent::AddEntity(entity_appeared_data) => {
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
                            while entities.len() > 1 {
                                entities.pop();
                            }

                            map = map_loader
                                .get(map_name, &mut game_file_loader, &mut model_loader, &mut texture_loader)
                                .unwrap();

                            entities[0].set_position(&map, player_position, client_tick);

                            particle_holder.clear();
                            networking_system.map_loaded();
                            // TODO: this is just a workaround until i find a better solution to make the
                            // cursor always look correct.
                            interface.set_start_time(client_tick);
                        }
                        NetworkEvent::SetPlayerPosition(player_position) => {
                            entities[0].set_position(&map, player_position, client_tick);
                        }
                        NetworkEvent::UpdateClientTick(client_tick) => {
                            game_timer.set_client_tick(client_tick);
                        }
                        NetworkEvent::ChatMessage(message) => {
                            chat_messages.borrow_mut().push(message);
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
                        NetworkEvent::OpenDialog(text, npc_id) => interface.open_dialog_window(&mut focus_state, text, npc_id),
                        NetworkEvent::AddNextButton => interface.add_next_button(),
                        NetworkEvent::AddCloseButton => interface.add_close_button(),
                        NetworkEvent::AddChoiceButtons(choices) => interface.add_choice_buttons(choices),
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
                            let entity = entities.iter_mut().find(|entity| entity.with_account_id(account_id)).unwrap();

                            let Entity::Player(player) = entity else {
                                panic!();
                            };

                            player.set_job(job_id as usize);
                            player.reload_sprite(&mut game_file_loader, &mut sprite_loader, &mut action_loader, &script_loader);
                        }
                        NetworkEvent::Disconnect => {
                            networking_system.disconnect_from_map_server();
                            entities.clear();

                            map = map_loader
                                .get(
                                    DEFAULT_MAP.to_string(),
                                    &mut game_file_loader,
                                    &mut model_loader,
                                    &mut texture_loader,
                                )
                                .expect("failed to load initial map");

                            interface.close_all_windows(&mut focus_state);

                            let character_selection_window = networking_system.character_selection_window();
                            interface.open_window(&mut focus_state, &character_selection_window);

                            start_camera.set_focus_point(cgmath::Vector3::new(600.0, 0.0, 240.0));
                            directional_shadow_camera.set_focus_point(cgmath::Vector3::new(600.0, 0.0, 240.0));
                        }
                        NetworkEvent::FriendRequest(friend) => interface.open_window(&mut focus_state, &FriendRequestWindow::new(friend)),
                    }
                }

                #[cfg(feature = "debug")]
                network_event_measuremen.stop();

                #[cfg(feature = "debug")]
                let user_event_measuremen = start_measurement("process user events");

                for event in user_events {
                    match event {
                        UserEvent::LogIn(username, password) => match networking_system.log_in(username, password) {
                            Ok(()) => {
                                // TODO: this will do one unnecessary restore_focus. check if
                                // that will be problematic
                                interface.close_window_with_class(&mut focus_state, LoginWindow::WINDOW_CLASS);

                                let character_selection_window = networking_system.character_selection_window();
                                interface.open_window(&mut focus_state, &character_selection_window);
                            }
                            Err(message) => interface.open_window(&mut focus_state, &ErrorWindow::new(message)),
                        },
                        UserEvent::LogOut => networking_system.log_out().unwrap(),
                        UserEvent::Exit => *control_flow = ControlFlow::Exit,
                        UserEvent::ToggleRemeberUsername => networking_system.toggle_remember_username(),
                        UserEvent::ToggleRemeberPassword => networking_system.toggle_remember_password(),
                        UserEvent::CameraZoom(factor) => player_camera.soft_zoom(factor),
                        UserEvent::CameraRotate(factor) => player_camera.soft_rotate(factor),
                        UserEvent::ToggleFrameLimit => {
                            graphics_settings.toggle_frame_limit();
                            swapchain_holder.set_frame_limit(present_mode_info, graphics_settings.frame_limit);

                            // NOTE: For some reason the interface buffer becomes messed up when
                            // recreating the swapchain, so we need to render it again.
                            interface.schedule_rerender();
                        }
                        UserEvent::ToggleShowInterface => graphics_settings.toggle_show_interface(),
                        UserEvent::OpenMenuWindow => {
                            if !entities.is_empty() {
                                interface.open_window(&mut focus_state, &MenuWindow::default())
                            }
                        }
                        UserEvent::OpenInventoryWindow => {
                            if !entities.is_empty() {
                                interface.open_window(&mut focus_state, &InventoryWindow::new(player_inventory.get_items()))
                            }
                        }
                        UserEvent::OpenEquipmentWindow => {
                            if !entities.is_empty() {
                                interface.open_window(&mut focus_state, &EquipmentWindow::new(player_inventory.get_items()))
                            }
                        }
                        UserEvent::OpenSkillTreeWindow => {
                            if !entities.is_empty() {
                                interface.open_window(&mut focus_state, &SkillTreeWindow::new(player_skill_tree.get_skills()))
                            }
                        }
                        UserEvent::OpenGraphicsSettingsWindow => {
                            interface.open_window(&mut focus_state, &GraphicsSettingsWindow::new(present_mode_info))
                        }
                        UserEvent::OpenAudioSettingsWindow => interface.open_window(&mut focus_state, &AudioSettingsWindow::default()),
                        UserEvent::OpenFriendsWindow => interface.open_window(&mut focus_state, &networking_system.friends_window()),
                        UserEvent::SetThemeFile(theme_file) => {
                            interface.set_theme_file(theme_file);
                            interface.reload_theme();
                        }
                        UserEvent::SaveTheme => interface.save_theme(),
                        UserEvent::ReloadTheme => interface.reload_theme(),
                        UserEvent::SelectCharacter(character_slot) => {
                            match networking_system.select_character(character_slot) {
                                Ok((character_information, map_name)) => {
                                    map = map_loader
                                        .get(map_name, &mut game_file_loader, &mut model_loader, &mut texture_loader)
                                        .unwrap();

                                    let player = Player::new(
                                        &mut game_file_loader,
                                        &mut sprite_loader,
                                        &mut action_loader,
                                        &script_loader,
                                        &map,
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
                                    interface.open_window(&mut focus_state, &CharacterOverviewWindow::new());
                                    interface.open_window(&mut focus_state, &ChatWindow::new(chat_messages.clone(), font_loader.clone()));
                                    interface.open_window(&mut focus_state, &HotbarWindow::new(hotbar.get_skills()));

                                    particle_holder.clear();
                                    networking_system.map_loaded();
                                    // TODO: this is just a workaround until i find a better solution to make the
                                    // cursor always look correct.
                                    interface.set_start_time(client_tick);
                                    game_timer.set_client_tick(client_tick);
                                }
                                Err(message) => interface.open_window(&mut focus_state, &ErrorWindow::new(message)),
                            }
                        }
                        UserEvent::OpenCharacterCreationWindow(character_slot) => {
                            interface.open_window(&mut focus_state, &CharacterCreationWindow::new(character_slot))
                        }
                        UserEvent::CreateCharacter(character_slot, name) => {
                            match networking_system.create_character(character_slot, name) {
                                Ok(..) => interface.close_window_with_class(&mut focus_state, CharacterCreationWindow::WINDOW_CLASS),
                                Err(message) => interface.open_window(&mut focus_state, &ErrorWindow::new(message)),
                            }
                        }
                        UserEvent::DeleteCharacter(character_id) => {
                            interface.handle_result(&mut focus_state, networking_system.delete_character(character_id))
                        }
                        UserEvent::RequestSwitchCharacterSlot(origin_slot) => networking_system.request_switch_character_slot(origin_slot),
                        UserEvent::CancelSwitchCharacterSlot => networking_system.cancel_switch_character_slot(),
                        UserEvent::SwitchCharacterSlot(destination_slot) => {
                            interface.handle_result(&mut focus_state, networking_system.switch_character_slot(destination_slot))
                        }
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
                            interface.close_dialog_window(&mut focus_state);
                        }
                        UserEvent::ChooseDialogOption(npc_id, option) => {
                            networking_system.choose_dialog_option(npc_id, option);

                            if option == -1 {
                                interface.close_dialog_window(&mut focus_state);
                            }
                        }
                        UserEvent::MoveItem(item_move) => match (item_move.source, item_move.destination) {
                            (ItemSource::Inventory, ItemSource::Equipment { position }) => {
                                networking_system.request_item_equip(item_move.item.index, position);
                            }
                            (ItemSource::Equipment { .. }, ItemSource::Inventory) => {
                                networking_system.request_item_unequip(item_move.item.index);
                            }
                            _ => {}
                        },
                        UserEvent::MoveSkill(skill_move) => match (skill_move.source, skill_move.destination) {
                            (SkillSource::SkillTree, SkillSource::Hotbar { slot }) => {
                                hotbar.set_slot(skill_move.skill, slot);
                            }
                            (SkillSource::Hotbar { slot: source_slot }, SkillSource::Hotbar { slot: destination_slot }) => {
                                hotbar.swap_slot(source_slot, destination_slot);
                            }
                            _ => {}
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
                                        true => {
                                            networking_system.cast_channeling_skill(skill.skill_id, skill.skill_level, EntityId(2000009))
                                        }
                                        false => networking_system.cast_skill(skill.skill_id, skill.skill_level, EntityId(2000009)),
                                    },
                                    SkillType::Support => {
                                        if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                                            networking_system.cast_skill(skill.skill_id, skill.skill_level, entity_id);
                                        } else {
                                            networking_system.cast_skill(skill.skill_id, skill.skill_level, EntityId(2000009));
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
                        UserEvent::ToggleFrustumCulling => render_settings.toggle_frustum_culling(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowBoundingBoxes => render_settings.toggle_show_bounding_boxes(),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMarkerDetails(marker_identifier) => {
                            interface.open_window(&mut focus_state, map.resolve_marker(&entities, marker_identifier))
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::OpenRenderSettingsWindow => interface.open_window(&mut focus_state, &RenderSettingsWindow::default()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapDataWindow => interface.open_window(&mut focus_state, map.to_prototype_window()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapsWindow => interface.open_window(&mut focus_state, &MapsWindow::default()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenCommandsWindow => interface.open_window(&mut focus_state, &CommandsWindow::default()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenTimeWindow => interface.open_window(&mut focus_state, &TimeWindow::default()),
                        #[cfg(feature = "debug")]
                        UserEvent::SetDawn => game_timer.set_day_timer(0.0),
                        #[cfg(feature = "debug")]
                        UserEvent::SetNoon => game_timer.set_day_timer(std::f32::consts::FRAC_PI_2),
                        #[cfg(feature = "debug")]
                        UserEvent::SetDusk => game_timer.set_day_timer(std::f32::consts::PI),
                        #[cfg(feature = "debug")]
                        UserEvent::SetMidnight => game_timer.set_day_timer(-std::f32::consts::FRAC_PI_2),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenThemeViewerWindow => interface.open_theme_viewer_window(&mut focus_state),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenProfilerWindow => interface.open_window(&mut focus_state, &ProfilerWindow::new()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenPacketWindow => interface.open_window(&mut focus_state, &networking_system.packet_window()),
                        #[cfg(feature = "debug")]
                        UserEvent::ClearPacketHistory => networking_system.clear_packet_history(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleUseDebugCamera => render_settings.toggle_use_debug_camera(),
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
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowFramesPerSecond => render_settings.toggle_show_frames_per_second(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowWireframe => {
                            render_settings.toggle_show_wireframe();
                            swapchain_holder.invalidate_swapchain();

                            // NOTE: For some reason the interface buffer becomes messed up when
                            // recreating the swapchain, so we need to render it again.
                            interface.schedule_rerender();
                        }
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowMap => render_settings.toggle_show_map(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowObjects => render_settings.toggle_show_objects(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowEntities => render_settings.toggle_show_entities(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowWater => render_settings.toggle_show_water(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowIndicators => render_settings.toggle_show_indicators(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowAmbientLight => render_settings.toggle_show_ambient_light(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDirectionalLight => render_settings.toggle_show_directional_light(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowPointLights => render_settings.toggle_show_point_lights(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowParticleLights => render_settings.toggle_show_particle_lights(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDirectionalShadows => render_settings.toggle_show_directional_shadows(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowObjectMarkers => render_settings.toggle_show_object_markers(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowLightMarkers => render_settings.toggle_show_light_markers(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowSoundMarkers => render_settings.toggle_show_sound_markers(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowEffectMarkers => render_settings.toggle_show_effect_markers(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowParticleMarkers => render_settings.toggle_show_particle_markers(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowEntityMarkers => render_settings.toggle_show_entity_markers(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowMapTiles => render_settings.toggle_show_map_tiles(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowPathing => render_settings.toggle_show_pathing(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDiffuseBuffer => render_settings.toggle_show_diffuse_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowNormalBuffer => render_settings.toggle_show_normal_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowWaterBuffer => render_settings.toggle_show_water_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDepthBuffer => render_settings.toggle_show_depth_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowShadowBuffer => render_settings.toggle_show_shadow_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowPickerBuffer => render_settings.toggle_show_picker_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowFontAtlas => render_settings.toggle_show_font_atlas(),
                    }
                }

                #[cfg(feature = "debug")]
                user_event_measuremen.stop();

                let texture_fence = texture_loader.submit_load_buffer();
                let sprite_fence = sprite_loader.submit_load_buffer();

                particle_holder.update(delta_time as f32);

                #[cfg(feature = "debug")]
                let update_entities_measuremen = start_measurement("update entities");

                entities
                    .iter_mut()
                    .for_each(|entity| entity.update(&map, delta_time as f32, client_tick));

                #[cfg(feature = "debug")]
                update_entities_measuremen.stop();

                if !entities.is_empty() {
                    let player_position = entities[0].get_position();
                    player_camera.set_focus_point(player_position);
                    directional_shadow_camera.set_focus_point(player_position);
                }

                #[cfg(feature = "debug")]
                let update_cameras_measuremen = start_measurement("update cameras");

                start_camera.update(delta_time);
                player_camera.update(delta_time);
                directional_shadow_camera.update(day_timer);

                #[cfg(feature = "debug")]
                update_cameras_measuremen.stop();

                let (clear_interface, rerender_interface) = interface.update(font_loader.clone(), &mut focus_state, client_tick);

                if swapchain_holder.is_swapchain_invalid() {
                    #[cfg(feature = "debug")]
                    profile_block!("recreate buffers");

                    let viewport = swapchain_holder.recreate_swapchain();

                    deferred_renderer.recreate_pipeline(
                        viewport.clone(),
                        swapchain_holder.window_size_u32(),
                        #[cfg(feature = "debug")]
                        render_settings.show_wireframe,
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

                if let Some(mut fence) = screen_targets[swapchain_holder.get_image_number()].state.try_take_fence() {
                    #[cfg(feature = "debug")]
                    profile_block!("wait for frame in current slot");

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
                let matrices_measuremen = start_measurement("generate view and projection matrices");

                if entities.is_empty() {
                    start_camera.generate_view_projection(swapchain_holder.window_size());
                }

                player_camera.generate_view_projection(swapchain_holder.window_size());
                directional_shadow_camera.generate_view_projection(swapchain_holder.window_size());
                #[cfg(feature = "debug")]
                if render_settings.use_debug_camera {
                    debug_camera.generate_view_projection(swapchain_holder.window_size());
                }

                #[cfg(feature = "debug")]
                matrices_measuremen.stop();

                let current_camera: &(dyn Camera + Send + Sync) = match entities.is_empty() {
                    #[cfg(feature = "debug")]
                    _ if render_settings.use_debug_camera => &debug_camera,
                    true => &start_camera,
                    false => &player_camera,
                };

                #[cfg(feature = "debug")]
                let prepare_frame_measuremen = start_measurement("prepare frame");

                let walk_indicator_color = *interface.get_theme().indicator.walking;
                let image_number = swapchain_holder.get_image_number();
                let directional_shadow_image = directional_shadow_targets[image_number].image.clone();
                let screen_target = &mut screen_targets[image_number];
                let window_size = swapchain_holder.window_size_f32();
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
                        let _measurement = profiler_start_picker_thread();

                        let picker_target = &mut picker_targets[image_number];

                        picker_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_tiles(picker_target, &picker_renderer, current_camera);

                        #[debug_condition(render_settings.show_entities)]
                        map.render_entities(entities, picker_target, &picker_renderer, current_camera, false);

                        #[cfg(feature = "debug")]
                        map.render_markers(
                            picker_target,
                            &picker_renderer,
                            current_camera,
                            &render_settings,
                            entities,
                            hovered_marker_identifier,
                        );

                        picker_target.finish();
                    });

                    scope.spawn(|_| {
                        #[cfg(feature = "debug")]
                        let _measurement = profiler_start_shadow_thread();

                        let directional_shadow_target = &mut directional_shadow_targets[image_number];

                        directional_shadow_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_ground(
                            directional_shadow_target,
                            &shadow_renderer,
                            &directional_shadow_camera,
                            animation_timer,
                        );

                        #[debug_condition(render_settings.show_objects)]
                        map.render_objects(
                            directional_shadow_target,
                            &shadow_renderer,
                            &directional_shadow_camera,
                            client_tick,
                            animation_timer,
                            #[cfg(feature = "debug")]
                            render_settings.frustum_culling,
                        );

                        #[debug_condition(render_settings.show_entities)]
                        map.render_entities(
                            entities,
                            directional_shadow_target,
                            &shadow_renderer,
                            &directional_shadow_camera,
                            true,
                        );

                        if let Some(PickerTarget::Tile { x, y }) = mouse_target && !entities.is_empty() {
                            #[debug_condition(render_settings.show_indicators)]
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
                        let _measurement = profiler_start_deferred_thread();

                        screen_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_ground(screen_target, &deferred_renderer, current_camera, animation_timer);

                        #[cfg(feature = "debug")]
                        if render_settings.show_map_tiles {
                            map.render_overlay_tiles(screen_target, &deferred_renderer, current_camera);
                        }

                        #[debug_condition(render_settings.show_objects)]
                        map.render_objects(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            client_tick,
                            animation_timer,
                            #[cfg(feature = "debug")]
                            render_settings.frustum_culling,
                        );

                        #[debug_condition(render_settings.show_entities)]
                        map.render_entities(entities, screen_target, &deferred_renderer, current_camera, true);

                        #[debug_condition(render_settings.show_water)]
                        map.render_water(screen_target, &deferred_renderer, current_camera, animation_timer);

                        if let Some(PickerTarget::Tile { x, y }) = mouse_target && !entities.is_empty() {
                            #[debug_condition(render_settings.show_indicators)]
                            map.render_walk_indicator(
                                screen_target,
                                &deferred_renderer,
                                current_camera,
                                walk_indicator_color,
                                Vector2::new(x as usize, y as usize),
                            );
                        }

                        screen_target.lighting_pass();

                        #[debug_condition(render_settings.show_ambient_light && !render_settings.show_buffers())]
                        map.ambient_light(screen_target, &deferred_renderer, day_timer);

                        let (view_matrix, projection_matrix) = directional_shadow_camera.view_projection_matrices();
                        let light_matrix = projection_matrix * view_matrix;

                        #[debug_condition(render_settings.show_directional_light && !render_settings.show_buffers())]
                        map.directional_light(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            directional_shadow_image.clone(),
                            light_matrix,
                            day_timer,
                        );

                        #[debug_condition(render_settings.show_point_lights && !render_settings.show_buffers())]
                        map.point_lights(screen_target, &deferred_renderer, current_camera);

                        #[debug_condition(render_settings.show_water && !render_settings.show_buffers())]
                        map.water_light(screen_target, &deferred_renderer, current_camera);

                        #[cfg(feature = "debug")]
                        map.render_markers(
                            screen_target,
                            &deferred_renderer,
                            current_camera,
                            &render_settings,
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
                    });

                    if rerender_interface {
                        #[cfg(feature = "debug")]
                        profile_block!("render user interface");

                        interface_target.start(window_size_u32, clear_interface);

                        let state_provider = &StateProvider::new(
                            &graphics_settings,
                            #[cfg(feature = "debug")]
                            &render_settings,
                            networking_system.get_login_settings(),
                        );

                        interface.render(
                            &mut interface_target,
                            &interface_renderer,
                            state_provider,
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
                        &render_settings,
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
                            interface.get_theme(),
                            window_size,
                        );

                        if let Some(name) = &entity.get_details() {
                            let name = name.split('#').next().unwrap();
                            interface.render_hover_text(screen_target, &deferred_renderer, name, input_system.get_mouse_position());
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
                        interface.get_theme(),
                        window_size,
                    );
                }

                #[cfg(feature = "debug")]
                if render_settings.show_frames_per_second {
                    interface.render_frames_per_second(screen_target, &deferred_renderer, game_timer.last_frames_per_second());
                }

                if graphics_settings.show_interface {
                    deferred_renderer.overlay_interface(screen_target, interface_target.image.clone());

                    interface.render_mouse_cursor(
                        screen_target,
                        &deferred_renderer,
                        input_system.get_mouse_position(),
                        input_system.get_mouse_mode().grabbed(),
                    );
                }

                #[cfg(feature = "debug")]
                let finalize_frame_measuremen = start_measurement("finalize frame");

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
