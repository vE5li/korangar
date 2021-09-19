mod deferred;
mod lighting;

use std::sync::Arc;

use cgmath::{ Vector2, Vector3, Matrix4 };

use vulkano::device::physical::PhysicalDevice;
use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, CommandBufferUsage, SubpassContents };
use vulkano::device::{ Device, Queue };
use vulkano::swapchain::{ AcquireError, Swapchain, SwapchainCreationError, SwapchainAcquireFuture, Surface, acquire_next_image };
use vulkano::image::{ ImageUsage, SwapchainImage };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::render_pass::{ Framebuffer, FramebufferAbstract, RenderPass, Subpass };
use vulkano::buffer::BufferUsage;
use vulkano::format::Format;
use vulkano::sync::{ FlushError, GpuFuture, now };

use winit::window::Window;

#[cfg(feature = "debug")]
use debug::*;

use graphics::*;

use self::deferred::DeferredRenderer;
use self::lighting::*;

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
    deferred_renderer: DeferredRenderer,
    ambient_light_renderer: AmbientLightRenderer,
    directional_light_renderer: DirectionalLightRenderer,
    point_light_renderer: PointLightRenderer,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    color_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    normal_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    depth_buffer: Arc<ImageView<Arc<AttachmentImage>>>,
    screen_vertex_buffer: ScreenVertexBuffer,
    current_frame: Option<CurrentFrame>,
    dimensions: [u32; 2],
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

        let (deferred_renderer, ambient_light_renderer, directional_light_renderer, point_light_renderer, framebuffers, color_buffer, normal_buffer, depth_buffer) = Self::window_size_dependent_setup(device.clone(), &images, render_pass.clone());

        #[cfg(feature = "debug")]
        print_debug!("created {}pipeline{}", magenta(), none());

        let vertices = vec![ScreenVertex::new(Vector2::new(-1.0, -1.0)), ScreenVertex::new(Vector2::new(-1.0, 3.0)), ScreenVertex::new(Vector2::new(3.0, -1.0))];
        let screen_vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

        let previous_frame_end = Some(now(device.clone()).boxed());

        return Self {
            queue: queue,
            device: device,
            deferred_renderer: deferred_renderer,
            ambient_light_renderer: ambient_light_renderer,
            directional_light_renderer: directional_light_renderer,
            point_light_renderer: point_light_renderer,
            swapchain: swapchain,
            render_pass: render_pass,
            framebuffers: framebuffers,
            color_buffer: color_buffer,
            normal_buffer: normal_buffer,
            depth_buffer: depth_buffer,
            screen_vertex_buffer: screen_vertex_buffer,
            current_frame: None,
            dimensions: dimensions,
            recreate_swapchain: false,
            previous_frame_end: previous_frame_end,
        }
    }

    fn window_size_dependent_setup(device: Arc<Device>, images: &[Arc<SwapchainImage<Window>>], render_pass: Arc<RenderPass>) -> (
        DeferredRenderer, AmbientLightRenderer, DirectionalLightRenderer, PointLightRenderer, Vec<Arc<dyn FramebufferAbstract + Send + Sync>>, ImageBuffer, ImageBuffer, ImageBuffer) {

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

        let deferred_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let deferred_renderer = DeferredRenderer::new(device.clone(), deferred_subpass, viewport.clone());
        let ambient_light_renderer = AmbientLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let directional_light_renderer = DirectionalLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let point_light_renderer = PointLightRenderer::new(device.clone(), lighting_subpass, viewport);

        return (deferred_renderer, ambient_light_renderer, directional_light_renderer, point_light_renderer, framebuffers, color_buffer, normal_buffer, depth_buffer);
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

            let (new_deferred_renderer, new_ambient_light_renderer, new_directional_light_renderer, new_point_light_renderer, new_framebuffers, new_color_buffer, new_normal_buffer, new_depth_buffer) = Self::window_size_dependent_setup(self.device.clone(), &new_images, self.render_pass.clone());

            #[cfg(feature = "debug")]
            print_debug!("recreated {}pipeline{}", magenta(), none());

            self.deferred_renderer = new_deferred_renderer;
            self.ambient_light_renderer = new_ambient_light_renderer;
            self.directional_light_renderer = new_directional_light_renderer;
            self.point_light_renderer = new_point_light_renderer;
            self.dimensions = new_dimensions;
            self.swapchain = new_swapchain;
            self.color_buffer = new_color_buffer;
            self.normal_buffer = new_normal_buffer;
            self.depth_buffer = new_depth_buffer;
            self.framebuffers = new_framebuffers;
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

        builder.begin_render_pass(self.framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap();

        self.current_frame = Some(CurrentFrame::new(builder, image_num, acquire_future));
    }

    pub fn lighting_pass(&mut self) {
        if let Some(current_frame) = &mut self.current_frame {
            current_frame.builder.next_subpass(SubpassContents::Inline).unwrap();
        }
    }

    pub fn render_geomitry(&mut self, camera: &Camera, vertex_buffer: VertexBuffer, textures: &Vec<Texture>, transform: &Transform) {
        if let Some(current_frame) = &mut self.current_frame {
            self.deferred_renderer.render_geometry(&camera, &mut current_frame.builder, vertex_buffer, textures, transform);
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
