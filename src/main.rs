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

use cgmath::{ Rad, Vector2, Vector3 };
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

const VERTICES: [Vertex; 6] = [ Vertex::new(-0.8, -0.8, -0.0, 0.0, 1.0), Vertex::new(0.8, 0.8, 0.0, 1.0, 0.0), Vertex::new(0.8, -0.8, 0.0, 1.0, 1.0),
                                Vertex::new(-0.8, -0.8, -0.0, 0.0, 1.0), Vertex::new(-0.8, 0.8, -0.0, 0.0, 0.0), Vertex::new(0.8, 0.8, 0.0, 1.0, 0.0) ];

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 texture_coordinates;
layout(location = 0) out vec2 texture_coordinates_out;

layout(set = 0, binding = 1) uniform Matrices {
    mat4 world;
    mat4 view;
    mat4 projection;
} uniforms;

void main() {

    mat4 worldview = uniforms.view * uniforms.world;
    gl_Position = uniforms.projection * worldview * vec4(position, 1.0);

//				gl_Position = vec4(position, 1.0);

    texture_coordinates_out = texture_coordinates;
}"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450

layout(location = 0) in vec2 texture_coordinates;
layout(location = 0) out vec4 fragment_color;

layout (set = 0, binding = 0) uniform sampler2D tex;

void main() {
    float red = gl_FragCoord.x / 3840.0;
    float green = gl_FragCoord.y / 2160.0;
		 	vec4 color = vec4(texture_coordinates.x, texture_coordinates.y, 0.0, 1.0);
    vec4 pixel = texture(tex, texture_coordinates);

		 	//fragment_color = (pixel + pixel + color) / 3.0;
		 	//fragment_color = min(pixel, color);
		 	fragment_color = pixel;
}"
    }
}

fn unique_vertex(vertex_positions: &Vec<Vector3<f32>>, texture_coordinates: &Vec<Vector2<f32>>, word: &str) -> Vertex {

    let mut components = word.split("/");
    let position_index: usize = components.next().expect("missing vertex position index").parse().expect("failed to parse vertex position index");
    let texture_index: usize = components.next().expect("missing vertex texture index").parse().expect("failed to parse vertex texture index");

    let position_entry = &vertex_positions[position_index - 1];
    let texture_entry = &texture_coordinates[texture_index - 1];

    return Vertex::new(position_entry.x, position_entry.y, position_entry.z, texture_entry.x, texture_entry.y);
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
    let timer = Timer::new("create resources");

    let character_vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, VERTICES.iter().cloned()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}vertex buffer{}", magenta(), none());

    let mut vertex_positions: Vec<Vector3<f32>> = Vec::new();
    let mut texture_coordinates: Vec<Vector2<f32>> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();

    let contents = std::fs::read_to_string("/home/korangar/models/test2.obj").expect("Something went wrong reading the file");
    let mut lines = contents.split("\n");

    while let Some(line) = lines.next() {
        let mut words = line.split(" ");
        let line_type = words.next().expect("failed to get line type");

        if line_type.is_empty() {
            continue;
        }

        match line_type {

            "v" => {
                let x = words.next().expect("failed to get x coordinate").parse().unwrap();
                let y = words.next().expect("failed to get y coordinate").parse().unwrap();
                let z = words.next().expect("failed to get z coordinate").parse().unwrap();
                vertex_positions.push(Vector3::new(x, y, z));
            }

            "vt" => {
                let u = words.next().expect("failed to get u coordinate").parse().unwrap();
                let v = words.next().expect("failed to get v coordinate").parse().unwrap();
                texture_coordinates.push(Vector2::new(u, v));
            }

            "f" => {
                let first = words.next().expect("failed to get first vertex");
                let first_vertex = unique_vertex(&vertex_positions, &texture_coordinates, first);
                let second = words.next().expect("failed to get second vertex");
                let second_vertex = unique_vertex(&vertex_positions, &texture_coordinates, second);
                let third = words.next().expect("failed to get third vertex");
                let third_vertex = unique_vertex(&vertex_positions, &texture_coordinates, third);

                vertices.push(first_vertex);
                vertices.push(second_vertex);
                vertices.push(third_vertex);
            }

            invalid => println!("invalid type {:?}", invalid),
        }
    }

    let vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("loaded {}object file{}", magenta(), none());

    let (texture, tex_future) = {
        let png_bytes = include_bytes!("/home/korangar/textures/texture5.png").to_vec();
        let cursor = Cursor::new(png_bytes);
        let decoder = png::Decoder::new(cursor);
        let (info, mut reader) = decoder.read_info().unwrap();
        let dimensions = ImageDimensions::Dim2d {
            width: info.width,
            height: info.height,
            array_layers: 1,
        };

        let mut image_data = Vec::new();
        image_data.resize((info.width * info.height * 4) as usize, 0);
        reader.next_frame(&mut image_data).unwrap();

        let (image, future) = ImmutableImage::from_iter(image_data.iter().cloned(), dimensions, MipmapsCount::One, Format::R8G8B8A8Srgb, queue.clone()).unwrap();
        (ImageView::new(image).unwrap(), future)
    };

    #[cfg(feature = "debug")]
    print_debug!("created {}texture{}", magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();

     // TODO:
    tex_future.boxed().cleanup_finished();
    let rotation_start = Instant::now();

    let mut camera = Camera::new();

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

                renderer.draw_textured(&camera, vertex_buffer.clone(), texture.clone(), &Transform::position(Vector3::new(0.0, -2.0, 0.0)));
                renderer.draw_textured(&camera, character_vertex_buffer.clone(), texture.clone(), &Transform::rotation(Vector3::new(Rad(0.0), Rad(rotation as f32 * 2.0), Rad(0.0))));

                renderer.stop_draw();
            }

            _ => ()
        }
    });
}

