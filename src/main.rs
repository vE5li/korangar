extern crate vulkano;
extern crate vulkano_shaders;
extern crate vulkano_win;
extern crate winit;
extern crate cgmath;
extern crate png;
extern crate bmp;
extern crate pathfinding;
#[cfg(feature = "debug")]
extern crate chrono;
#[cfg(feature = "debug")]
#[macro_use]
extern crate lazy_static;
extern crate pnet;

#[cfg(feature = "debug")]
#[macro_use]
mod debug;
mod input;
mod system;
#[macro_use]
mod maths;
mod map;
mod entity;
mod graphics;
mod loaders;
mod interface;
mod network;

use vulkano::device::physical::{ PhysicalDevice, PhysicalDeviceType };
use vulkano::device::{ Device, DeviceExtensions };
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
use input::{ InputSystem, UserEvent };
use system::FrameTimer;
use maths::Vector2;
use entity::Entity;
use graphics::*;
use loaders::{ MapLoader, ModelLoader, TextureLoader };
use interface::{ Interface, StateProvider };
use network::{ NetworkingSystem, NetworkEvent };

fn get_layers<'a>(desired_layers: Vec<&'a str>) -> Vec<&'a str> {
    let available_layers: Vec<_> = vulkano::instance::layers_list().unwrap().collect();
    println!("Available layers:");
    for l in &available_layers {
        println!("\t{}", l.name());
    }
    desired_layers
        .into_iter()
        .filter(|&l| available_layers.iter().any(|li| li.name() == l))
        .collect()
}

fn print_message_callback(message: &vulkano::instance::debug::Message) {

    let message_type = if message.ty.general {
        "general"
    } else if message.ty.validation {
        "validation"
    } else if message.ty.performance {
        "performance"
    } else {
        panic!("not implemented");
    };

    println!("{:?} [{}] : {}", message.layer_prefix, message_type, message.description);
}

fn main() {

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let extensions = vulkano::instance::InstanceExtensions {
        ext_debug_report: true,
        ..vulkano_win::required_extensions()
    };

    let layers = get_layers(vec!["VK_LAYER_KHRONOS_validation"]);
    println!("Using layers: {:?}", layers);

    //let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, Version::V1_1, &extensions, layers).expect("failed to create instance");

    let _debug_callback = vulkano::instance::debug::DebugCallback::new(
        &instance,
        MessageSeverity::all(),
        MessageType::all(),
        print_message_callback,
    ).ok();

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
    let timer = Timer::new("create device");

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

    let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
        .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
        .filter_map(|p| {
            p.queue_families()
                .find(|&q| q.supports_graphics() && surface.is_supported(q).unwrap_or(false))
                .map(|q| (p, q))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            PhysicalDeviceType::Other => 4,
        })
        .unwrap();

    let device_extensions = physical_device.required_extensions().union(&device_extensions);
    let (device, mut queues) = Device::new(physical_device, physical_device.supported_features(), &device_extensions, [(queue_family, 0.5)].iter().cloned()).expect("failed to create device");

    #[cfg(feature = "debug")]
    print_debug!("created {}vulkan device{}", magenta(), none());

    let queue = queues.next().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("received {}queue{} from {}device{}", magenta(), none(), magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resource managers");

    let mut model_loader = ModelLoader::new(device.clone());
    let mut texture_loader = TextureLoader::new(device.clone(), queue.clone());
    let mut map_loader = MapLoader::new(device.clone());

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

    let mut interface = Interface::new();
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

    let mut networking_system = NetworkingSystem::new();

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

                let network_events = networking_system.network_events();
                let (user_events, element_index) = input_system.user_events(&interface);

                for event in network_events {
                    match event {

                        NetworkEvent::PlayerMove(position_from, position_to) => {
                            let focus_point = entities[0].move_from_to(&map, position_from, position_to);
                            player_camera.set_focus(focus_point);

                            #[cfg(feature = "debug")]
                            entities[0].generate_steps_vertex_buffer(device.clone(), &map);
                        }

                        NetworkEvent::EntityAppear(entity_type, entity_id, character_id) => {
                            entities.push(Entity::new(&mut texture_loader, &mut texture_future));
                            println!("{} : {} : {}", entity_type, entity_id, character_id);
                        }
                    }
                }

                for event in user_events {
                    match event {
                        UserEvent::CameraZoom(factor) => player_camera.soft_zoom(factor),
                        UserEvent::CameraRotate(factor) => player_camera.soft_rotate(factor),
                        UserEvent::ToggleShowFramesPerSecond => render_settings.toggle_show_frames_per_second(),
                        UserEvent::ToggleShowMap => render_settings.toggle_show_map(),
                        UserEvent::ToggleShowObjects => render_settings.toggle_show_objects(),
                        UserEvent::ToggleShowEntities => render_settings.toggle_show_entities(),
                        UserEvent::ToggleShowAmbientLight => render_settings.toggle_show_ambient_light(),
                        UserEvent::ToggleShowDirectionalLight => render_settings.toggle_show_directional_light(),
                        UserEvent::ToggleShowPointLights => render_settings.toggle_show_point_lights(),
                        UserEvent::ToggleShowParticleLights => render_settings.toggle_show_particle_lights(),
                        UserEvent::MoveInterface(index, offset) => interface.move_hovered(index, offset),
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

                renderer.start_frame(&surface);

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
                if let Some(marker) = &hovered_marker {
                    println!("{:?} : {}", marker, map.marker_information(marker));
                }

                map.render_geomitry(&mut renderer, current_camera, &render_settings);

                if render_settings.show_entities {
                    entities.iter().for_each(|entity| entity.render(&mut renderer, current_camera));
                }

                #[cfg(feature = "debug")]
                if render_settings.show_pathing {
                    entities.iter().for_each(|entity| entity.render_pathing(&mut renderer, current_camera));
                }

                renderer.lighting_pass();

                #[cfg(feature = "debug")]
                match render_settings.show_buffers() {
                    true => renderer.render_buffers(current_camera, &render_settings),
                    false => map.render_lights(&mut renderer, current_camera, &render_settings),
                }

                #[cfg(not(feature = "debug"))]
                map.render_lights(&mut renderer, current_camera, &render_settings);

                #[cfg(feature = "debug")]
                map.render_markers(&mut renderer, current_camera, &render_settings);

                let state_provider = StateProvider::new(&render_settings, &entities[0]);
                interface.render(&mut renderer, &state_provider, element_index);

                if render_settings.show_frames_per_second {
                    renderer.render_text(&frame_timer.last_frames_per_second().to_string(), Vector2::new(10.0, 5.0), Color::new(150, 150, 150), 20.0);
                }

                renderer.stop_frame();
            }

            _ignored => ()
        }
    });
}
