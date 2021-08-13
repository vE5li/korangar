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
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/fragment_shader.glsl"
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

    let floor_vertex_buffer = model_manager.get(String::from("models/floor.obj"));
    let cube_vertex_buffer = model_manager.get(String::from("models/test3.obj"));

    #[cfg(feature = "debug")]
    print_debug!("created {}vertex buffer{}", magenta(), none());

    let (floor_texture, floor_texture_future) = texture_manager.get(String::from("textures/floor.png"));
    let (floor_normal_texture, floor_normal_texture_future) = texture_manager.get(String::from("textures/floor_normal.png"));
    let (floor_specular_texture, floor_specular_texture_future) = texture_manager.get(String::from("textures/floor_specular.png"));

    let (cube_texture, cube_texture_future) = texture_manager.get(String::from("textures/cube.png"));
    let (cube_normal_texture, cube_normal_texture_future) = texture_manager.get(String::from("textures/cube_normal.png"));
    let (cube_specular_texture, cube_specular_texture_future) = texture_manager.get(String::from("textures/cube_specular.png"));

    #[cfg(feature = "debug")]
    timer.stop();

    let rotation_start = Instant::now();
    let mut camera = Camera::new();

    floor_texture_future
        .join(floor_normal_texture_future)
        .join(floor_specular_texture_future)
        .join(cube_texture_future)
        .join(cube_normal_texture_future)
        .join(cube_specular_texture_future)
        .cleanup_finished();

    events_loop.run(move |event, _, control_flow| {
        match event {

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                renderer.invalidate_swapchain();
            }

            Event::RedrawEventsCleared => {
                renderer.start_draw(&surface);
                camera.generate_view_projection(renderer.get_dimensions());

                let elapsed = rotation_start.elapsed();
                let rotation = elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1_000_000_000.0;
                let transform = Transform::scale(Vector3::new(-1.5, -1.5, -1.5)) + Transform::position(Vector3::new(0.0, 0.5, 0.0)) + Transform::rotation(Vector3::new(Rad(rotation as f32 / 5.245), Rad(-rotation as f32 / 2.845), Rad(0.0)));

                renderer.draw_textured(&camera, floor_vertex_buffer.clone(), floor_texture.clone(), floor_normal_texture.clone(), floor_specular_texture.clone(), &Transform::rotation(Vector3::new(Rad(0.0), Rad(rotation as f32 / 2.845), Rad(0.0))));
//                renderer.draw_textured(&camera, cube_vertex_buffer.clone(), cube_texture.clone(), cube_normal_texture.clone(), cube_specular_texture.clone(), &transform);

                renderer.stop_draw();
            }

            _ => ()
        }
    });
}

