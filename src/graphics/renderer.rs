use std::sync::Arc;
use std::iter;

use vulkano::device::physical::PhysicalDevice;
use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CommandBufferUsage, DynamicState, SubpassContents };
use vulkano::device::{ Device, Queue };
use vulkano::swapchain::{ AcquireError, Swapchain, SwapchainCreationError, SwapchainAcquireFuture, Surface, acquire_next_image };
use vulkano::image::{ ImageUsage, SwapchainImage };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{ GraphicsPipeline, GraphicsPipelineAbstract };
use vulkano::pipeline::vertex::BuffersDefinition;
use vulkano::render_pass::{ Framebuffer, FramebufferAbstract, RenderPass, Subpass };
use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::buffer::BufferUsage;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::sampler::{ Filter, MipmapMode, Sampler, SamplerAddressMode };
use vulkano::format::Format;
use vulkano::sync;
use vulkano::sync::{ FlushError, GpuFuture, now };

use winit::window::Window;

#[cfg(feature = "debug")]
use debug::*;

use graphics::*;

struct CurrentFrame {
    pub builder: AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pub image_num: usize,
    pub swapchain_future: SwapchainAcquireFuture<Window>,
}

impl CurrentFrame {

    pub fn new(builder: AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, image_num: usize, swapchain_future: SwapchainAcquireFuture<Window>) -> Self {
        return Self {
            builder: builder,
            image_num: image_num,
            swapchain_future: swapchain_future,
        }
    }
}

pub struct Renderer {
    queue: Arc<Queue>,
    device: Arc<Device>,
    vertex_shader: VertexShader,
    fragment_shader: FragmentShader,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    current_frame: Option<CurrentFrame>,
    matrix_buffer: MatrixBuffer,
    lights_buffer: LightsBuffer,
    dimensions: [u32; 2],
    sampler: Arc<Sampler>,
    sampler2: Arc<Sampler>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Renderer {

    pub fn new(physical_device: &PhysicalDevice, device: Arc<Device>, queue: Arc<Queue>, surface: Arc<Surface<Window>>) -> Self {

        let capabilities = surface.capabilities(*physical_device).expect("failed to get surface capabilities");
        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let format = capabilities.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();

        let (swapchain, images) = Swapchain::start(device.clone(), surface)
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

        let vertex_shader = VertexShader::load(device.clone()).unwrap();
        let fragment_shader = FragmentShader::load(device.clone()).unwrap();

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
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D16Unorm,
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {depth}
                }
            )
            .unwrap(),
        );

        #[cfg(feature = "debug")]
        print_debug!("created {}render pass{}", magenta(), none());

        let (pipeline, framebuffers) = Self::window_size_dependent_setup(device.clone(), &vertex_shader, &fragment_shader, &images, render_pass.clone());

        #[cfg(feature = "debug")]
        print_debug!("created {}pipeline{}", magenta(), none());

        let matrix_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        #[cfg(feature = "debug")]
        print_debug!("created {}matrix buffer{}", magenta(), none());

        let lights_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        #[cfg(feature = "debug")]
        print_debug!("created {}lights buffer{}", magenta(), none());

        let sampler = Sampler::new(
            device.clone(),
            Filter::Nearest,
            Filter::Nearest,
            MipmapMode::Nearest,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0,
            1.0,
            0.0,
            0.0,
        ).unwrap();

        let sampler2 = Sampler::new(
            device.clone(),
            Filter::Linear,
            Filter::Linear,
            MipmapMode::Linear,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            SamplerAddressMode::Repeat,
            0.0,
            1.0,
            0.0,
            0.0,
        ).unwrap();

        #[cfg(feature = "debug")]
        print_debug!("created {}sampler{}", magenta(), none());

        let previous_frame_end = Some(now(device.clone()).boxed());

        return Self {
            queue: queue,
            device: device,
            vertex_shader: vertex_shader,
            fragment_shader: fragment_shader,
            swapchain: swapchain,
            render_pass: render_pass,
            pipeline: pipeline,
            framebuffers: framebuffers,
            matrix_buffer: matrix_buffer,
            lights_buffer: lights_buffer,
            current_frame: None,
            dimensions: dimensions,
            sampler: sampler,
            sampler2: sampler2,
            recreate_swapchain: false,
            previous_frame_end: previous_frame_end,
        }
    }

    fn window_size_dependent_setup(device: Arc<Device>, vertex_shader: &VertexShader, fragment_shader: &FragmentShader, images: &[Arc<SwapchainImage<Window>>], render_pass: Arc<RenderPass>) -> (
        Arc<dyn GraphicsPipelineAbstract + Send + Sync>, Vec<Arc<dyn FramebufferAbstract + Send + Sync>>) {

        let dimensions = images[0].dimensions();
        let depth_buffer = ImageView::new(AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap()).unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let view = ImageView::new(image.clone()).unwrap();
                Arc::new(
                    Framebuffer::start(render_pass.clone())
                        .add(view).unwrap()
                        .add(depth_buffer.clone()).unwrap()
                        .build().unwrap(),
                ) as Arc<dyn FramebufferAbstract + Send + Sync>
            })
            .collect::<Vec<_>>();

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input(BuffersDefinition::new().vertex::<Vertex>())
                .vertex_shader(vertex_shader.main_entry_point(), ())
                .triangle_list()
                .viewports_dynamic_scissors_irrelevant(1)
                .viewports(iter::once(Viewport {
                    origin: [0.0, 0.0],
                    dimensions: [dimensions[0] as f32, dimensions[1] as f32],
                    depth_range: 0.0..1.0,
                }))
                .fragment_shader(fragment_shader.main_entry_point(), ())
                .depth_stencil_simple_depth()
                .blend_alpha_blending()
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(device.clone())
                .unwrap(),
        );

        (pipeline, framebuffers)
    }

    pub fn invalidate_swapchain(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn get_dimensions(&mut self) -> [u32; 2] {
        return self.dimensions;
    }

    pub fn start_draw(&mut self, surface: &Arc<Surface<Window>>) {
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {

            #[cfg(feature = "debug")]
            let timer = Timer::new("recreating swapchain");

            let new_dimensions: [u32; 2] = surface.window().inner_size().into();
            let (new_swapchain, new_images) =  match self.swapchain.recreate().dimensions(new_dimensions).build() {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            let (new_pipeline, new_framebuffers) = Self::window_size_dependent_setup(self.device.clone(), &self.vertex_shader, &self.fragment_shader, &new_images, self.render_pass.clone());

            #[cfg(feature = "debug")]
            print_debug!("recreated {}pipeline{}", magenta(), none());

            self.dimensions = new_dimensions;
            self.swapchain = new_swapchain;
            self.pipeline = new_pipeline;
            self.framebuffers = new_framebuffers;
            self.recreate_swapchain = false;

            #[cfg(feature = "debug")]
            timer.stop();
        }

        let mut builder = AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        let clear_values = vec![[0.02, 0.02, 0.02, 1.0].into(), 1f32.into()];

        let (image_num, suboptimal, acquire_future) = match acquire_next_image(self.swapchain.clone(), None) {
            Ok(r) => r,
            Err(AcquireError::OutOfDate) => {
                self.recreate_swapchain = true;
                return;
            }
            Err(e) => panic!("Failed to acquire next image: {:?}", e),
        };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        builder.begin_render_pass(self.framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap();

        self.current_frame = Some(CurrentFrame::new(builder, image_num, acquire_future));
    }

    pub fn draw_textured(&mut self, camera: &Camera, vertex_buffer: VertexBuffer, texture: Texture, bump_map: Texture, specular_map: Texture, transform: &Transform) {

        let matrix_buffer_data = camera.matrix_buffer_data(transform);
        let matrix_subbuffer = self.matrix_buffer.next(matrix_buffer_data).unwrap();

        let lights_buffer_data0 = Light {
            position: [0.0, 0.5, -2.0],
            color: [1.0, 0.05, 0.05],
            intensity: 0.7,
            _dummy0: [0; 4],
        };

        let lights_buffer_data1 = Light {
            position: [2.0, 0.5, 1.0],
            color: [0.05, 1.0, 0.05],
            intensity: 0.3,
            _dummy0: [0; 4],
        };

        let lights_buffer_data2 = Light {
            position: [-2.0, 0.5, 1.0],
            color: [0.05, 0.05, 1.0],
            intensity: 0.1,
            _dummy0: [0; 4],
        };

        let lights_subbuffer0 = self.lights_buffer.next(lights_buffer_data0).unwrap();
        let lights_subbuffer1 = self.lights_buffer.next(lights_buffer_data1).unwrap();
        let lights_subbuffer2 = self.lights_buffer.next(lights_buffer_data2).unwrap();

        let layout = self.pipeline.layout().descriptor_set_layouts().get(0).unwrap();
        let set = Arc::new(
            PersistentDescriptorSet::start(layout.clone())
                .add_buffer(matrix_subbuffer).unwrap()
                .enter_array().unwrap()
                    .add_buffer(lights_subbuffer0).unwrap()
                    .add_buffer(lights_subbuffer1).unwrap()
                    .add_buffer(lights_subbuffer2).unwrap()
                .leave_array().unwrap()
                .add_sampled_image(texture, self.sampler2.clone()).unwrap()
                .add_sampled_image(bump_map, self.sampler2.clone()).unwrap()
                .add_sampled_image(specular_map, self.sampler2.clone()).unwrap()
                .build().unwrap()
        );

        if let Some(current_frame) = &mut self.current_frame {
            current_frame.builder.draw(self.pipeline.clone(), &DynamicState::none(), vec![vertex_buffer], set, ()).unwrap();
        }
    }

    pub fn stop_draw(&mut self) {

        if let Some(mut current_frame) = self.current_frame.take() {

            current_frame.builder.end_render_pass().unwrap();
            let command_buffer = current_frame.builder.build().unwrap();

            let future = self.previous_frame_end
                 .take().unwrap()
                 .join(current_frame.swapchain_future)
                 .then_execute(self.queue.clone(), command_buffer).unwrap()
                 .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), current_frame.image_num)
                 .then_signal_fence_and_flush();

            match future {

                Ok(future) => {
                    self.previous_frame_end = Some(future.boxed());
                }

                Err(FlushError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    self.previous_frame_end = Some(now(self.device.clone()).boxed());
                }

                Err(e) => {
                    println!("Failed to flush future: {:?}", e);
                    self.previous_frame_end = Some(now(self.device.clone()).boxed());
                }
            }
        }
    }
}
