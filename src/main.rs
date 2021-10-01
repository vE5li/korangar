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
extern crate notify;

#[cfg(feature = "debug")]
#[macro_use]
mod debug;

mod graphics;
mod managers;

#[cfg(feature = "debug")]
use debug::*;

use graphics::*;
use managers::{ MapManager, ModelManager, TextureManager };

use cgmath::{ Rad, Vector2, Vector3 };

use std::time::Instant;
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{ Device, DeviceExtensions };
use vulkano::instance::Instance;
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
    let mut map_manager = MapManager::new(device.clone());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("load resources");

    //let model = model_manager.get(&mut texture_manager, String::from("data/model/2.rsm"));

    let (font_map, mut font_future) = texture_manager.get(String::from("assets/font.png"));

    let map = map_manager.get(&mut model_manager, &mut texture_manager, String::from("pay_dun00.gnd"));

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("setup reload watcher");

    #[cfg(feature = "debug")]
    let mut reload_watcher = ReloadWatcher::new("/home/korangar/", 200);

    #[cfg(feature = "debug")]
    timer.stop();

    let frame_timer = Instant::now();
    let mut previous_elapsed = 0.0;
    let mut counter_update_time = 0.0;
    let mut frame_counter = 0;
    let mut frames_per_second = 0;

    let mut left_mouse_button_pressed = false;
    let mut right_mouse_button_pressed = false;
    let mut w_button_pressed = false;
    let mut a_button_pressed = false;
    let mut s_button_pressed = false;
    let mut d_button_pressed = false;
    let mut space_pressed = false;
    let mut shift_pressed = false;

    let mut previous_mouse_position = Vector2::new(0.0, 0.0);
    let mut new_mouse_position = Vector2::new(0.0, 0.0);

    let mut player_camera = PlayerCamera::new();

    #[cfg(feature = "debug")]
    let mut debug_camera = DebugCamera::new();

    #[cfg(feature = "debug")]
    let mut use_debug_camera = false;

    let mut player_position = Vector3::new(400.0, 0.0, 400.0);
    player_camera.set_focus(player_position);

    font_future.cleanup_finished();

    events_loop.run(move |event, _, control_flow| {
        match event {

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                renderer.invalidate_swapchain();
            }

            Event::WindowEvent { event: WindowEvent::CursorMoved { position, .. }, .. } => {
                new_mouse_position = Vector2::new(position.x, position.y);

                #[cfg(feature = "debug")]
                if (use_debug_camera) {
                    //let center_position = PhysicalPosition::new(0.5, 0.5);
                    //surface.window().set_cursor_position(center_position).unwrap();
                }
            }

            Event::WindowEvent { event: WindowEvent::MouseWheel{ delta, .. }, .. } => {
                if let MouseScrollDelta::LineDelta(_x, y) = delta {
                    player_camera.soft_zoom(y as f32 * -5.0);
                }
            }

            Event::WindowEvent { event: WindowEvent::MouseInput{ state, button, .. }, .. } => {
                let pressed = matches!(state, ElementState::Pressed);

                match button {
                    MouseButton::Left => left_mouse_button_pressed = pressed,
                    MouseButton::Right => right_mouse_button_pressed = pressed,
                    _ignored => {},
                }
            }

            Event::WindowEvent { event: WindowEvent::KeyboardInput{ input, .. }, .. } => {
                let pressed = matches!(input.state, ElementState::Pressed);

                #[cfg(feature = "debug")]
                match input.scancode {

                    17 => w_button_pressed = pressed,

                    31 => s_button_pressed = pressed,

                    30 => a_button_pressed = pressed,

                    32 => d_button_pressed = pressed,

                    57 => space_pressed = pressed,

                    42 => shift_pressed = pressed,

                    33 => {
                        if pressed {
                            use_debug_camera = !use_debug_camera;
//                          surface.window().set_cursor_grab(use_debug_camera).unwrap();
//                          surface.window().set_cursor_visible(!use_debug_camera);
                        }
                    },

                    _ignored => {},
                }

                //println!("{:?}", input);
            }

            Event::RedrawEventsCleared => {

                #[cfg(feature = "debug")]
                while let Some(path) = reload_watcher.poll_event() {
                    if path.contains("/") {
                        let mut iterator = path.split("/");
                        let asset_type = iterator.next().unwrap();
                        let file_name = iterator.next().unwrap();

                        #[cfg(feature = "debug")]
                        print_debug!("asset {}{}{} of type {}{}{}", magenta(), file_name, none(), magenta(), asset_type, none());
                    }
                }

                let new_elapsed = frame_timer.elapsed().as_secs_f64();
                let delta_time = new_elapsed - previous_elapsed;
                previous_elapsed = new_elapsed;

                frame_counter += 1;
                counter_update_time += delta_time;

                if counter_update_time > 1.0 {
                    frames_per_second = frame_counter;
                    counter_update_time = 0.0;
                    frame_counter = 0;
                }

                let mouse_delta = previous_mouse_position - new_mouse_position;

                if right_mouse_button_pressed {
                    player_camera.soft_rotate(mouse_delta.x as f32 / -50.0);
                }

                previous_mouse_position = new_mouse_position;

                renderer.start_draw(&surface);

                player_camera.update(delta_time);

                #[cfg(feature = "debug")]
                if (w_button_pressed) {
                    debug_camera.move_forward(delta_time as f32);
                }

                #[cfg(feature = "debug")]
                if (s_button_pressed) {
                    debug_camera.move_backward(delta_time as f32);
                }

                #[cfg(feature = "debug")]
                if (a_button_pressed) {
                    debug_camera.move_left(delta_time as f32);
                }

                #[cfg(feature = "debug")]
                if (d_button_pressed) {
                    debug_camera.move_right(delta_time as f32);
                }

                #[cfg(feature = "debug")]
                if (space_pressed) {
                    debug_camera.move_up(delta_time as f32);
                }

                #[cfg(feature = "debug")]
                if (shift_pressed) {
                    debug_camera.move_down(delta_time as f32);
                }

                #[cfg(feature = "debug")]
                if (use_debug_camera && left_mouse_button_pressed) {
                    debug_camera.look_around(mouse_delta);
                }

                #[cfg(feature = "debug")]
                let current_camera: &mut dyn Camera = match use_debug_camera {
                    true => &mut debug_camera,
                    false => &mut player_camera,
                };

                #[cfg(not(feature = "debug"))]
                let current_camera = &mut player_camera;

                current_camera.generate_view_projection(renderer.get_window_size());

                map.render_geomitry(&mut renderer, current_camera);
                //model.render_geomitry(&mut renderer, &camera, &Transform::rotation(Vector3::new(Rad(0.0), Rad(rotation as f32), Rad(0.0))));

                renderer.lighting_pass();

                let screen_to_world_matrix = current_camera.screen_to_world_matrix();

                renderer.ambient_light(Color::new(60, 60, 60));
                //renderer.directional_light(Vector3::new(0.0, -1.0, -0.7), Color::new(100, 100, 100));

                renderer.point_light(screen_to_world_matrix, Vector3::new(100.0, 10.0, 100.0), Color::new(255, 10, 10), 60.0);
                renderer.point_light(screen_to_world_matrix, Vector3::new(150.0, 10.0, 150.0), Color::new(10, 255, 10), 20.0);
                renderer.point_light(screen_to_world_matrix, Vector3::new(150.0, 10.0, 300.0), Color::new(10, 10, 255), 40.0);
                renderer.point_light(screen_to_world_matrix, Vector3::new(300.0, 10.0, 110.0), Color::new(255, 255, 255), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(300.0, 10.0, 300.0), Color::new(255, 10, 10), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(300.0, 10.0, 150.0), Color::new(10, 255, 10), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(300.0, 10.0, 450.0), Color::new(10, 10, 255), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(450.0, 10.0, 300.0), Color::new(255, 255, 255), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(700.0, 10.0, 450.0), Color::new(255, 10, 10), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(450.0, 10.0, 700.0), Color::new(10, 255, 10), 40.0);
                //renderer.point_light(screen_to_world_matrix, Vector3::new(700.0, 10.0, 700.0), Color::new(10, 10, 255), 40.0);

                renderer.render_text(font_map.clone(), &frames_per_second.to_string(), Vector2::new(20.0, 10.0), Color::new(55, 244, 22), 40.0);

                #[cfg(feature = "debug")]
                match use_debug_camera {
                    true => renderer.render_text(font_map.clone(), "debug camera", Vector2::new(20.0, 60.0), Color::new(255, 255, 255), 30.0),
                    false => renderer.render_text(font_map.clone(), "player camera", Vector2::new(20.0, 60.0), Color::new(255, 255, 255), 30.0),
                }

                renderer.stop_draw();
            }

            _ignored => ()
        }
    });
}
