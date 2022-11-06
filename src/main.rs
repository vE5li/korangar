#![allow(incomplete_features)]
#![allow(clippy::too_many_arguments)]
#![feature(unzip_option)]
#![feature(option_zip)]
#![feature(adt_const_params)]
#![feature(arc_unwrap_or_clone)]
#![feature(option_result_contains)]
#![feature(proc_macro_hygiene)]
#![feature(negative_impls)]
#![feature(iter_intersperse)]
#![feature(auto_traits)]
#![feature(let_chains)]
#![feature(variant_count)]

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
use std::rc::Rc;
use std::sync::Arc;

use chrono::prelude::*;
use procedural::debug_condition;
use vulkano::device::{Device, DeviceCreateInfo, QueueCreateInfo};
use vulkano::instance::debug::{DebugUtilsMessageSeverity, DebugUtilsMessageType, DebugUtilsMessenger, DebugUtilsMessengerCreateInfo};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::sync::{now, GpuFuture};
use vulkano::VulkanLibrary;
use vulkano_win::VkSurfaceBuild;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::*;
use crate::input::{FocusState, InputSystem, MouseInputMode, UserEvent};
use crate::interface::*;
use crate::inventory::Inventory;
use crate::loaders::*;
use crate::network::{ChatMessage, NetworkEvent, NetworkingSystem};
use crate::system::{get_device_extensions, get_instance_extensions, get_layers, GameTimer};
use crate::world::*;

fn main() {
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

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new()
        .with_title("korangar".to_string())
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
    let (physical_device, queue_family_index) = choose_physical_device!(&instance, &surface, &desired_device_extensions);

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

    game_file_loader.add_archive("data.grf".to_string());
    game_file_loader.add_archive("rdata.grf".to_string());
    game_file_loader.add_archive("korangar.grf".to_string());

    // patch precompiled lua files to lua 5.1 64 bit
    game_file_loader.patch();
    game_file_loader.add_archive("lua_files.grf".to_string());

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
            "pay_dun00".to_string(),
            &mut game_file_loader,
            &mut model_loader,
            &mut texture_loader,
        )
        .expect("failed to load initial map");

    // interesting: ma_zif07, ama_dun01

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

    let shadow_renderer = ShadowRenderer::new(memory_allocator, queue);

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
    let mut player_camera = PlayerCamera::new();
    let mut directional_shadow_camera = ShadowCamera::new();

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

    // move
    const TIME_FACTOR: f32 = 1000.0;
    let local: DateTime<Local> = Local::now();
    let mut day_timer = (local.hour() as f32 / TIME_FACTOR * 60.0 * 60.0) + (local.minute() as f32 / TIME_FACTOR * 60.0);
    let mut animation_timer = 0.0;
    //

    let welcome_message = ChatMessage::new("Welcome to Korangar!".to_string(), Color::rgb(220, 170, 220));
    let chat_messages = Rc::new(RefCell::new(vec![welcome_message]));

    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(3).build().unwrap();

    events_loop.run(move |event, _, control_flow| {
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
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
            } => input_system.update_keyboard(input.virtual_keycode.unwrap(), input.state),
            Event::WindowEvent {
                event: WindowEvent::ReceivedCharacter(character),
                ..
            } => input_system.buffer_character(character),
            Event::RedrawEventsCleared => {
                input_system.update_delta();

                let delta_time = game_timer.update();
                let client_tick = game_timer.get_client_tick();

                day_timer += delta_time as f32 / TIME_FACTOR;
                animation_timer += delta_time as f32;

                networking_system.keep_alive(delta_time, client_tick);

                let network_events = networking_system.network_events();
                let (user_events, hovered_element, focused_element, mouse_target) = input_system.user_events(
                    &mut interface,
                    &mut focus_state,
                    &mut picker_targets[swapchain_holder.get_image_number()],
                    &render_settings,
                    swapchain_holder.window_size(),
                    client_tick,
                );

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {
                    if let Some(entity) = entities.iter_mut().find(|entity| entity.get_entity_id() == entity_id) {
                        if entity.are_details_unavalible() {
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
                                game_timer.get_client_tick(),
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

                            entities[0].set_position(&map, player_position, game_timer.get_client_tick());

                            particle_holder.clear();
                            networking_system.map_loaded();
                            // TODO: this is just a workaround until i find a better solution to make the
                            // cursor always look correct.
                            interface.set_start_time(game_timer.get_client_tick());
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
                        NetworkEvent::UpdateEquippedPosition { index, equipped_position } => {
                            player_inventory.update_equipped_position(index, equipped_position);
                        }
                    }
                }

                for event in user_events {
                    match event {
                        UserEvent::LogIn(username, password) => match networking_system.log_in(username, password) {
                            Ok(character_selection_window) => {
                                // TODO: this will do one unnecessary restore_focus. check if
                                // that will be problematic
                                interface.close_window_with_class(&mut focus_state, LoginWindow::WINDOW_CLASS);
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
                            render_settings.toggle_frame_limit();
                            swapchain_holder.set_frame_limit(render_settings.frame_limit);

                            // for some reason the interface buffer becomes messed up when
                            // recreating the swapchain, so we need to render it again
                            interface.schedule_rerender();
                        }
                        UserEvent::OpenMenuWindow => interface.open_window(&mut focus_state, &MenuWindow::default()),
                        UserEvent::OpenInventoryWindow => {
                            interface.open_window(&mut focus_state, &InventoryWindow::new(player_inventory.get_item_state()))
                        }
                        UserEvent::OpenEquipmentWindow => {
                            interface.open_window(&mut focus_state, &EquipmentWindow::new(player_inventory.get_item_state()))
                        }
                        UserEvent::OpenGraphicsSettingsWindow => {
                            interface.open_window(&mut focus_state, &GraphicsSettingsWindow::default())
                        }
                        UserEvent::OpenAudioSettingsWindow => interface.open_window(&mut focus_state, &AudioSettingsWindow::default()),
                        UserEvent::ReloadTheme => interface.reload_theme(),
                        UserEvent::SaveTheme => interface.save_theme(),
                        UserEvent::SelectCharacter(character_slot) => {
                            match networking_system.select_character(character_slot, &chat_messages) {
                                Ok((map_name, player_position, character_information, client_tick)) => {
                                    // TODO: this will do one unnecessary restore_focus. check if
                                    // that will be problematic
                                    interface.close_window_with_class(&mut focus_state, CharacterSelectionWindow::WINDOW_CLASS);
                                    interface.open_window(&mut focus_state, &CharacterOverviewWindow::new());
                                    interface.open_window(&mut focus_state, &PrototypeChatWindow::new(chat_messages.clone()));

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
                                        player_position,
                                        client_tick,
                                    );
                                    let player = Entity::Player(player);

                                    player_camera.set_focus_point(player.get_position());
                                    entities.push(player);

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
                        UserEvent::RequestPlayerMove(destination) => networking_system.request_player_move(destination),
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
                        UserEvent::OpenTimeWindow => interface.open_window(&mut focus_state, &TimeWindow::default()),
                        #[cfg(feature = "debug")]
                        UserEvent::SetDawn => day_timer = 0.0,
                        #[cfg(feature = "debug")]
                        UserEvent::SetNoon => day_timer = std::f32::consts::FRAC_PI_2,
                        #[cfg(feature = "debug")]
                        UserEvent::SetDusk => day_timer = std::f32::consts::PI,
                        #[cfg(feature = "debug")]
                        UserEvent::SetMidnight => day_timer = -std::f32::consts::FRAC_PI_2,
                        #[cfg(feature = "debug")]
                        UserEvent::OpenThemeViewerWindow => interface.open_theme_viewer_window(&mut focus_state),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenProfilerWindow => interface.open_window(&mut focus_state, &ProfilerWindow::default()),
                        #[cfg(feature = "debug_network")]
                        UserEvent::OpenPacketWindow => {
                            interface.open_window(&mut focus_state, &PacketWindow::new(networking_system.packets()))
                        }
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

                            // for some reason the interface buffer becomes messed up when
                            // recreating the swapchain, so we need to render it again
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

                let texture_fence = texture_loader.submit_load_buffer();
                let sprite_fence = sprite_loader.submit_load_buffer();

                particle_holder.update(delta_time as f32);

                entities
                    .iter_mut()
                    .for_each(|entity| entity.update(&map, delta_time as f32, game_timer.get_client_tick()));

                if !entities.is_empty() {
                    let player_position = entities[0].get_position();
                    player_camera.set_focus_point(player_position);
                    directional_shadow_camera.set_focus_point(player_position);
                }

                player_camera.update(delta_time);
                directional_shadow_camera.update(day_timer);

                let (clear_interface, rerender_interface) = interface.update(&mut focus_state, game_timer.get_client_tick());

                if swapchain_holder.is_swapchain_invalid() {
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
                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                #[cfg(feature = "debug")]
                let wait_for_previous = rerender_interface || render_settings.show_buffers();

                #[cfg(not(feature = "debug"))]
                let wait_for_previous = rerender_interface;

                if wait_for_previous {
                    for screen_target in &mut screen_targets {
                        if let Some(mut fence) = screen_target.state.try_take_fence() {
                            fence.wait(None).unwrap();
                            fence.cleanup_finished();
                        }
                    }
                }

                if let Some(mut fence) = texture_fence {
                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                if let Some(mut fence) = sprite_fence {
                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                player_camera.generate_view_projection(swapchain_holder.window_size());
                directional_shadow_camera.generate_view_projection(swapchain_holder.window_size());
                #[cfg(feature = "debug")]
                if render_settings.use_debug_camera {
                    debug_camera.generate_view_projection(swapchain_holder.window_size());
                }

                #[cfg(feature = "debug")]
                let current_camera: &(dyn Camera + Send + Sync) = match render_settings.use_debug_camera {
                    true => &debug_camera,
                    false => &player_camera,
                };

                #[cfg(not(feature = "debug"))]
                let current_camera: &(dyn Camera + Send + Sync) = &player_camera;

                let image_number = swapchain_holder.get_image_number();
                let client_tick = game_timer.get_client_tick();
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

                thread_pool.in_place_scope(|scope| {
                    scope.spawn(|_| {
                        let picker_target = &mut picker_targets[image_number];

                        picker_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_tiles(picker_target, &picker_renderer, current_camera);

                        #[debug_condition(render_settings.show_entities)]
                        entities
                            .iter()
                            .skip(1)
                            .for_each(|entity| entity.render(picker_target, &picker_renderer, current_camera));

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
                        entities
                            .iter()
                            .for_each(|entity| entity.render(directional_shadow_target, &shadow_renderer, &directional_shadow_camera));

                        directional_shadow_target.finish();
                    });

                    scope.spawn(|_| {
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
                        entities
                            .iter()
                            .for_each(|entity| entity.render(screen_target, &deferred_renderer, current_camera));

                        #[debug_condition(render_settings.show_water)]
                        map.render_water(screen_target, &deferred_renderer, current_camera, animation_timer);

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
                        interface_target.start(window_size_u32, clear_interface);

                        let state_provider = &StateProvider::new(&render_settings, networking_system.get_login_settings());
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
                    let entity = entities.iter().find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.render_status(screen_target, &deferred_renderer, current_camera, window_size);

                        if let Some(name) = &entity.get_details() {
                            let name = name.split('#').next().unwrap();
                            interface.render_hover_text(screen_target, &deferred_renderer, name, input_system.get_mouse_position());
                        }

                        entity.render_status(screen_target, &deferred_renderer, current_camera, window_size);
                    }
                }

                if !entities.is_empty() {
                    entities[0].render_status(screen_target, &deferred_renderer, current_camera, window_size);
                }

                if render_settings.show_frames_per_second {
                    interface.render_frames_per_second(screen_target, &deferred_renderer, game_timer.last_frames_per_second());
                }

                if render_settings.show_interface {
                    deferred_renderer.overlay_interface(screen_target, interface_target.image.clone());

                    let grabbed_item = match input_system.get_mouse_mode() {
                        MouseInputMode::MoveItem(_, item) => Some(item.texture.clone()),
                        _ => None,
                    };

                    interface.render_mouse_cursor(
                        screen_target,
                        &deferred_renderer,
                        input_system.get_mouse_position(),
                        grabbed_item,
                    );
                }

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
            }
            _ignored => (),
        }
    });
}
