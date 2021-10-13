extern crate vulkano;
extern crate vulkano_shaders;
extern crate vulkano_win;
extern crate winit;
extern crate cgmath;
extern crate png;
extern crate bmp;
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
mod map;
mod graphics;
mod loaders;
mod interface;
mod network;

use cgmath::{ Vector3, Vector2 };
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{ Device, DeviceExtensions };
use vulkano::instance::Instance;
use vulkano::Version;
use vulkano_win::VkSurfaceBuild;
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;

#[cfg(feature = "debug")]
use debug::*;
use input::{ InputSystem, UserEvent };
use system::FrameTimer;
use graphics::*;
use loaders::{ MapLoader, ModelLoader, TextureLoader };
use interface::{ Interface, StateProvider };
use network::{ NetworkingSystem, NetworkEvent };

// entities = vec![player];

pub struct Entity {
    pub position: Vector2<f32>,
}

impl Entity {

    pub fn new() -> Self {

        let position = Vector2::new(400.0, 400.0);

        return Self { position };
    }

    pub fn move_from_to(&mut self, _from: Vector2<usize>, to: Vector2<usize>) {
        self.position =  to.map(|component| component as f32 * 5.0);
    }
}

fn main() {

    #[cfg(feature = "debug")]
    let timer = Timer::new("create device");

    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, Version::V1_1, &required_extensions, None).expect("failed to create instance");

    #[cfg(feature = "debug")]
    print_debug!("created {}instance{}", magenta(), none());

    let physical_device = PhysicalDevice::enumerate(&instance).next().expect("no device available");

    #[cfg(feature = "debug")]
    print_debug!("retrieved {}physical device{}", magenta(), none());

    let mut queue_families = physical_device.queue_families();

    #[cfg(feature = "debug")]
    for family in physical_device.queue_families() {
        print_debug!("found queue family with {}{}{} queues", magenta(), family.queues_count(), none());
    }

    let queue_family = queue_families.find(|&family| family.supports_graphics()).expect("couldn't find a graphical queue family");
    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };

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
    let timer = Timer::new("create window");

    let events_loop = EventLoop::new();
    let surface = WindowBuilder::new().with_title(String::from("korangar")).build_vk_surface(&events_loop, instance.clone()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}window{}", magenta(), none());

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

    let mut player = Entity::new();

    // TEMP
    let position = Vector3::new(player.position.x, 30.0, player.position.y);
    player_camera.set_focus(position);

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
                let (user_events, element_index) = input_system.user_events(&mut interface);

                for event in network_events {
                    match event {

                        NetworkEvent::PlayerMove(position_from, position_to) => {
                            player.move_from_to(position_from, position_to);

                            let position = Vector3::new(player.position.x, 30.0, player.position.y);
                            player_camera.set_focus(position);
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
                        UserEvent::CameraMoveDown => debug_camera.move_down(delta_time as f32),
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
                    }
                }

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

                map.render_geomitry(&mut renderer, current_camera, &render_settings);

                renderer.lighting_pass();

                map.render_lights(&mut renderer, current_camera, &render_settings);

                #[cfg(feature = "debug")]
                map.render_markers(&mut renderer, current_camera, &render_settings);

                let state_provider = StateProvider::new(&render_settings);
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
