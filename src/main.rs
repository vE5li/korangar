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
use cgmath::num_traits::Float;

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
        path: "/home/korangar/shaders/vertex_shader.glsl"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "/home/korangar/shaders/fragment_shader.glsl"
    }
}

struct PartialVertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub texture_coordinates: Vector2<f32>,
}

impl PartialVertex {

    pub fn new(position: Vector3<f32>, normal: Vector3<f32>, texture_coordinates: Vector2<f32>) -> Self {
        return Self {
            position: position,
            normal: normal,
            texture_coordinates: texture_coordinates,
        }
    }
}

fn partial_vertex(vertex_positions: &Vec<Vector3<f32>>, normals: &Vec<Vector3<f32>>, texture_coordinates: &Vec<Vector2<f32>>, word: &str) -> PartialVertex {

    let mut components = word.split("/");
    let position_index: usize = components.next().expect("missing vertex position index").parse().expect("failed to parse vertex position index");
    let texture_index: usize = components.next().expect("missing vertex texture index").parse().expect("failed to parse vertex texture index");
    let normal_index: usize = components.next().expect("missing vertex normal index").parse().expect("failed to parse vertex normal index");

    let position_entry = &vertex_positions[position_index - 1];
    let texture_entry = &texture_coordinates[texture_index - 1];
    let normal_entry = &normals[normal_index - 1];

    return PartialVertex::new(*position_entry, *normal_entry, *texture_entry);
}

fn calculate_tangent_bitangent(first_partial: &PartialVertex, second_partial: &PartialVertex, third_partial: &PartialVertex) -> (Vector3<f32>, Vector3<f32>) {

    let delta_position_1 = second_partial.position - first_partial.position;
    let delta_position_2 = third_partial.position - first_partial.position;
    let delta_texture_coordinates_1 = second_partial.texture_coordinates - first_partial.texture_coordinates;
    let delta_texture_coordinates_2 = third_partial.texture_coordinates - first_partial.texture_coordinates;

    let r = 1.0 / (delta_texture_coordinates_1.x * delta_texture_coordinates_2.y - delta_texture_coordinates_1.y * delta_texture_coordinates_2.x);
    let tangent = (delta_position_1 * delta_texture_coordinates_2.y - delta_position_2 * delta_texture_coordinates_1.y) * r;
    let bitangent = (delta_position_2 * delta_texture_coordinates_1.x - delta_position_1 * delta_texture_coordinates_2.x) * r;

    return (tangent.normalize(), bitangent.normalize());
}

fn vertex_from_partial(partial_vertex: PartialVertex, tangent: Vector3<f32>, bitangent: Vector3<f32>) -> Vertex {
    return Vertex::new(partial_vertex.position, partial_vertex.normal, tangent, bitangent, partial_vertex.texture_coordinates);
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

    //let character_vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, VERTICES.iter().cloned()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}vertex buffer{}", magenta(), none());

    let mut vertex_positions: Vec<Vector3<f32>> = Vec::new();
    let mut normals: Vec<Vector3<f32>> = Vec::new();
    let mut texture_coordinates: Vec<Vector2<f32>> = Vec::new();
    let mut vertices: Vec<Vertex> = Vec::new();

    let contents = std::fs::read_to_string("/home/korangar/models/test3.obj").expect("Something went wrong reading the file");
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

            "vn" => {
                let nx = words.next().expect("failed to get normal x coordinate").parse().unwrap();
                let ny = words.next().expect("failed to get normal y coordinate").parse().unwrap();
                let nz = words.next().expect("failed to get normal z coordinate").parse().unwrap();
                normals.push(Vector3::new(nx, ny, nz));
            }

            "vt" => {
                let u = words.next().expect("failed to get u coordinate").parse().unwrap();
                let v = words.next().expect("failed to get v coordinate").parse().unwrap();
                texture_coordinates.push(Vector2::new(u, v));
            }

            "f" => {
                let first = words.next().expect("failed to get first vertex");
                let first_partial = partial_vertex(&vertex_positions, &normals, &texture_coordinates, first);
                let second = words.next().expect("failed to get second vertex");
                let second_partial = partial_vertex(&vertex_positions, &normals, &texture_coordinates, second);
                let third = words.next().expect("failed to get third vertex");
                let third_partial = partial_vertex(&vertex_positions, &normals, &texture_coordinates, third);

                let (tangent, bitangent) = calculate_tangent_bitangent(&first_partial, &second_partial, &third_partial);

                vertices.push(vertex_from_partial(first_partial, tangent, bitangent));
                vertices.push(vertex_from_partial(second_partial, tangent, bitangent));
                vertices.push(vertex_from_partial(third_partial, tangent, bitangent));
            }

            "o" | "#" | "s" => continue,

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

    let (bump_texture, bump_future) = {
        let png_bytes = include_bytes!("/home/korangar/textures/texture5_normal.png").to_vec();
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

    let (specular_texture, specular_future) = {
        let png_bytes = include_bytes!("/home/korangar/textures/texture5_specular.png").to_vec();
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
    tex_future.join(bump_future).join(specular_future).boxed().cleanup_finished();

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

                renderer.draw_textured(&camera, vertex_buffer.clone(), texture.clone(), bump_texture.clone(), specular_texture.clone(), &Transform::rotation(Vector3::new(Rad(rotation as f32 / 2.36), Rad(rotation as f32 / 5.845), Rad(0.0))));
                //renderer.draw_textured(&camera, character_vertex_buffer.clone(), texture.clone(), bump_texture.clone(), &Transform::rotation(Vector3::new(Rad(0.0), Rad(rotation as f32), Rad(0.0))));

                renderer.stop_draw();
            }

            _ => ()
        }
    });
}

