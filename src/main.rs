#![feature(unzip_option)]
#![feature(option_zip)]
#![feature(let_else)]
#![feature(adt_const_params)]
#![feature(arc_unwrap_or_clone)]
#![feature(option_result_contains)]
#![feature(proc_macro_hygiene)]

#[cfg(feature = "debug")]
#[macro_use]
mod debug;
#[macro_use]
mod types;
mod traits;
mod input;
#[macro_use]
mod system;
mod graphics;
mod loaders;
mod interface;
mod network;
mod database;

use procedural::debug_condition;
use std::cell::RefCell;
use std::rc::Rc;
use vulkano::device::Device;
use vulkano::instance::Instance;
#[cfg(feature = "debug")]
use vulkano::instance::debug::{ MessageSeverity, MessageType };
use vulkano::Version;
use vulkano::sync::{ GpuFuture, now };
use vulkano_win::VkSurfaceBuild;
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;
use database::Database;
use chrono::{prelude::*, offset};

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::*;
use crate::input::{ InputSystem, UserEvent };
use crate::system::{ GameTimer, get_instance_extensions, get_layers, get_device_extensions };
use crate::loaders::*;
use crate::graphics::*;
use crate::interface::*;
use crate::network::{ NetworkingSystem, NetworkEvent };

fn main() {

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let instance = Instance::new(None, Version::V1_2, &get_instance_extensions(), get_layers()).expect("failed to create instance");

    #[cfg(feature = "debug")]
    let _debug_callback = vulkano::instance::debug::DebugCallback::new(&instance, MessageSeverity::all(), MessageType::all(), vulkan_message_callback).ok();

    #[cfg(feature = "debug")]
    print_debug!("created {}instance{}", MAGENTA, NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create window");

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new().with_title(String::from("korangar")).build_vk_surface(&events_loop, instance.clone()).unwrap();

    surface.window().set_cursor_visible(false);

    #[cfg(feature = "debug")]
    print_debug!("created {}window{}", MAGENTA, NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("choose physical device");

    let desired_device_extensions = get_device_extensions();
    let (physical_device, queue_family) = choose_physical_device!(&instance, surface, &desired_device_extensions);
    let required_device_extensions = physical_device.required_extensions().union(&desired_device_extensions);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let (device, mut queues) = Device::new(physical_device, physical_device.supported_features(), &required_device_extensions, [(queue_family, 0.5)].iter().cloned()).expect("failed to create device");

    #[cfg(feature = "debug")]
    print_debug!("created {}vulkan device{}", MAGENTA, NONE);

    let queue = queues.next().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("received {}queue{} from {}device{}", MAGENTA, NONE, MAGENTA, NONE);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resource managers");

    let mut game_file_loader = GameFileLoader::default();

    game_file_loader.add_archive("data3.grf".to_string());
    game_file_loader.add_archive("data.grf".to_string());
    game_file_loader.add_archive("rdata.grf".to_string());

    let game_file_loader = Rc::new(RefCell::new(game_file_loader));
    let font_loader = Rc::new(RefCell::new(FontLoader::new(device.clone(), queue.clone())));

    let mut model_loader = ModelLoader::new(game_file_loader.clone(), device.clone());
    let mut texture_loader = TextureLoader::new(game_file_loader.clone(), device.clone(), queue.clone());
    let mut map_loader = MapLoader::new(game_file_loader.clone(), device.clone());
    let mut sprite_loader = SpriteLoader::new(game_file_loader.clone(), device.clone(), queue.clone());
    let mut action_loader = ActionLoader::new(game_file_loader.clone());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    let mut map = map_loader.get(&mut model_loader, &mut texture_loader, "pay_dun00.rsw").expect("failed to load initial map");

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

    let mut deferred_renderer = DeferredRenderer::new(device.clone(), queue.clone(), swapchain_holder.swapchain_format(), viewport.clone(), swapchain_holder.window_size_u32(), &mut texture_loader);
    let mut interface_renderer = InterfaceRenderer::new(device.clone(), queue.clone(), viewport.clone(), swapchain_holder.window_size_u32(), &mut texture_loader);
    let mut picker_renderer = PickerRenderer::new(device.clone(), queue.clone(), viewport.clone(), swapchain_holder.window_size_u32());
    let mut shadow_renderer = ShadowRenderer::new(device.clone(), queue.clone());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create render targets");

    let mut screen_targets = swapchain_holder.get_swapchain_images()
        .into_iter()
        .map(|swapchain_image| deferred_renderer.create_render_target(swapchain_image))
        .collect::<Vec<<DeferredRenderer as Renderer>::Target>>();

    let mut interface_target = interface_renderer.create_render_target();

    let mut picker_targets = swapchain_holder.get_swapchain_images()
        .into_iter()
        .map(|_| picker_renderer.create_render_target())
        .collect::<Vec<<PickerRenderer as Renderer>::Target>>();

    let mut directional_shadow_targets = swapchain_holder.get_swapchain_images()
        .into_iter()
        .map(|_| shadow_renderer.create_render_target(8192))
        .collect::<Vec<<ShadowRenderer as Renderer>::Target>>();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize interface");

    let mut texture_future = now(device.clone()).boxed();

    let mut interface = Interface::new(&mut sprite_loader, &mut action_loader, &mut texture_future, swapchain_holder.window_size_f32());
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

    match networking_system.log_in() {
        Ok(character_selection_window) => interface.open_window(&character_selection_window),
        Err(message) => interface.open_window(&ErrorWindow::new(message)),
    }

    #[cfg(feature = "debug")]
    timer.stop();

    let mut particle_holder = ParticleHolder::default();
    let mut entities = Vec::<Entity>::new();

    // move
    const TIME_FACTOR: f32 = 1000.0;
    let local: DateTime<Local> = Local::now();
    let mut day_timer = (local.hour() as f32 / TIME_FACTOR * 60.0 * 60.0) + (local.minute() as f32 / TIME_FACTOR * 60.0);
    //

    let welcome_message = ChatMessage::new("Welcome to Korangar!".to_string(), Color::rgb(220, 170, 220));
    let chat_messages = Rc::new(RefCell::new(vec![welcome_message]));
    let database = Database::new();

    let thread_pool = rayon::ThreadPoolBuilder::new().num_threads(3).build().unwrap();

    texture_future
        .then_signal_fence_and_flush()
        .unwrap()
        .wait(None)
        .unwrap();

    events_loop.run(move |event, _, control_flow| {
        match event {

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                let window_size = surface.window().inner_size();
                interface.update_window_size(Size::new(window_size.width as f32, window_size.height as f32));
                swapchain_holder.update_window_size(window_size.into());
            }

            Event::WindowEvent { event: WindowEvent::Focused(focused), .. } => {
                if !focused {
                    input_system.reset();
                }
            }

            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                input_system.update_mouse_position(position);
            }

            Event::WindowEvent { event: WindowEvent::MouseInput{ button, state, .. }, .. } => {
                input_system.update_mouse_buttons(button, state);
            }

            Event::WindowEvent { event: WindowEvent::MouseWheel{ delta, .. }, .. } => {
                input_system.update_mouse_wheel(delta);
            }

            Event::WindowEvent { event: WindowEvent::KeyboardInput{ input, .. }, .. } => {
                input_system.update_keyboard(input.scancode as usize, input.state);
            }

            Event::RedrawEventsCleared => {

                input_system.update_delta();

                let delta_time = game_timer.update();

                day_timer += delta_time as f32 / TIME_FACTOR;

                networking_system.keep_alive(delta_time, game_timer.get_client_tick());

                let network_events = networking_system.network_events();
                let (user_events, hovered_element, mouse_target) = input_system.user_events(&mut interface, &mut picker_targets[swapchain_holder.get_image_number()], &render_settings, swapchain_holder.window_size());

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {

                    let entity = entities
                        .iter_mut()
                        .find(|entity| entity.get_entity_id() == entity_id)
                        .unwrap();

                    if entity.are_details_unavalible() {
                        networking_system.request_entity_details(entity_id);
                        entity.set_details_requested();
                    }
                }

                let mut texture_future = now(device.clone()).boxed();

                for event in network_events {
                    match event {

                        NetworkEvent::AddEntity(entity_appeared_data) => {
                            let npc = Npc::new(&mut sprite_loader, &mut action_loader, &mut texture_future, &map, &database, entity_appeared_data);
                            let npc = Entity::Npc(npc);
                            entities.push(npc);
                        }

                        NetworkEvent::RemoveEntity(entity_id) => {
                            entities.retain(|entity| entity.get_entity_id() != entity_id);
                        }

                        NetworkEvent::EntityMove(entity_id, position_from, position_to, starting_timestamp) => {
                            let entity = entities
                                .iter_mut()
                                .find(|entity| entity.get_entity_id() == entity_id);


                            if let Some(entity) = entity {
                                entity.move_from_to(&map, position_from, position_to, starting_timestamp);
                                #[cfg(feature = "debug")]
                                entity.generate_steps_vertex_buffer(device.clone(), &map);
                            }
                        }

                        NetworkEvent::PlayerMove(position_from, position_to, starting_timestamp) => {
                            entities[0].move_from_to(&map, position_from, position_to, starting_timestamp);
                            #[cfg(feature = "debug")]
                            entities[0].generate_steps_vertex_buffer(device.clone(), &map);
                        }

                        NetworkEvent::ChangeMap(map_name, player_position) => {

                            while entities.len() > 1 {
                                entities.pop();
                            }

                            map = map_loader.get(&mut model_loader, &mut texture_loader, &format!("{}.rsw", map_name)).unwrap();

                            entities[0].set_position(&map, player_position);

                            networking_system.map_loaded();
                        }

                        NetworkEvent::UpdateClientTick(client_tick) => {
                            game_timer.set_client_tick(client_tick);
                        }

                        NetworkEvent::ChatMessage(message) => {
                            chat_messages.borrow_mut().push(message);
                        }

                        NetworkEvent::UpdateEntityDetails(entity_id, name) => {
                            let entity = entities
                                .iter_mut()
                                .find(|entity| entity.get_entity_id() == entity_id);

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

                            let entity = entities
                                .iter_mut()
                                .find(|entity| entity.get_entity_id() == entity_id);

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

                        NetworkEvent::OpenDialog(text, npc_id) => interface.open_dialog_window(text, npc_id),

                        NetworkEvent::AddNextButton => interface.add_next_button(),

                        NetworkEvent::AddCloseButton => interface.add_close_button(),

                        NetworkEvent::AddChoiceButtons(choices) => interface.add_choice_buttons(choices),

                        NetworkEvent::Inventory(inventory) => {
                            for stack in inventory {
                                println!("item: {}", database.itme_name_from_id(stack));
                            }
                        }
                    }
                }

                for event in user_events {
                    match event {

                        UserEvent::Exit => *control_flow = ControlFlow::Exit,

                        UserEvent::LogOut => networking_system.log_out().unwrap(),

                        UserEvent::CameraZoom(factor) => player_camera.soft_zoom(factor),

                        UserEvent::CameraRotate(factor) => player_camera.soft_rotate(factor),

                        UserEvent::ToggleFrameLimit => {
                            render_settings.toggle_frame_limit();
                            swapchain_holder.set_frame_limit(render_settings.frame_limit);
                        },

                        UserEvent::OpenMenuWindow => interface.open_window(&MenuWindow::default()),

                        UserEvent::OpenGraphicsSettingsWindow => interface.open_window(&GraphicsSettingsWindow::default()),

                        UserEvent::OpenAudioSettingsWindow => interface.open_window(&AudioSettingsWindow::default()),

                        UserEvent::ReloadTheme => interface.reload_theme(),

                        UserEvent::SaveTheme => interface.save_theme(),

                        UserEvent::SelectCharacter(character_slot) => {
                            match networking_system.select_character(character_slot, &chat_messages) {

                                Ok((map_name, player_position, character_information, client_tick)) => {

                                    interface.close_window_with_class(CharacterSelectionWindow::WINDOW_CLASS);
                                    interface.open_window(&PrototypeChatWindow::new(chat_messages.clone()));

                                    map = map_loader.get(&mut model_loader, &mut texture_loader, &format!("{}.rsw", map_name)).unwrap();

                                    let player = Player::new(&mut sprite_loader, &mut action_loader, &mut texture_future, &map, &database, character_information, player_position);
                                    let player = Entity::Player(player);

                                    player_camera.set_focus_point(player.get_position());
                                    entities.push(player);

                                    networking_system.map_loaded();
                                    game_timer.set_client_tick(client_tick);
                                }

                                Err(message) => interface.open_window(&ErrorWindow::new(message)),
                            }
                        },

                        UserEvent::CreateCharacter(character_slot) => interface.handle_result(networking_system.crate_character(character_slot)),

                        UserEvent::DeleteCharacter(character_id) => interface.handle_result(networking_system.delete_character(character_id)),

                        UserEvent::RequestSwitchCharacterSlot(origin_slot) => networking_system.request_switch_character_slot(origin_slot),

                        UserEvent::CancelSwitchCharacterSlot => networking_system.cancel_switch_character_slot(),

                        UserEvent::SwitchCharacterSlot(destination_slot) => interface.handle_result(networking_system.switch_character_slot(destination_slot)),

                        UserEvent::RequestPlayerMove(destination) => networking_system.request_player_move(destination),

                        UserEvent::RequestPlayerInteract(entity_id) => {

                            let entity = entities
                                .iter_mut()
                                .find(|entity| entity.get_entity_id() == entity_id);

                            if let Some(entity) = entity {
                                let job = entity.get_job();

                                match job > 1002 && job < 1010 {
                                    true => networking_system.request_player_attack(entity_id),
                                    false => networking_system.start_dialog(entity_id),
                                }
                            }
                        },

                        UserEvent::RequestWarpToMap(map_name, position) => networking_system.request_warp_to_map(map_name, position),

                        UserEvent::NextDialog(npc_id) => networking_system.next_dialog(npc_id),

                        UserEvent::CloseDialog(npc_id) => {
                            networking_system.close_dialog(npc_id);
                            interface.close_dialog_window();
                        },

                        UserEvent::ChooseDialogOption(npc_id, option) => networking_system.choose_dialog_option(npc_id, option),

                        #[cfg(feature = "debug")]
                        UserEvent::OpenRenderSettingsWindow => interface.open_window(&RenderSettingsWindow::default()),

                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapDataWindow => interface.open_window(map.to_prototype_window()),

                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapsWindow => interface.open_window(&MapsWindow::default()),

                        #[cfg(feature = "debug")]
                        UserEvent::OpenTimeWindow => interface.open_window(&TimeWindow::default()),

                        #[cfg(feature = "debug")]
                        UserEvent::SetDawn => day_timer = 0.0,

                        #[cfg(feature = "debug")]
                        UserEvent::SetNoon => day_timer = std::f32::consts::FRAC_PI_2,

                        #[cfg(feature = "debug")]
                        UserEvent::SetDusk => day_timer = std::f32::consts::PI,

                        #[cfg(feature = "debug")]
                        UserEvent::SetMidnight => day_timer = -std::f32::consts::FRAC_PI_2,

                        #[cfg(feature = "debug")]
                        UserEvent::OpenThemeViewerWindow => interface.open_theme_viewer_window(),

                        #[cfg(feature = "debug")]
                        UserEvent::OpenProfilerWindow => interface.open_window(&ProfilerWindow::default()),

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
                        },

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

                let texture_fence = texture_future
                    .queue()
                    .map(|_| texture_future.then_signal_fence_and_flush().unwrap());

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

                let (clear_interface, rerender_interface) = interface.update();

                networking_system.changes_applied();

                if swapchain_holder.is_swapchain_invalid() {
                    let viewport = swapchain_holder.recreate_swapchain();

                    deferred_renderer.recreate_pipeline(viewport.clone(), swapchain_holder.window_size_u32());
                    interface_renderer.recreate_pipeline(viewport.clone(), swapchain_holder.window_size_u32());
                    picker_renderer.recreate_pipeline(viewport.clone(), swapchain_holder.window_size_u32());
                    shadow_renderer.recreate_pipeline(); // does this need to be recreated?

                    screen_targets = swapchain_holder.get_swapchain_images()
                        .into_iter()
                        .map(|swapchain_image| deferred_renderer.create_render_target(swapchain_image))
                        .collect();

                    interface_target = interface_renderer.create_render_target();

                    picker_targets = swapchain_holder.get_swapchain_images()
                        .into_iter()
                        .map(|_| picker_renderer.create_render_target())
                        .collect();

                    directional_shadow_targets = swapchain_holder.get_swapchain_images()
                        .into_iter()
                        .map(|_| shadow_renderer.create_render_target(8192))
                        .collect(); // does this need to be recreated?
                }

                if swapchain_holder.acquire_next_image().is_err() { // temporary check?
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
                    for index in 0..screen_targets.len() {
                        if let Some(mut fence) = screen_targets[index].state.try_take_fence() {
                            fence.wait(None).unwrap();
                            fence.cleanup_finished();
                        }
                    }
                }

                if let Some(mut fence) = texture_fence {
                    fence.wait(None).unwrap();
                    fence.cleanup_finished();
                }

                #[cfg(feature = "debug")]
                let current_camera: &mut (dyn Camera + Send + Sync) = match render_settings.use_debug_camera {
                    true => &mut debug_camera,
                    false => &mut player_camera,
                };

                #[cfg(not(feature = "debug"))]
                let current_camera: &mut (dyn Camera + Send + Sync) = &mut player_camera;

                current_camera.generate_view_projection(swapchain_holder.window_size());
                directional_shadow_camera.generate_view_projection(swapchain_holder.window_size());

                let image_number = swapchain_holder.get_image_number();
                let client_tick = game_timer.get_client_tick();
                let directional_shadow_image = directional_shadow_targets[image_number].image.clone();
                let screen_target = &mut screen_targets[image_number];
                let window_size = swapchain_holder.window_size_f32();
                let entities = &entities[..];

                thread_pool.in_place_scope(|scope| {

                    scope.spawn(|_| {

                        let picker_target = &mut picker_targets[image_number];

                        picker_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_tiles(picker_target, &picker_renderer, current_camera);

                        #[debug_condition(render_settings.show_entities)]
                        entities.iter().for_each(|entity| entity.render(picker_target, &picker_renderer, current_camera));

                        picker_target.finish();
                    });

                    scope.spawn(|_| {

                        let directional_shadow_target = &mut directional_shadow_targets[image_number];

                        directional_shadow_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_ground(directional_shadow_target, &shadow_renderer, &directional_shadow_camera);

                        #[debug_condition(render_settings.show_objects)]
                        map.render_objects(directional_shadow_target, &shadow_renderer, &directional_shadow_camera, client_tick);

                        #[debug_condition(render_settings.show_entities)]
                        entities.iter().for_each(|entity| entity.render(directional_shadow_target, &shadow_renderer, &directional_shadow_camera));

                        directional_shadow_target.finish();
                    });

                    scope.spawn(|_| {

                        screen_target.start();

                        #[debug_condition(render_settings.show_map)]
                        map.render_ground(screen_target, &deferred_renderer, current_camera);

                        #[debug_condition(render_settings.show_objects)]
                        map.render_objects(screen_target, &deferred_renderer, current_camera, client_tick);

                        #[debug_condition(render_settings.show_entities)]
                        entities.iter().for_each(|entity| entity.render(screen_target, &deferred_renderer, current_camera));

                        #[debug_condition(render_settings.show_water)]
                        map.render_water(screen_target, &deferred_renderer, current_camera, day_timer);

                        screen_target.lighting_pass();

                        #[debug_condition(render_settings.show_ambient_light && !render_settings.show_buffers())]
                        map.ambient_light(screen_target, &deferred_renderer, day_timer);

                        let (view_matrix, projection_matrix) = directional_shadow_camera.view_projection_matrices();
                        let light_matrix = projection_matrix * view_matrix;

                        #[debug_condition(render_settings.show_directional_light && !render_settings.show_buffers())]
                        map.directional_light(screen_target, &deferred_renderer, current_camera, directional_shadow_image.clone(), light_matrix, day_timer);

                        #[debug_condition(render_settings.show_point_lights && !render_settings.show_buffers())]
                        map.point_lights(screen_target, &deferred_renderer, current_camera);

                        // just for fun
                        entities.iter().for_each(|entity| entity.render_lights(screen_target, &deferred_renderer, current_camera));

                        #[debug_condition(render_settings.show_water && !render_settings.show_buffers())]
                        map.water_light(screen_target, &deferred_renderer, current_camera);

                        particle_holder.render(screen_target, &deferred_renderer, current_camera, window_size);
                    });

                    if rerender_interface {

                        interface_target.start_interface(clear_interface);

                        let state_provider = &StateProvider::new(&render_settings);
                        interface.render(&mut interface_target, &interface_renderer, state_provider, hovered_element);

                        interface_target.finish();
                    }
                });

                #[cfg(feature = "debug")]
                if render_settings.show_buffers() {

                    let picker_target = &mut picker_targets[image_number];

                    if let Some(fence) = picker_target.state.try_take_fence() {
                        fence.wait(None).unwrap();
                    }

                    deferred_renderer.overlay_buffers(screen_target, directional_shadow_image, picker_target.image.clone(), &render_settings);
                }

                if let Some(PickerTarget::Entity(entity_id)) = mouse_target {

                    let entity = entities
                        .iter()
                        .find(|entity| entity.get_entity_id() == entity_id);

                    if let Some(entity) = entity {
                        entity.render_status(screen_target, &deferred_renderer, current_camera, window_size);

                        if let Some(name) = &entity.get_details() {
                            let name = name.split("#").next().unwrap();
                            interface.render_hover_text(screen_target, &deferred_renderer, name, input_system.mouse_position());
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
                    interface.render_mouse_cursor(screen_target, &deferred_renderer, input_system.mouse_position());
                }

                let interface_future = interface_target.state.try_take_semaphore().unwrap_or_else(|| now(device.clone()).boxed());
                let directional_shadow_future = directional_shadow_targets[image_number].state.take_semaphore();
                let swapchain_acquire_future = swapchain_holder.take_acquire_future();

                let combined_future = interface_future
                    .join(directional_shadow_future)
                    .join(swapchain_acquire_future)
                    .boxed();

                screen_target.finish(swapchain_holder.get_swapchain(), combined_future, image_number);
            }

            _ignored => ()
        }
    });
}
