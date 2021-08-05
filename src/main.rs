extern crate vulkano;
extern crate vulkano_shaders;
extern crate vulkano_win;
extern crate winit;

#[cfg(feature = "debug")]
extern crate chrono;

#[cfg(feature = "debug")]
#[macro_use]
extern crate lazy_static;

#[cfg(feature = "debug")]
mod debug;

#[cfg(feature = "debug")]
use debug::*;

use std::sync::Arc;
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::command_buffer::{ AutoCommandBufferBuilder, CommandBufferUsage, DynamicState, SubpassContents };
use vulkano::device::physical::PhysicalDevice;
use vulkano::device::{ Device, DeviceExtensions };
use vulkano::image::view::ImageView;
use vulkano::image::{ ImageUsage, SwapchainImage };
use vulkano::instance::Instance;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{ Framebuffer, FramebufferAbstract, RenderPass, Subpass };
use vulkano::swapchain;
use vulkano::swapchain::{ AcquireError, Swapchain, SwapchainCreationError };
use vulkano::sync;
use vulkano::sync::{ FlushError, GpuFuture };
use vulkano::Version;
use vulkano_win::VkSurfaceBuild;
use winit::event::{ Event, WindowEvent };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::{ Window, WindowBuilder };

#[derive(Default, Debug, Clone, Copy)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {

    pub const fn new(x: f32, y: f32) -> Self {
        return Self {
            position: [x, y],
        }
    }
}

vulkano::impl_vertex!(Vertex, position);

const VERTICES: [Vertex; 3] = [ Vertex::new(-0.5, -0.25), Vertex::new(0.0, 0.5), Vertex::new(0.25, -0.1) ];

/*mod cs {
    vulkano_shaders::shader!{
        ty: "compute",
        src: "
#version 450

layout(local_size_x = 64, local_size_y = 1, local_size_z = 1) in;

layout(set = 0, binding = 0) buffer Data {
    uint data[];
} buf;

void main() {
    uint idx = gl_GlobalInvocationID.x;
    buf.data[idx] *= 12;
}"
    }
}*/

mod vertex_shader {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: "
#version 450

layout(location = 0) in vec2 position;

void main() {
				gl_Position = vec4(position, 0.0, 1.0);
}"
    }
}

mod fragment_shader {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: "
#version 450

layout(location = 0) out vec4 f_color;

void main() {
		 	f_color = vec4(1.0, 0.0, 0.0, 1.0);
}"
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

    //#[cfg(feature = "debug")]
    //print_debug!("device name is {}{}{}", magenta(), physical_device.name(), none());

    //#[cfg(feature = "debug")]
    //print_debug!("device if of type {}{:?}{}", magenta(), physical_device.ty(), none());

    let mut queue_families = physical_device.queue_families();

    #[cfg(feature = "debug")]
    for family in physical_device.queue_families() {
        print_debug!("found queue family with {}{}{} queues", magenta(), family.queues_count(), none());
    }

    let queue_family = queue_families.find(|&family| family.supports_graphics()).expect("couldn't find a graphical queue family");
    let device_extensions = DeviceExtensions {
        //khr_storage_buffer_storage_class: true,
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
    let surface = WindowBuilder::new().build_vk_surface(&events_loop, instance.clone()).unwrap();
    //println!("{:?}", surface.window().primary_monitor().size());

    #[cfg(feature = "debug")]
    print_debug!("created {}window{}", magenta(), none());

    let capabilities = surface.capabilities(physical_device).expect("failed to get surface capabilities");
    let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
    let format = capabilities.supported_formats[0].0;
    let dimensions: [u32; 2] = surface.window().inner_size().into();

    let (mut swapchain, images) = Swapchain::start(device.clone(), surface.clone())
        .num_images(capabilities.min_image_count)
        .format(format)
        .dimensions(dimensions)
        .usage(ImageUsage::color_attachment())
        .sharing_mode(&queue)
        .composite_alpha(composite_alpha)
        .build()
        .expect("failed to create swapchain");

    #[cfg(feature = "debug")]
    print_debug!("created {}swapchain{}", magenta(), none());

    //let (image_num, suboptimal, acquire_future) = acquire_next_image(swapchain.clone(), None).unwrap();

    //#[cfg(feature = "debug")]
    //print_debug!("acquired {}image{} from swapchain", magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("create resources");

    let vertex_buffer = CpuAccessibleBuffer::from_data(device.clone(), BufferUsage::all(), false, VERTICES).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}vertex buffer{}", magenta(), none());

    let vertex_shader = vertex_shader::Shader::load(device.clone()).unwrap();
    let fragment_shader = fragment_shader::Shader::load(device.clone()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("loaded {}vertex{} and {}fragment shaders{}", magenta(), none(), magenta(), none());

    let render_pass = Arc::new(
        vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap(),
    );

    #[cfg(feature = "debug")]
    print_debug!("created {}render pass{}", magenta(), none());

    let pipeline = Arc::new(
        GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .fragment_shader(fragment_shader.main_entry_point(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap()
    );

    #[cfg(feature = "debug")]
    print_debug!("created {}pipeline{}", magenta(), none());

    let mut dynamic_state = DynamicState {
        line_width: None,
        viewports: None,
        scissors: None,
        compare_mask: None,
        write_mask: None,
        reference: None,
    };

    #[cfg(feature = "debug")]
    print_debug!("created {}dynamic state{}", magenta(), none());

    let mut framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut dynamic_state);

    #[cfg(feature = "debug")]
    print_debug!("created {}framebuffers{}", magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();

    /*#[cfg(feature = "debug")]
    let timer = Timer::new("copy buffers");

    let source_content = 0 .. 64;
    let source = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, source_content).expect("failed to create buffer");
    
    let dest_content = (0 .. 64).map(|_| 0);
    let dest = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, dest_content).expect("failed to create buffer");

    #[cfg(feature = "debug")]
    print_debug!("created {}source{} and {}destination{} buffers", magenta(), none(), magenta(), none());

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder.copy_buffer(source.clone(), dest.clone()).unwrap();
    let command_buffer = builder.build().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}command buffer{}", magenta(), none());

    let finished = command_buffer.execute(queue.clone()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("execute command buffer");

    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("command buffer is done");

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("compute shader");

    let data_iter = 0 .. 128; //65536;
    let data_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, data_iter).expect("failed to create buffer");

    #[cfg(feature = "debug")]
    print_debug!("created {}data buffer{}", magenta(), none());

    let shader = cs::Shader::load(device.clone()).expect("failed to create shader module");

    #[cfg(feature = "debug")]
    print_debug!("created {}shader{}", magenta(), none());

    let compute_pipeline = Arc::new(ComputePipeline::new(device.clone(), &shader.main_entry_point(), &()).expect("failed to create compute pipeline"));

    #[cfg(feature = "debug")]
    print_debug!("created {}compute pipeline{}", magenta(), none());

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(PersistentDescriptorSet::start(layout.clone())
        .add_buffer(data_buffer.clone()).unwrap()
        .build().unwrap()
    );

    #[cfg(feature = "debug")]
    print_debug!("created {}descriptor set{}", magenta(), none());

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder.dispatch([1024, 1, 1], compute_pipeline.clone(), set.clone(), ()).unwrap();
    let command_buffer = builder.build().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}command buffer{}", magenta(), none());

    let finished = command_buffer.execute(queue.clone()).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("execute command buffer");

    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("command buffer is done");

    #[cfg(feature = "debug")]
    timer.stop();

    #[cfg(feature = "debug")]
    let timer = Timer::new("image");

    let image = StorageImage::new(device.clone(), Dimensions::Dim2d { width: 1024, height: 1024 }, Format::R8G8B8A8Unorm, Some(queue.family())).unwrap();

    #[cfg(feature = "debug")]
    print_debug!("created {}store image{}", magenta(), none());

    let mut builder = AutoCommandBufferBuilder::new(device.clone(), queue.family()).unwrap();
    builder.clear_color_image(image.clone(), ClearValue::Float([0.0, 0.0, 1.0, 1.0])).unwrap();
    let command_buffer = builder.build().unwrap();

    #[cfg(feature = "debug")]
    print_debug!("successfully {}cleared{} image", magenta(), none());

    #[cfg(feature = "debug")]
    timer.stop();*/

    let mut recreate_swapchain = false;
    let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

    events_loop.run(move |event, _, control_flow| {
        match event {

            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }

            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                recreate_swapchain = true;
            }

            Event::RedrawEventsCleared => {
                previous_frame_end.as_mut().unwrap().cleanup_finished();

                if recreate_swapchain {

                    #[cfg(feature = "debug")]
                    let timer = Timer::new("recreating swapchain");

                    let dimensions: [u32; 2] = surface.window().inner_size().into();
                    let (new_swapchain, new_images) =  match swapchain.recreate().dimensions(dimensions).build() {
                        Ok(r) => r,
                        Err(SwapchainCreationError::UnsupportedDimensions) => return,
                        Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                    };

                    swapchain = new_swapchain;
                    framebuffers = window_size_dependent_setup(&new_images, render_pass.clone(), &mut dynamic_state);
                    recreate_swapchain = false;

                    #[cfg(feature = "debug")]
                    timer.stop();
                }

                let (image_num, suboptimal, acquire_future) = match swapchain::acquire_next_image(swapchain.clone(), None) {
                    Ok(r) => r,
                    Err(AcquireError::OutOfDate) => {
                        recreate_swapchain = true;
                        return;
                    }
                    Err(e) => panic!("Failed to acquire next image: {:?}", e),
                };

                if suboptimal {
                    recreate_swapchain = true;
                }

                let clear_values = vec![[0.0, 0.0, 1.0, 1.0].into()];

                let mut builder = AutoCommandBufferBuilder::primary(device.clone(), queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

                builder
                    .begin_render_pass(framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap()
                    .draw(pipeline.clone(), &dynamic_state, vertex_buffer.clone(), (), ()).unwrap()
                    .end_render_pass().unwrap();

                let command_buffer = builder.build().unwrap();

                let future = previous_frame_end
                    .take().unwrap()
                    .join(acquire_future)
                    .then_execute(queue.clone(), command_buffer).unwrap()
                    .then_swapchain_present(queue.clone(), swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {

                    Ok(future) => {
                        previous_frame_end = Some(future.boxed());
                    }

                    Err(FlushError::OutOfDate) => {
                        recreate_swapchain = true;
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }

                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        previous_frame_end = Some(sync::now(device.clone()).boxed());
                    }
                }
            }

            _ => ()
        }
    });
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    dynamic_state: &mut DynamicState,
) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
    let dimensions = images[0].dimensions();

    let viewport = Viewport {
        origin: [0.0, 0.0],
        dimensions: [dimensions[0] as f32, dimensions[1] as f32],
        depth_range: 0.0..1.0,
    };
    dynamic_state.viewports = Some(vec![viewport]);

    images
        .iter()
        .map(|image| {
            let view = ImageView::new(image.clone()).unwrap();
            Arc::new(
                Framebuffer::start(render_pass.clone())
                    .add(view)
                    .unwrap()
                    .build()
                    .unwrap(),
            ) as Arc<dyn FramebufferAbstract + Send + Sync>
        })
        .collect::<Vec<_>>()
}
