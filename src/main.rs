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

#[cfg(feature = "debug")]
#[macro_use]
mod debug;
mod input;
mod system;
mod entities;
mod graphics;
mod managers;

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
use input::{ InputSystem, InputEvent };
use system::{ FrameTimer, DisplaySettings };
use graphics::*;
use managers::{ MapManager, ModelManager, TextureManager };

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

    let mut model_manager = ModelManager::new(device.clone());
    let mut texture_manager = TextureManager::new(device.clone(), queue.clone());
    let mut map_manager = MapManager::new(device.clone());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    let (font_map, mut font_future) = texture_manager.get(String::from("assets/font.png"));
    font_future.cleanup_finished();

    let map = map_manager.get(&mut model_manager, &mut texture_manager, String::from("pay_dun00.rsw"));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create renderer");

    let mut renderer = Renderer::new(&physical_device, device.clone(), queue.clone(), surface.clone(), &mut texture_manager);

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("initialize interface");

    let mut input_system = InputSystem::new();
    let mut display_settings = DisplaySettings::new();

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

    // TEMP
    player_camera.set_focus(Vector3::new(400.0, 0.0, 400.0));

    #[cfg(feature = "debug")]
    timer.stop();

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

                let delta_time = frame_timer.update();

                input_system.update_delta();

                for event in input_system.input_events() {
                    match event {
                        InputEvent::CameraZoom(factor) => player_camera.soft_zoom(factor),
                        InputEvent::CameraRotate(factor) => player_camera.soft_rotate(factor),
                        InputEvent::ToggleFramesPerSecond => display_settings.show_frames_per_second = !display_settings.show_frames_per_second,
                        #[cfg(feature = "debug")]
                        InputEvent::ToggleDebugCamera => display_settings.use_debug_camera = !display_settings.use_debug_camera,
                        #[cfg(feature = "debug")]
                        InputEvent::CameraLookAround(offset) => debug_camera.look_around(offset),
                        #[cfg(feature = "debug")]
                        InputEvent::CameraMoveForward => debug_camera.move_forward(delta_time as f32),
                        #[cfg(feature = "debug")]
                        InputEvent::CameraMoveBackward => debug_camera.move_backward(delta_time as f32),
                        #[cfg(feature = "debug")]
                        InputEvent::CameraMoveLeft => debug_camera.move_left(delta_time as f32),
                        #[cfg(feature = "debug")]
                        InputEvent::CameraMoveRight => debug_camera.move_right(delta_time as f32),
                        #[cfg(feature = "debug")]
                        InputEvent::CameraMoveUp => debug_camera.move_up(delta_time as f32),
                        #[cfg(feature = "debug")]
                        InputEvent::CameraMoveDown => debug_camera.move_down(delta_time as f32),
                    }
                }

                player_camera.update(delta_time);

                map.update(delta_time as f32);

                renderer.start_frame(&surface);

                #[cfg(feature = "debug")]
                let current_camera: &mut dyn Camera = match display_settings.use_debug_camera {
                    true => &mut debug_camera,
                    false => &mut player_camera,
                };

                #[cfg(not(feature = "debug"))]
                let current_camera = &mut player_camera;

                current_camera.generate_view_projection(renderer.get_window_size());

                map.render_geomitry(&mut renderer, current_camera, &display_settings);

                renderer.lighting_pass();

                map.render_lights(&mut renderer, current_camera, &display_settings);

                #[cfg(feature = "debug")]
                map.render_markers(&mut renderer, current_camera, &display_settings);

                if display_settings.show_frames_per_second {
                    renderer.render_text(font_map.clone(), &frame_timer.last_frames_per_second().to_string(), Vector2::new(20.0, 10.0), Color::new(55, 244, 22), 40.0);
                }

                renderer.stop_frame();
            }

            _ignored => ()
        }
    });
}
