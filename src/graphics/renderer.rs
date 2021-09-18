use std::sync::Arc;
use std::iter;

use cgmath::{ Vector2, Vector3, Matrix4 };

use vulkano::device::physical::PhysicalDevice;
use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CommandBufferUsage, SubpassContents };
use vulkano::device::{ Device, Queue };
use vulkano::swapchain::{ AcquireError, Swapchain, SwapchainCreationError, SwapchainAcquireFuture, Surface, acquire_next_image };
use vulkano::image::{ ImageUsage, SwapchainImage };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::pipeline::{ GraphicsPipeline, PipelineBindPoint };
use vulkano::render_pass::{ Framebuffer, FramebufferAbstract, RenderPass, Subpass };
use vulkano::buffer::cpu_pool::CpuBufferPool;
use vulkano::buffer::BufferUsage;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::sampler::{ Filter, MipmapMode, Sampler, SamplerAddressMode };
use vulkano::format::Format;
use vulkano::sync::{ FlushError, GpuFuture, now };
use vulkano::buffer::BufferAccess;

use winit::window::Window;

#[cfg(feature = "debug")]
use debug::*;

use graphics::*;

macro_rules! create_sampler {
    ($device:expr, $filter_mode:ident, $address_mode:ident) => {
        Sampler::new(
            $device,
            Filter::$filter_mode,
            Filter::$filter_mode,
            MipmapMode::$filter_mode,
            SamplerAddressMode::$address_mode,
            SamplerAddressMode::$address_mode,
            SamplerAddressMode::$address_mode,
            0.0,
            1.0,
            0.0,
            0.0,
        ).unwrap()
    }
}

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
    deferred_vertex_shader: DeferredVertexShader,
    deferred_fragment_shader: DeferredFragmentShader,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    deferred_pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    color_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    normal_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    depth_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    current_frame: Option<CurrentFrame>,
    matrix_buffer: MatrixBuffer,
    dimensions: [u32; 2],
    nearest_sampler: Arc<Sampler>,
    linear_sampler: Arc<Sampler>,
    recreate_swapchain: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    ambient_light_renderer: AmbientLightRenderer,
    directional_light_renderer: DirectionalLightRenderer,
    point_light_renderer: PointLightRenderer,
    screen_vertex_buffer: ScreenVertexBuffer,
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

        let deferred_vertex_shader = DeferredVertexShader::load(device.clone()).unwrap();
        let deferred_fragment_shader = DeferredFragmentShader::load(device.clone()).unwrap();

        #[cfg(feature = "debug")]
        print_debug!("loaded {}vertex{} and {}fragment shaders{}", magenta(), none(), magenta(), none());

        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(device.clone(),
                attachments: {
                    output: {
                        load: Clear,
                        store: Store,
                        format: swapchain.format(),
                        samples: 1,
                    },
                    color: {
                        load: Clear,
                        store: DontCare,
                        format: Format::A2B10G10R10_UNORM_PACK32,
                        samples: 1,
                    },
                    normal: {
                        load: Clear,
                        store: DontCare,
                        format: Format::R16G16B16A16_SFLOAT,
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D32_SFLOAT,
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [color, normal],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                        color: [output],
                        depth_stencil: {},
                        input: [color, normal, depth]
                    }
                ]
            )
            .unwrap(),
        );

        #[cfg(feature = "debug")]
        print_debug!("created {}render pass{}", magenta(), none());

        let (deferred_pipeline, framebuffers, color_buffer, normal_buffer, depth_buffer, ambient_light_renderer, directional_light_renderer, point_light_renderer) = Self::window_size_dependent_setup(device.clone(), &deferred_vertex_shader, &deferred_fragment_shader, &images, render_pass.clone());

        #[cfg(feature = "debug")]
        print_debug!("created {}pipeline{}", magenta(), none());

        let matrix_buffer = CpuBufferPool::new(device.clone(), BufferUsage::all());

        #[cfg(feature = "debug")]
        print_debug!("created {}matrix buffer{}", magenta(), none());

        let nearest_sampler = create_sampler!(device.clone(), Nearest, Repeat);
        let linear_sampler = create_sampler!(device.clone(), Linear, Repeat);

        #[cfg(feature = "debug")]
        print_debug!("created {}sampler{}", magenta(), none());

        let vertices = vec![ScreenVertex::new(Vector2::new(-1.0, -1.0)), ScreenVertex::new(Vector2::new(-1.0, 3.0)), ScreenVertex::new(Vector2::new(3.0, -1.0))];
        let screen_vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

        let previous_frame_end = Some(now(device.clone()).boxed());

        return Self {
            queue: queue,
            device: device,
            deferred_vertex_shader: deferred_vertex_shader,
            deferred_fragment_shader: deferred_fragment_shader,
            swapchain: swapchain,
            render_pass: render_pass,
            deferred_pipeline: deferred_pipeline,
            framebuffers: framebuffers,
            color_buffer: color_buffer,
            normal_buffer: normal_buffer,
            matrix_buffer: matrix_buffer,
            depth_buffer: depth_buffer,
            current_frame: None,
            dimensions: dimensions,
            nearest_sampler: nearest_sampler,
            linear_sampler: linear_sampler,
            recreate_swapchain: false,
            previous_frame_end: previous_frame_end,
            ambient_light_renderer: ambient_light_renderer,
            directional_light_renderer: directional_light_renderer,
            point_light_renderer: point_light_renderer,
            screen_vertex_buffer: screen_vertex_buffer,
        }
    }

    fn window_size_dependent_setup(device: Arc<Device>, deferred_vertex_shader: &DeferredVertexShader, deferred_fragment_shader: &DeferredFragmentShader, images: &[Arc<SwapchainImage<Window>>], render_pass: Arc<RenderPass>) -> (
        Arc<GraphicsPipeline>, Vec<Arc<dyn FramebufferAbstract + Send + Sync>>, ImageBuffer, ImageBuffer, ImageBuffer, AmbientLightRenderer, DirectionalLightRenderer, PointLightRenderer) {

        let dimensions = images[0].dimensions();

        let color_buffer = ImageView::new(AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::A2B10G10R10_UNORM_PACK32).unwrap()).unwrap();
        let normal_buffer = ImageView::new(AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::R16G16B16A16_SFLOAT).unwrap()).unwrap();
        let depth_buffer = ImageView::new(AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::D32_SFLOAT).unwrap()).unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let image_buffer = ImageView::new(image.clone()).unwrap();
                let framebuffer = Framebuffer::start(render_pass.clone())
                    .add(image_buffer).unwrap()
                    .add(color_buffer.clone()).unwrap()
                    .add(normal_buffer.clone()).unwrap()
                    .add(depth_buffer.clone()).unwrap()
                    .build().unwrap();

                Arc::new(framebuffer) as Arc<dyn FramebufferAbstract + Send + Sync>
            })
            .collect::<Vec<_>>();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        let deferred_pipeline = Arc::new(GraphicsPipeline::start()
            .vertex_input_single_buffer::<Vertex>()
            .vertex_shader(deferred_vertex_shader.main_entry_point(), ())
            .triangle_list()
            .viewports_dynamic_scissors_irrelevant(1)
            .viewports(iter::once(viewport.clone()))
            .fragment_shader(deferred_fragment_shader.main_entry_point(), ())
            .depth_stencil_simple_depth()
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap());

        let subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let ambient_light_renderer = AmbientLightRenderer::new(device.clone(), subpass.clone(), viewport.clone());
        let directional_light_renderer = DirectionalLightRenderer::new(device.clone(), subpass.clone(), viewport.clone());
        let point_light_renderer = PointLightRenderer::new(device.clone(), subpass, viewport);

        return (deferred_pipeline, framebuffers, color_buffer, normal_buffer, depth_buffer, ambient_light_renderer, directional_light_renderer, point_light_renderer);
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

            let (new_deferred_pipeline, new_framebuffers, new_color_buffer, new_normal_buffer, new_depth_buffer, new_ambient_light_renderer, new_directional_light_renderer, new_point_light_renderer) = Self::window_size_dependent_setup(self.device.clone(), &self.deferred_vertex_shader, &self.deferred_fragment_shader, &new_images, self.render_pass.clone());

            #[cfg(feature = "debug")]
            print_debug!("recreated {}pipeline{}", magenta(), none());

            self.dimensions = new_dimensions;
            self.swapchain = new_swapchain;
            self.deferred_pipeline = new_deferred_pipeline;
            self.color_buffer = new_color_buffer;
            self.normal_buffer = new_normal_buffer;
            self.depth_buffer = new_depth_buffer;
            self.framebuffers = new_framebuffers;
            self.ambient_light_renderer = new_ambient_light_renderer;
            self.directional_light_renderer = new_directional_light_renderer;
            self.point_light_renderer = new_point_light_renderer;
            self.recreate_swapchain = false;

            #[cfg(feature = "debug")]
            timer.stop();
        }

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

        let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into(), [0.0, 0.0, 0.0, 1.0].into(), [0.0, 0.0, 0.0, 1.0].into(), 1f32.into()];

        let mut builder = AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        builder.begin_render_pass(self.framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap()
            .bind_pipeline_graphics(self.deferred_pipeline.clone());

        self.current_frame = Some(CurrentFrame::new(builder, image_num, acquire_future));
    }

    pub fn lighting_pass(&mut self) {
        if let Some(current_frame) = &mut self.current_frame {
            current_frame.builder.next_subpass(SubpassContents::Inline).unwrap();
        }
    }

    pub fn render_geomitry(&mut self, camera: &Camera, vertex_buffer: VertexBuffer, textures: &Vec<Texture>, transform: &Transform) {

        let matrix_buffer_data = camera.matrix_buffer_data(transform);
        let matrix_subbuffer = Arc::new(self.matrix_buffer.next(matrix_buffer_data).unwrap());

        let deferred_layout = self.deferred_pipeline.layout().descriptor_set_layouts().get(0).unwrap();
        let mut deferred_set_builder = PersistentDescriptorSet::start(deferred_layout.clone());

        // SUPER DIRTY, PLEASE FIX

        let texture0 = textures[0].clone();

        let texture1 = match textures.len() > 1 {
            true => textures[1].clone(),
            false => texture0.clone(),
        };

        let texture2 = match textures.len() > 2 {
            true => textures[2].clone(),
            false => texture0.clone(),
        };

        let texture3 = match textures.len() > 3 {
            true => textures[3].clone(),
            false => texture0.clone(),
        };

        //

        deferred_set_builder
            .add_buffer(matrix_subbuffer).unwrap()
            .enter_array().unwrap()
                .add_sampled_image(texture0, self.linear_sampler.clone()).unwrap()
                .add_sampled_image(texture1, self.linear_sampler.clone()).unwrap()
                .add_sampled_image(texture2, self.linear_sampler.clone()).unwrap()
                .add_sampled_image(texture3, self.linear_sampler.clone()).unwrap()
            .leave_array().unwrap();

        let deferred_set = Arc::new(deferred_set_builder.build().unwrap());

        if let Some(current_frame) = &mut self.current_frame {
            let vertex_count = vertex_buffer.size() as usize / std::mem::size_of::<Vertex>();

            current_frame.builder
                .bind_descriptor_sets(PipelineBindPoint::Graphics, self.deferred_pipeline.layout().clone(), 0, deferred_set)
                .bind_vertex_buffers(0, vertex_buffer)
                .draw(vertex_count as u32, 1, 0, 0).unwrap();
        }
    }

    pub fn ambient_light(&mut self, color: Color) {
        if let Some(current_frame) = &mut self.current_frame {
            self.ambient_light_renderer.render(&mut current_frame.builder, self.color_buffer.clone(), self.screen_vertex_buffer.clone(), color);
        }
    }

    pub fn directional_light(&mut self, direction: Vector3<f32>, color: Color) {
        if let Some(current_frame) = &mut self.current_frame {
            self.directional_light_renderer.render(&mut current_frame.builder, self.color_buffer.clone(), self.normal_buffer.clone(), self.screen_vertex_buffer.clone(), direction, color);
        }
    }

    pub fn point_light(&mut self, screen_to_world_matrix: Matrix4<f32>, position: Vector3<f32>, color: Color, intensity: f32) {
        if let Some(current_frame) = &mut self.current_frame {
            self.point_light_renderer.render(&mut current_frame.builder, self.color_buffer.clone(), self.normal_buffer.clone(), self.depth_buffer.clone(), self.screen_vertex_buffer.clone(), screen_to_world_matrix, position, color, intensity);
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

                Err(error) => {
                    println!("Failed to flush future: {:?}", error);
                    self.previous_frame_end = Some(now(self.device.clone()).boxed());
                }
            }
        }
    }
}
