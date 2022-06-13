#![feature(unzip_option)]
#![feature(let_else)]

#[macro_use]
extern crate korangar_procedural;
extern crate derive_new;
extern crate vulkano;
extern crate vulkano_shaders;
extern crate vulkano_win;
extern crate winit;
extern crate num;
extern crate cgmath;
extern crate serde;
extern crate serde_json;
extern crate png;
extern crate bmp;
extern crate pathfinding;
#[cfg(feature = "debug")]
extern crate chrono;
#[cfg(feature = "debug")]
#[macro_use]
extern crate lazy_static;
extern crate pnet;
extern crate yazi;

#[cfg(feature = "debug")]
#[macro_use]
mod debug;
#[macro_use]
mod types;
mod input;
#[macro_use]
mod system;
mod graphics;
mod loaders;
mod interface;
//mod network;

use vulkano::device::Device;
use vulkano::instance::Instance;
use vulkano::instance::debug::{ MessageSeverity, MessageType };
use vulkano::Version;
use vulkano::sync::{ GpuFuture, now };
use vulkano_win::VkSurfaceBuild;
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;

#[cfg(feature = "debug")]
use debug::*;
use types::Entity;
use input::{ InputSystem, UserEvent };
use system::{ FrameTimer, get_instance_extensions, get_layers, get_device_extensions };
use loaders::{ GameFileLoader, MapLoader, ModelLoader, TextureLoader };
use graphics::{ Renderer, RenderSettings };
use graphics::camera::*;
use interface::*;
//use network::{ NetworkingSystem, NetworkEvent };

fn main() {

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let instance = Instance::new(None, Version::V1_1, &get_instance_extensions(), get_layers()).expect("failed to create instance");

    #[cfg(feature = "debug")]
    let _debug_callback = vulkano::instance::debug::DebugCallback::new(&instance, MessageSeverity::all(), MessageType::all(), vulkan_message_callback).ok();

    #[cfg(feature = "debug")]
    print_debug!("created {}instance{}", magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create window");

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new().with_title(String::from("korangar")).build_vk_surface(&events_loop, instance.clone()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}window{}", magenta(), none());

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
    print_debug!("created {}vulkan device{}", magenta(), none());

    let queue = queues.next().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("received {}queue{} from {}device{}", magenta(), none(), magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resource managers");

    let mut game_file_loader = GameFileLoader::new();
    let mut model_loader = ModelLoader::new(device.clone());
    let mut texture_loader = TextureLoader::new(device.clone(), queue.clone());
    let mut map_loader = MapLoader::new(device.clone());

    //game_file_loader.get(String::from("data.grf"));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    let map = map_loader.get(&mut model_loader, &mut texture_loader, String::from("pay_dun00.rsw"));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create renderer");

    let mut renderer = Renderer::new(&physical_device, device.clone(), queue.clone(), surface.clone(), &mut texture_loader);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize interface");

    let window_size = Size::from(renderer.get_window_size().map(|c| c as f32));
    let mut interface = Interface::new(window_size);
    let mut input_system = InputSystem::new();
    let mut render_settings = RenderSettings::new();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize timer");

    let mut frame_timer = FrameTimer::new();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize camera");

    #[cfg(feature = "debug")]
    let mut debug_camera = DebugCamera::new();
    let mut player_camera = PlayerCamera::new();

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize networking");

    //let mut networking_system = NetworkingSystem::new();

    #[cfg(feature = "debug")]
    timer.stop();

    let mut texture_future = now(device.clone()).boxed();

    let mut entities = vec![Entity::new(&mut texture_loader, &mut texture_future)];

    texture_future.flush().unwrap();
    texture_future.cleanup_finished();

    // TEMP
    player_camera.set_focus(entities[0].position);

    events_loop.run(move |event, _, control_flow| {
        match event {

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                let window_size = surface.window().inner_size();
                let window_size = Size::new(window_size.width as f32, window_size.height as f32);
                interface.update_window_size(window_size);
                renderer.invalidate_swapchain();
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

                let delta_time = frame_timer.update();

                //let network_events = networking_system.network_events();
                let (user_events, hovered_element) = input_system.user_events(&mut interface);

                //for event in network_events {
                //    match event {

                //        NetworkEvent::PlayerMove(position_from, position_to) => {
                //            let focus_point = entities[0].move_from_to(&map, position_from, position_to);
                //            player_camera.set_focus(focus_point);

                //            #[cfg(feature = "debug")]
                //            entities[0].generate_steps_vertex_buffer(device.clone(), &map);
                //        }

                //        NetworkEvent::EntityAppear(entity_type, entity_id, character_id) => {
                //            entities.push(Entity::new(&mut texture_loader, &mut texture_future));
                //            println!("{} : {} : {}", entity_type, entity_id, character_id);
                //        }
                //    }
                //}

                for event in user_events {
                    match event {
                        UserEvent::Exit => *control_flow = ControlFlow::Exit,
                        UserEvent::CameraZoom(factor) => player_camera.soft_zoom(factor),
                        UserEvent::CameraRotate(factor) => player_camera.soft_rotate(factor),
                        UserEvent::ToggleFrameLimit => {
                            render_settings.toggle_frame_limit();
                            renderer.set_frame_limit(render_settings.frame_limit);
                            interface.schedule_rerender();
                        },
                        UserEvent::OpenMenuWindow => interface.open_window(&MenuWindow::default()),
                        UserEvent::OpenGraphicsSettingsWindow => interface.open_window(&GraphicsSettingsWindow::default()),
                        UserEvent::OpenAudioSettingsWindow => interface.open_window(&AudioSettingsWindow::default()),
                        UserEvent::ReloadTheme => interface.reload_theme(),
                        UserEvent::SaveTheme => interface.save_theme(),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenRenderSettingsWindow => interface.open_window(&RenderSettingsWindow::default()),
                        #[cfg(feature = "debug")]
                        UserEvent::OpenMapDataWindow => interface.open_window(std::ops::Deref::deref(&map)),
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
                        UserEvent::ToggleShowMap => render_settings.toggle_show_map(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowObjects => render_settings.toggle_show_objects(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowEntities => render_settings.toggle_show_entities(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowAmbientLight => render_settings.toggle_show_ambient_light(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDirectionalLight => render_settings.toggle_show_directional_light(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowPointLights => render_settings.toggle_show_point_lights(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowParticleLights => render_settings.toggle_show_particle_lights(),
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
                        UserEvent::ToggleShowMapTiles => render_settings.toggle_show_map_tiles(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowPathing => render_settings.toggle_show_pathing(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDiffuseBuffer => render_settings.toggle_show_diffuse_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowNormalBuffer => render_settings.toggle_show_normal_buffer(),
                        #[cfg(feature = "debug")]
                        UserEvent::ToggleShowDepthBuffer => render_settings.toggle_show_depth_buffer(),
                    }
                }

                entities.iter_mut().for_each(|entity| entity.update(delta_time as f32));

                player_camera.update(delta_time);

                map.update(delta_time as f32);

                let renderer_interface = interface.update();
                renderer.start_frame(&surface, renderer_interface);

                #[cfg(feature = "debug")]
                let current_camera: &mut dyn Camera = match render_settings.use_debug_camera {
                    true => &mut debug_camera,
                    false => &mut player_camera,
                };

                #[cfg(not(feature = "debug"))]
                let current_camera = &mut player_camera;

                current_camera.generate_view_projection(renderer.get_window_size());

                #[cfg(feature = "debug")]
                let hovered_marker = map.hovered_marker(&mut renderer, current_camera, &render_settings, input_system.mouse_position());

                #[cfg(feature = "debug")]
                if let Some(marker) = hovered_marker {
                    if input_system.unused_left_click() {
                        let prototype_window = map.resolve_marker(marker);
                        interface.open_window(prototype_window);
                        input_system.set_interface_clicked();
                    }
                }

                map.render_geomitry(&mut renderer, current_camera, &render_settings);

                if render_settings.show_entities {
                    entities.iter().for_each(|entity| entity.render(&mut renderer, current_camera));
                }

                #[cfg(feature = "debug")]
                if render_settings.show_pathing {
                    entities.iter().for_each(|entity| entity.render_pathing(&mut renderer, current_camera));
                }

                let state_provider = StateProvider::new(&render_settings, &entities[0]);
                interface.render(&mut renderer, &state_provider, hovered_element);

                renderer.lighting_pass();

                #[cfg(feature = "debug")]
                match render_settings.show_buffers() {
                    true => renderer.render_buffers(current_camera, &render_settings),
                    false => map.render_lights(&mut renderer, current_camera, &render_settings),
                }

                #[cfg(not(feature = "debug"))]
                map.render_lights(&mut renderer, current_camera, &render_settings);

                #[cfg(feature = "debug")]
                map.render_markers(&mut renderer, current_camera, &render_settings, hovered_marker);

                renderer.render_interface(&render_settings);

                if render_settings.show_frames_per_second {
                    interface.render_frames_per_second(&mut renderer, frame_timer.last_frames_per_second());
                }

                renderer.stop_frame();
            }

            _ignored => ()
        }
    });
}
