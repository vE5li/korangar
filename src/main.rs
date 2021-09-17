extern crate vulkano;
extern crate vulkano_shaders;
extern crate vulkano_win;
extern crate winit;
extern crate cgmath;
extern crate png;

#[cfg(feature = "debug")]
extern crate chrono;

#[cfg(feature = "debug")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "debug")]
#[macro_use]
extern crate notify;

#[cfg(feature = "debug")]
#[macro_use]
mod debug;

mod graphics;
mod managers;

#[cfg(feature = "debug")]
use debug::*;

use graphics::*;
use managers::{ ModelManager, TextureManager };

use cgmath::{ Rad, Vector2, Vector3, InnerSpace };

use std::io::Cursor;
use std::time::Instant;
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{ Device, DeviceExtensions };
use vulkano::image::view::ImageView;
use vulkano::image::{ ImageDimensions, ImmutableImage, MipmapsCount };
use vulkano::instance::Instance;
use vulkano::sync::GpuFuture;
use vulkano::format::Format;
use vulkano::Version;
use vulkano_win::VkSurfaceBuild;
use winit::event::{ Event, WindowEvent, MouseButton, ElementState, MouseScrollDelta };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;

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
    let timer = Timer::new("create renderer");

    let mut renderer = Renderer::new(&physical_device, device.clone(), queue.clone(), surface.clone());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resource managers");

    let mut model_manager = ModelManager::new(device.clone());
    let mut texture_manager = TextureManager::new(device.clone(), queue.clone());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    let test_vertex_buffer = model_manager.get(String::from("models/test2.rsm"));

    #[cfg(feature = "debug")]
    print_debug!("created {}vertex buffer{}", magenta(), none());

    let (test_texture, test_texture_future) = texture_manager.get(String::from("textures/test_up.png"));
    let (test_normal_texture, test_normal_texture_future) = texture_manager.get(String::from("textures/test_up_normal.png"));
    let (test_specular_texture, test_specular_texture_future) = texture_manager.get(String::from("textures/test_up_specular.png"));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("setup reload watcher");

    #[cfg(feature = "debug")]
    let mut reload_watcher = ReloadWatcher::new("/home/korangar/", 200);

    #[cfg(feature = "debug")]
    timer.stop();

    let rotation_start = Instant::now();
    let mut previous_elapsed = 0.0;
    let mut counter_update_time = 0.0;
    let mut frame_counter = 0;

    let mut left_mouse_button_pressed = false;
    let mut right_mouse_button_pressed = false;
    let mut previous_mouse_position = Vector2::new(0.0, 0.0);

    let mut camera = Camera::new();
    let mut rotation = 0.0;

    test_texture_future
        .join(test_normal_texture_future)
        .join(test_specular_texture_future)
        .cleanup_finished();

    events_loop.run(move |event, _, control_flow| {
        match event {

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                renderer.invalidate_swapchain();
            }

            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                let new_mouse_position = Vector2::new(position.x, position.y);

                if left_mouse_button_pressed {
                    let delta = previous_mouse_position.x - new_mouse_position.x;

                    rotation += delta / 50.0;
                }

                if right_mouse_button_pressed {
                    let delta = previous_mouse_position.x - new_mouse_position.x;

                    camera.soft_rotate(delta as f32 / -50.0);
                }

                previous_mouse_position = new_mouse_position;
            }

            Event::WindowEvent { event: WindowEvent::MouseWheel{ delta, .. }, .. } => {
                if let MouseScrollDelta::LineDelta(_x, y) = delta {
                    camera.soft_zoom(y as f32 * -5.0);
                }
            }

            Event::WindowEvent { event: WindowEvent::MouseInput{ state, button, .. }, .. } => {

                if matches!(button, MouseButton::Left) {
                    left_mouse_button_pressed = matches!(state, ElementState::Pressed);
                }

                if matches!(button, MouseButton::Right) {
                    right_mouse_button_pressed = matches!(state, ElementState::Pressed);
                }
            }

            Event::RedrawEventsCleared => {

                while let Some(path) = reload_watcher.poll_event() {
                    if path.contains("/") {
                        let mut iterator = path.split("/");
                        let asset_type = iterator.next().unwrap();
                        let file_name = iterator.next().unwrap();

                        #[cfg(feature = "debug")]
                        print_debug!("asset {}{}{} of type {}{}{}", magenta(), file_name, none(), magenta(), asset_type, none());
                    }
                }

                let new_elapsed = rotation_start.elapsed().as_secs_f64();
                let delta_time = new_elapsed - previous_elapsed;
                previous_elapsed = new_elapsed;

                frame_counter += 1;
                counter_update_time += delta_time;

                if counter_update_time > 1.0 {
                    println!("FPS: {}", frame_counter);
                    counter_update_time = 0.0;
                    frame_counter = 0;
                }

                camera.update(delta_time);

                renderer.start_draw(&surface);
                camera.generate_view_projection(renderer.get_dimensions());

                renderer.draw_textured_deferred(&camera, test_vertex_buffer.clone(), test_texture.clone(), test_normal_texture.clone(), test_specular_texture.clone(), &Transform::rotation(Vector3::new(Rad(0.0), Rad(rotation as f32 / 2.845), Rad(0.0))));

                renderer.lighting_pass();

                let test = (new_elapsed as f32).sin();
                let screen_to_world_matrix = camera.screen_to_world_matrix();

                renderer.ambient_light(Color::new(5, 5, 5));
                renderer.directional_light(Vector3::new(0.0, -1.0, -0.7), Color::new(255, 255, 255));
                renderer.point_light(screen_to_world_matrix, Vector3::new(0.0, test, -4.0), Color::new(10, 255, 10), 50.0);
                renderer.point_light(screen_to_world_matrix, Vector3::new(0.0, test + 5.0, -1.0), Color::new(10, 10, 255), 100.0);
                renderer.point_light(screen_to_world_matrix, Vector3::new(0.0, test + 10.0, -5.0), Color::new(255, 10, 10), 100.0);

                renderer.stop_draw();
            }

            _ => ()
        }
    });
}

