mod settings;
mod picker;
mod deferred;
mod shadow;
mod lighting;
mod sprite;
mod interface;
#[cfg(feature = "debug")]
mod debug;

use derive_new::new;

use cgmath::{ Vector4, Vector3, Vector2, Matrix4 };
use vulkano::device::physical::PhysicalDevice;
use vulkano::command_buffer::{ AutoCommandBufferBuilder, PrimaryCommandBuffer, CommandBufferUsage, SubpassContents };
use vulkano::device::{ Device, Queue };
use vulkano::swapchain::{ AcquireError, Swapchain, SwapchainCreationError, SwapchainAcquireFuture, Surface, ColorSpace, PresentMode, acquire_next_image };
use vulkano::image::{ ImageUsage, SwapchainImage, ImageAccess, SampleCount };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{ Framebuffer, RenderPass, Subpass };
use vulkano::buffer::BufferUsage;
use vulkano::format::{ Format, ClearValue };
use vulkano::sync::{ FlushError, GpuFuture, now, FenceSignalFuture };
use vulkano::ordered_passes_renderpass;
use winit::window::Window;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::*;
use crate::loaders::TextureLoader;
use crate::types::map::model::Node;

use self::picker::PickerRenderer;
use self::deferred::*;
use self::shadow::*;
use self::lighting::*;
use self::sprite::DynamicSpriteRenderer;
use self::interface::*;
#[cfg(feature = "debug")]
use self::debug::*;

pub use self::settings::RenderSettings;

#[cfg(feature = "debug")]
const MARKER_SIZE: f32 = 1.25;

#[derive(new)]
struct CurrentFrame {
    pub builder: CommandBuilder,
    pub shadow_builder: Option<CommandBuilder>,
    pub picker_builder: Option<CommandBuilder>,
    pub shadow_frame_end: Option<Box<dyn GpuFuture>>,
    pub image_num: usize,
    pub swapchain_future: SwapchainAcquireFuture<Window>,
    pub render_info: RenderInfo,
}

pub struct Renderer {
    queue: Arc<Queue>,
    device: Arc<Device>,
    picker_renderer: PickerRenderer,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    water_renderer: WaterRenderer,
    #[cfg(feature = "debug")]
    area_renderer: AreaRenderer,
    ambient_light_renderer: AmbientLightRenderer,
    directional_light_renderer: DirectionalLightRenderer,
    water_light_renderer: WaterLightRenderer,
    point_light_renderer: PointLightRenderer,
    sprite_renderer: SpriteRenderer,
    rectangle_renderer: RectangleRenderer,
    interface_renderer: InterfaceRenderer,
    dynamic_sprite_renderer: DynamicSpriteRenderer,
    #[cfg(feature = "debug")]
    debug_renderer: DebugRenderer,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    diffuse_buffer: ImageBuffer,
    normal_buffer: ImageBuffer,
    water_buffer: ImageBuffer,
    interface_buffer: ImageBuffer,
    depth_buffer: ImageBuffer,
    present_mode: PresentMode,
    screen_vertex_buffer: ScreenVertexBuffer,
    billboard_vertex_buffer: ScreenVertexBuffer,
    current_frame: Option<CurrentFrame>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    picker_fence: Option<FenceSignalFuture<Box<dyn GpuFuture>>>,
    recreate_swapchain: bool,
    window_size: Vector2<usize>,
    font_map: Texture,
    #[cfg(feature = "debug")]
    debug_icon_texture: Texture,
    checked_box_texture: Texture,
    unchecked_box_texture: Texture,
    expanded_arrow_texture: Texture,
    collapsed_arrow_texture: Texture,

    shadow_framebuffers: Vec<Arc<Framebuffer>>,
    directional_shadow_maps: Vec<ImageBuffer>,
    geometry_shadow_renderer: GeometryShadowRenderer,

    picker_render_pass: Arc<RenderPass>,
    picker_framebuffers: Vec<Arc<Framebuffer>>,
    picker_images: Vec<ImageBuffer>,
    picker_buffers: Vec<Arc<CpuAccessibleBuffer<[u32]>>>,

    last_image_num: usize,
}

impl Renderer {

    pub fn new(physical_device: &PhysicalDevice, device: Arc<Device>, queue: Arc<Queue>, surface: Arc<Surface<Window>>, texture_loader: &mut TextureLoader) -> Self {

        let mut texture_future = now(device.clone()).boxed();

        let capabilities = surface.capabilities(*physical_device).expect("failed to get surface capabilities");
        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let format = capabilities.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        let window_size = Vector2::new(dimensions[0] as usize, dimensions[1] as usize);
        let present_mode = PresentMode::Fifo;

        let (swapchain, images) = Swapchain::start(device.clone(), surface)
            .num_images(capabilities.min_image_count)
            .format(format)
            .dimensions(dimensions)
            .usage(ImageUsage::color_attachment())
            .sharing_mode(&queue)
            .composite_alpha(composite_alpha)
            .color_space(ColorSpace::SrgbNonLinear)
            .present_mode(present_mode)
            .build()
            .expect("failed to create swapchain");

        #[cfg(feature = "debug")]
        print_debug!("created {}swapchain{}", MAGENTA, NONE);

        let render_pass = ordered_passes_renderpass!(device.clone(),
            attachments: {
                output: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                },
                diffuse: {
                    load: Clear,
                    store: Store,
                    format: Format::R32G32B32A32_SFLOAT,
                    samples: 4,
                },
                normal: {
                    load: Clear,
                    store: Store,
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 4,
                },
                water: {
                    load: Clear,
                    store: Store,
                    format: Format::R8_SRGB,
                    samples: 4,
                },
                interface: {
                    load: DontCare,
                    store: Store,
                    format: Format::R32G32B32A32_SFLOAT,
                    samples: 4,
                },
                depth: {
                    load: Clear,
                    store: Store,
                    format: Format::D32_SFLOAT,
                    samples: 4,
                }
            },
            passes: [
                {
                    color: [diffuse, normal, water, interface],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [output],
                    depth_stencil: {},
                    input: [diffuse, normal, water, depth]
                }
            ]
        )
        .unwrap();

        let shadow_render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                depth: {
                    load: Clear,
                    store: Store,
                    format: Format::D32_SFLOAT,
                    samples: 1,
                }
            },
            pass: {
                color: [],
                depth_stencil: {depth}
            }
        )
        .unwrap();

        let picker_render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: Format::R32_UINT,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        )
        .unwrap();

        let deferred_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        #[cfg(feature = "debug")]
        print_debug!("created {}render pass{}", MAGENTA, NONE);

        let (framebuffers, diffuse_buffer, normal_buffer, water_buffer, interface_buffer, depth_buffer, viewport) = Self::window_size_dependent_setup(device.clone(), &images, render_pass.clone());
        let (shadow_framebuffers, directional_shadow_maps) = Self::window_size_dependent_shadow_setup(device.clone(), shadow_render_pass.clone(), images.len());
        let (picker_framebuffers, picker_images, picker_buffers) = Self::window_size_dependent_picker_setup(device.clone(), &images, picker_render_pass.clone());

        #[cfg(feature = "debug")]
        print_debug!("created {}pipeline{}", MAGENTA, NONE);
        
        let shadow_viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [8096.0, 8096.0],
            depth_range: 0.0..1.0,
        };

        let picker_renderer = PickerRenderer::new(device.clone(), picker_render_pass.clone().first_subpass(), viewport.clone());
        let geometry_renderer = GeometryRenderer::new(device.clone(), deferred_subpass.clone(), viewport.clone());
        let entity_renderer = EntityRenderer::new(device.clone(), deferred_subpass.clone(), viewport.clone());
        let water_renderer = WaterRenderer::new(device.clone(), deferred_subpass.clone(), viewport.clone());
        let rectangle_renderer = RectangleRenderer::new(device.clone(), deferred_subpass.clone(), viewport.clone());
        let sprite_renderer = SpriteRenderer::new(device.clone(), deferred_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        let area_renderer = AreaRenderer::new(device.clone(), deferred_subpass, viewport.clone());
        let ambient_light_renderer = AmbientLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let directional_light_renderer = DirectionalLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let water_light_renderer = WaterLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let point_light_renderer = PointLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let interface_renderer = InterfaceRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let dynamic_sprite_renderer = DynamicSpriteRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        let debug_renderer = DebugRenderer::new(device.clone(), lighting_subpass, viewport, texture_loader, &mut texture_future);
        let geometry_shadow_renderer = GeometryShadowRenderer::new(device.clone(), shadow_render_pass.first_subpass(), shadow_viewport);

        #[cfg(feature = "debug")]
        print_debug!("created {}renderers{}", MAGENTA, NONE);

        let vertices = vec![ScreenVertex::new(Vector2::new(-1.0, -1.0)), ScreenVertex::new(Vector2::new(-1.0, 3.0)), ScreenVertex::new(Vector2::new(3.0, -1.0))];
        let screen_vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

        let vertices = vec![
            ScreenVertex::new(Vector2::new(0.0, 0.0)),
            ScreenVertex::new(Vector2::new(0.0, 1.0)),
            ScreenVertex::new(Vector2::new(1.0, 0.0)),
            ScreenVertex::new(Vector2::new(1.0, 0.0)),
            ScreenVertex::new(Vector2::new(0.0, 1.0)),
            ScreenVertex::new(Vector2::new(1.0, 1.0))
        ];
        let billboard_vertex_buffer = CpuAccessibleBuffer::from_iter(device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();

        let font_map = texture_loader.get("assets/font.png", &mut texture_future).unwrap();
        #[cfg(feature = "debug")]
        let debug_icon_texture = texture_loader.get("assets/debug_icon.png", &mut texture_future).unwrap();
        let checked_box_texture = texture_loader.get("assets/checked_box.png", &mut texture_future).unwrap();
        let unchecked_box_texture = texture_loader.get("assets/unchecked_box.png", &mut texture_future).unwrap();
        let expanded_arrow_texture = texture_loader.get("assets/expanded_arrow.png", &mut texture_future).unwrap();
        let collapsed_arrow_texture = texture_loader.get("assets/collapsed_arrow.png", &mut texture_future).unwrap();

        texture_future.flush().unwrap();
        texture_future.cleanup_finished();

        let previous_frame_end = now(device.clone()).boxed().into();
        let picker_fence = None;
        let current_frame = None;
        let recreate_swapchain = false;

        let last_image_num = 0;

        Self {
            queue,
            device,
            picker_renderer,
            geometry_renderer,
            entity_renderer,
            water_renderer,
            #[cfg(feature = "debug")]
            area_renderer,
            ambient_light_renderer,
            directional_light_renderer,
            water_light_renderer,
            point_light_renderer,
            sprite_renderer,
            rectangle_renderer,
            interface_renderer,
            dynamic_sprite_renderer,
            #[cfg(feature = "debug")]
            debug_renderer,
            swapchain,
            render_pass,
            framebuffers,
            diffuse_buffer,
            normal_buffer,
            water_buffer,
            interface_buffer,
            depth_buffer,
            present_mode,
            screen_vertex_buffer,
            billboard_vertex_buffer,
            current_frame,
            previous_frame_end,
            picker_fence,
            recreate_swapchain,
            window_size,
            font_map,
            #[cfg(feature = "debug")]
            debug_icon_texture,
            checked_box_texture,
            unchecked_box_texture,
            expanded_arrow_texture,
            collapsed_arrow_texture,

            geometry_shadow_renderer,
            shadow_framebuffers,
            directional_shadow_maps,

            picker_render_pass,
            picker_framebuffers,
            picker_images,
            picker_buffers,

            last_image_num,
        }
    }

    fn window_size_dependent_setup(device: Arc<Device>, images: &[Arc<SwapchainImage<Window>>], render_pass: Arc<RenderPass>) -> (Vec<Arc<Framebuffer>>, ImageBuffer, ImageBuffer, ImageBuffer, ImageBuffer, ImageBuffer, Viewport) {

        let dimensions = images[0].dimensions().width_height();

        let color_buffer_usage = ImageUsage {
            sampled: true,
            color_attachment: true,
            input_attachment: true, 
            ..ImageUsage::none()
        };

        let interface_image_usage = ImageUsage {
            sampled: true,
            transfer_destination: true,
            color_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        };

        let depth_buffer_usage = ImageUsage {
            sampled: true,
            depth_stencil_attachment: true,
            input_attachment: true,
            ..ImageUsage::none()
        };

        let diffuse_buffer = ImageView::new(Arc::new(AttachmentImage::multisampled_with_usage(device.clone(), dimensions, SampleCount::Sample4, Format::R32G32B32A32_SFLOAT, color_buffer_usage).unwrap())).unwrap();
        let normal_buffer = ImageView::new(Arc::new(AttachmentImage::multisampled_with_usage(device.clone(), dimensions, SampleCount::Sample4, Format::R16G16B16A16_SFLOAT, color_buffer_usage).unwrap())).unwrap();
        let water_buffer = ImageView::new(Arc::new(AttachmentImage::multisampled_with_usage(device.clone(), dimensions, SampleCount::Sample4, Format::R8_SRGB, color_buffer_usage).unwrap())).unwrap();
        let interface_buffer = ImageView::new(Arc::new(AttachmentImage::multisampled_with_usage(device.clone(), dimensions, SampleCount::Sample4, Format::R32G32B32A32_SFLOAT, interface_image_usage).unwrap())).unwrap();
        let depth_buffer = ImageView::new(Arc::new(AttachmentImage::multisampled_with_usage(device, dimensions, SampleCount::Sample4, Format::D32_SFLOAT, depth_buffer_usage).unwrap())).unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let image_buffer = ImageView::new(image.clone()).unwrap();

                Framebuffer::start(render_pass.clone())
                    .add(image_buffer).unwrap()
                    .add(diffuse_buffer.clone()).unwrap()
                    .add(normal_buffer.clone()).unwrap()
                    .add(water_buffer.clone()).unwrap()
                    .add(interface_buffer.clone()).unwrap()
                    .add(depth_buffer.clone()).unwrap()
                    .build().unwrap()
            })
            .collect::<Vec<_>>();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };

        (framebuffers, diffuse_buffer, normal_buffer, water_buffer, interface_buffer, depth_buffer, viewport)
    }

    fn window_size_dependent_shadow_setup(device: Arc<Device>, render_pass: Arc<RenderPass>, framebuffer_count: usize) -> (Vec<Arc<Framebuffer>>, Vec<ImageBuffer>) {
        
        let dimensions = [8096, 8096];
        let buffer_usage = ImageUsage {
            sampled: true,
            depth_stencil_attachment: true,
            ..ImageUsage::none()
        };

        let directional_shadow_maps: Vec<ImageBuffer> = (0..framebuffer_count).map(|_| ImageView::new(Arc::new(AttachmentImage::with_usage(device.clone(), dimensions, Format::D32_SFLOAT, buffer_usage).unwrap())).unwrap()).collect();

        let framebuffers = directional_shadow_maps.iter().map(|image| {
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
        }).collect();

        (framebuffers, directional_shadow_maps)
    }

    fn window_size_dependent_picker_setup(device: Arc<Device>, images: &[Arc<SwapchainImage<Window>>], render_pass: Arc<RenderPass>) -> (Vec<Arc<Framebuffer>>, Vec<ImageBuffer>, Vec<Arc<CpuAccessibleBuffer<[u32]>>>) {
        
        let dimensions = images[0].dimensions().width_height();
        let image_usage = ImageUsage {
            sampled: true,
            transfer_source: true,
            color_attachment: true,
            ..ImageUsage::none()
        };

        let picker_images: Vec<ImageBuffer> = (0..images.len())
            .map(|_| ImageView::new(Arc::new(AttachmentImage::with_usage(device.clone(), dimensions, Format::R32_UINT, image_usage).unwrap())).unwrap())
            .collect();

        let framebuffers = picker_images.iter().map(|image| {
            Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()
        }).collect();

        let picker_buffers: Vec<Arc<CpuAccessibleBuffer<[u32]>>> = (0..images.len())
            .map(|_| unsafe { CpuAccessibleBuffer::uninitialized_array(device.clone(), dimensions[0] as u64 * dimensions[1] as u64, BufferUsage::transfer_destination(), false).unwrap() })
            .collect();

        (framebuffers, picker_images, picker_buffers)
    }

    pub fn invalidate_swapchain(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn set_frame_limit(&mut self, capped: bool) {
        self.present_mode = match capped {
            true => PresentMode::Fifo,
            false => PresentMode::Mailbox,
        };
        self.invalidate_swapchain();
    }

    pub fn get_window_size(&mut self) -> Vector2<usize> {
        self.window_size
    }

    pub fn get_picker_buffer(&mut self) -> &CpuAccessibleBuffer<[u32]> {
        if let Some(fence) = self.picker_fence.take() {
            fence.wait(None).unwrap()
        }
        &self.picker_buffers[self.last_image_num]
    }

    pub fn update(&mut self, delta_time: f32) {
        self.water_renderer.update(delta_time);
    }

    pub fn start_frame(&mut self, surface: &Arc<Surface<Window>>, clear_interface: bool) {

        self.previous_frame_end
            .as_mut()
            .unwrap()
            .cleanup_finished();

        if self.recreate_swapchain {

            #[cfg(feature = "debug")]
            let timer = Timer::new("recreating swapchain");

            let new_dimensions: [u32; 2] = surface.window().inner_size().into();
            let new_window_size = Vector2::new(new_dimensions[0] as usize, new_dimensions[1] as usize);
            let swapchain_result  = self.swapchain
                .recreate()
                .dimensions(new_dimensions)
                .present_mode(self.present_mode)
                .build();

            let (new_swapchain, new_images) =  match swapchain_result {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            let (new_framebuffers, new_diffuse_buffer, new_normal_buffer, new_water_buffer, new_interface_buffer, new_depth_buffer, new_viewport) = Self::window_size_dependent_setup(self.device.clone(), &new_images, self.render_pass.clone());
            let (new_picker_framebuffers, new_picker_images, new_picker_buffers) = Self::window_size_dependent_picker_setup(self.device.clone(), &new_images, self.picker_render_pass.clone());

            let deferred_subpass = Subpass::from(self.render_pass.clone(), 0).unwrap();
            let lighting_subpass = Subpass::from(self.render_pass.clone(), 1).unwrap();

            self.picker_renderer.recreate_pipeline(self.device.clone(), self.picker_render_pass.clone().first_subpass(), new_viewport.clone());
            self.geometry_renderer.recreate_pipeline(self.device.clone(), deferred_subpass.clone(), new_viewport.clone());
            self.entity_renderer.recreate_pipeline(self.device.clone(), deferred_subpass.clone(), new_viewport.clone());
            self.water_renderer.recreate_pipeline(self.device.clone(), deferred_subpass.clone(), new_viewport.clone());
            self.rectangle_renderer.recreate_pipeline(self.device.clone(), deferred_subpass.clone(), new_viewport.clone());
            self.sprite_renderer.recreate_pipeline(self.device.clone(), deferred_subpass.clone(), new_viewport.clone());
            #[cfg(feature = "debug")]
            self.area_renderer.recreate_pipeline(self.device.clone(), deferred_subpass, new_viewport.clone());
            self.ambient_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), new_viewport.clone());
            self.directional_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), new_viewport.clone());
            self.water_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), new_viewport.clone());
            self.point_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), new_viewport.clone());
            self.interface_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), new_viewport.clone());
            self.dynamic_sprite_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), new_viewport.clone());
            #[cfg(feature = "debug")]
            self.debug_renderer.recreate_pipeline(self.device.clone(), lighting_subpass, new_viewport);

            self.swapchain = new_swapchain;
            self.diffuse_buffer = new_diffuse_buffer;
            self.normal_buffer = new_normal_buffer;
            self.water_buffer = new_water_buffer;
            self.interface_buffer = new_interface_buffer;
            self.depth_buffer = new_depth_buffer;
            self.framebuffers = new_framebuffers;
            self.window_size = new_window_size;
            self.recreate_swapchain = false;

            self.picker_framebuffers = new_picker_framebuffers;
            self.picker_images = new_picker_images;
            self.picker_buffers = new_picker_buffers;

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

        let clear_values = vec![[0.0, 0.0, 0.0, 1.0].into(), [0.0, 0.0, 0.0, 1.0].into(), [0.0, 0.0, 0.0, 1.0].into(), [0f32].into(), ClearValue::None, 1f32.into()];
        let mut builder = AutoCommandBufferBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();

        if clear_interface {
            builder.clear_color_image(self.interface_buffer.image().clone(), [0.0, 0.0, 0.0, 0.0].into()).unwrap();
        }

        builder.begin_render_pass(self.framebuffers[image_num].clone(), SubpassContents::Inline, clear_values).unwrap();

        let shadow_clear_values: Vec<ClearValue> = vec![1f32.into()];
        let mut shadow_builder = CommandBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();
        shadow_builder.begin_render_pass(self.shadow_framebuffers[image_num].clone(), SubpassContents::Inline, shadow_clear_values).unwrap();
        
        let picker_clear_values: Vec<ClearValue> = vec![[0u32].into()];
        let mut picker_builder = CommandBuilder::primary(self.device.clone(), self.queue.family(), CommandBufferUsage::OneTimeSubmit).unwrap();
        picker_builder.begin_render_pass(self.picker_framebuffers[image_num].clone(), SubpassContents::Inline, picker_clear_values).unwrap();

        self.current_frame = Some(CurrentFrame::new(builder, shadow_builder.into(), picker_builder.into(), None, image_num, acquire_future, RenderInfo::default()));
    }

    pub fn geometry_pass(&mut self) {

        let current_frame = self.current_frame.as_mut().unwrap(); 
        let mut picker_builder = current_frame.picker_builder.take().unwrap();

        picker_builder.end_render_pass().unwrap();
        picker_builder.copy_image_to_buffer(self.picker_images[current_frame.image_num].image().clone(), self.picker_buffers[current_frame.image_num].clone()).unwrap();

        let future = picker_builder
            .build()
            .unwrap()
            .execute(self.queue.clone())
            .unwrap()
            .boxed()
            .then_signal_fence_and_flush()
            .unwrap();

        self.picker_fence = future.into();
    }

    pub fn lighting_pass(&mut self) {
        let current_frame = self.current_frame.as_mut().unwrap();

        current_frame.builder.next_subpass(SubpassContents::Inline).unwrap();

        let mut shadow_builder = current_frame.shadow_builder.take().unwrap();
        shadow_builder.end_render_pass().unwrap();
        let shadow_command_buffer = shadow_builder.build().unwrap();

        let shadow_future = self.previous_frame_end
            .take()
            .unwrap()
            .then_execute(self.queue.clone(), shadow_command_buffer).unwrap();

        shadow_future.flush().unwrap();
        current_frame.shadow_frame_end = shadow_future.boxed().into();
    }

    pub fn render_tiles(&mut self, camera: &dyn Camera, vertex_buffer: TileVertexBuffer) {
        let current_frame = self.current_frame.as_mut().unwrap();
        let picker_builder = current_frame.picker_builder.as_mut().unwrap();
        self.picker_renderer.render(camera, picker_builder, vertex_buffer);
    }

    pub fn render_geomitry(&mut self, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, transform: &Transform) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.geometry_renderer.render(camera, &mut current_frame.builder, vertex_buffer.clone(), textures, transform);

        let shadow_builder = current_frame.shadow_builder.as_mut().unwrap();
        self.geometry_shadow_renderer.render(camera, shadow_builder, vertex_buffer, textures, transform);
    }

    pub fn render_node(&mut self, camera: &dyn Camera, node: &Node, transform: &Transform, client_tick: u32) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.geometry_renderer.render_node(camera, &mut current_frame.builder, node, transform, client_tick);

        current_frame.render_info.object_count += 1;

        let shadow_builder = current_frame.shadow_builder.as_mut().unwrap();
        self.geometry_shadow_renderer.render_node(camera, shadow_builder, node, transform, client_tick);
    }

    pub fn render_entity(&mut self, camera: &dyn Camera, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>) {
        let current_frame = self.current_frame.as_mut().unwrap();

        current_frame.render_info.entity_count += 1;

        self.entity_renderer.render(camera, &mut current_frame.builder, texture, position, origin, size, cell_count, cell_position);
    }

    pub fn render_water(&mut self, camera: &dyn Camera, vertex_buffer: WaterVertexBuffer) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.water_renderer.render(camera, &mut current_frame.builder, vertex_buffer);
    }

    pub fn ambient_light(&mut self, color: Color) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.ambient_light_renderer.render(&mut current_frame.builder, self.diffuse_buffer.clone(), self.normal_buffer.clone(), self.screen_vertex_buffer.clone(), color);
    }

    pub fn directional_light(&mut self, camera: &dyn Camera, direction: Vector3<f32>, color: Color) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.directional_light_renderer.render(&mut current_frame.builder, camera, self.diffuse_buffer.clone(), self.normal_buffer.clone(), self.depth_buffer.clone(), self.directional_shadow_maps[current_frame.image_num].clone(), self.screen_vertex_buffer.clone(), direction, color);
    }

    pub fn water_light(&mut self, camera: &dyn Camera, water_level: f32) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.water_light_renderer.render(&mut current_frame.builder, camera, self.water_buffer.clone(), self.depth_buffer.clone(), self.screen_vertex_buffer.clone(), water_level);
    }

    pub fn point_light(&mut self, camera: &dyn Camera, position: Vector3<f32>, color: Color, range: f32) {
        let current_frame = self.current_frame.as_mut().unwrap();

        current_frame.render_info.point_light_count += 1;

        self.point_light_renderer.render(&mut current_frame.builder, camera, self.diffuse_buffer.clone(), self.normal_buffer.clone(), self.depth_buffer.clone(), self.billboard_vertex_buffer.clone(), position, color, range);
    }

    pub fn render_sprite(&mut self, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, smooth: bool) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.sprite_renderer.render(&mut current_frame.builder, self.window_size, texture, position, size, clip_size, color, smooth);
    }

    pub fn render_sprite_indexed(&mut self, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, column_count: usize, cell_index: usize, smooth: bool) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.sprite_renderer.render_indexed(&mut current_frame.builder, self.window_size, texture, position, size, clip_size, color, column_count, cell_index, smooth);
    }

    pub fn render_rectangle(&mut self, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, corner_radius: Vector4<f32>, color: Color) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.rectangle_renderer.render(&mut current_frame.builder, self.window_size, position, size, clip_size, corner_radius, color);
    }

    pub fn render_checkbox(&mut self, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, checked: bool) {
        match checked {
            true => self.render_sprite(self.checked_box_texture.clone(), position, size, clip_size, color, true),
            false => self.render_sprite(self.unchecked_box_texture.clone(), position, size, clip_size, color, true),
        }
    }

    pub fn render_expand_arrow(&mut self, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color, expanded: bool) {
        match expanded {
            true => self.render_sprite(self.expanded_arrow_texture.clone(), position, size, clip_size, color, true),
            false => self.render_sprite(self.collapsed_arrow_texture.clone(), position, size, clip_size, color, true),
        }
    }

    pub fn render_text(&mut self, text: &str, mut position: Vector2<f32>, clip_size: Vector2<f32>, color: Color, font_size: f32) {
        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.render_sprite_indexed(self.font_map.clone(), position, Vector2::new(font_size, font_size), clip_size, color, 10, index, true);
            position.x += font_size / 2.0;
        }
    }

    pub fn render_dynamic_sprite_direct(&mut self, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, color: Color, smooth: bool) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.dynamic_sprite_renderer.render_direct(&mut current_frame.builder, texture, position, size, color, smooth);
    }

    pub fn render_dynamic_sprite_indexed(&mut self, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, color: Color, column_count: usize, cell_index: usize, smooth: bool) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.dynamic_sprite_renderer.render_indexed(&mut current_frame.builder, self.window_size, texture, position, size, color, column_count, cell_index, smooth);
    }

    pub fn render_dynamic_text(&mut self, text: &str, mut position: Vector2<f32>, color: Color, font_size: f32) {
        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.render_dynamic_sprite_indexed(self.font_map.clone(), position, Vector2::new(font_size, font_size), color, 10, index, true);
            position.x += font_size / 2.0;
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug_icon(&mut self, position: Vector2<f32>, size: Vector2<f32>, clip_size: Vector2<f32>, color: Color) {
        self.render_sprite(self.debug_icon_texture.clone(), position, size, clip_size, color, true);
    }

    #[cfg(feature = "debug")]
    pub fn render_map_tiles(&mut self, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, transform: &Transform) { // remove transform
        let tile_textures = self.debug_renderer.tile_textures.clone();
        self.render_geomitry(camera, vertex_buffer, &tile_textures, transform);
    }

    #[cfg(feature = "debug")]
    pub fn render_pathing(&mut self, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, transform: &Transform) { // remove transform
        let step_textures = self.debug_renderer.step_textures.clone();
        self.render_geomitry(camera, vertex_buffer, &step_textures, transform);
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&mut self, camera: &dyn Camera, transform: &Transform) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.area_renderer.render(&mut current_frame.builder, camera, transform);
    }

    #[cfg(feature = "debug")]
    pub fn marker_hovered(&self, camera: &dyn Camera, position: Vector3<f32>, mouse_position: Vector2<f32>) -> bool {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MARKER_SIZE);

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 {
            return false;
        }

        let (screen_position, screen_size) = camera.screen_position_size(bottom_right_position, top_left_position); // WHY ARE THESE INVERTED ???
        let half_screen = Vector2::new(self.window_size.x as f32 / 2.0, self.window_size.y as f32 / 2.0);
        let mouse_position = Vector2::new(mouse_position.x / half_screen.x, mouse_position.y / half_screen.y);

        mouse_position.x >= screen_position.x && mouse_position.y >= screen_position.y &&
            mouse_position.x <= screen_position.x + screen_size.x && mouse_position.y <= screen_position.y + screen_size.y
    }

    #[cfg(feature = "debug")]
    pub fn render_debug_marker(&mut self, camera: &dyn Camera, icon: Texture, position: Vector3<f32>, color: Color) {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MARKER_SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(bottom_right_position, top_left_position); // WHY ARE THESE INVERTED ???
            self.render_dynamic_sprite_direct(icon, screen_position, screen_size, color, true);
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_object_marker(&mut self, camera: &dyn Camera, position: Vector3<f32>, hovered: bool) {
        let color = match hovered {
            true => Color::rgb(100, 100, 255),
            false => Color::rgb(255, 100, 100),
        };
        self.render_debug_marker(camera, self.debug_renderer.object_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_light_marker(&mut self, camera: &dyn Camera, position: Vector3<f32>, color: Color, hovered: bool) {
        let color = match hovered {
            true => color.invert(),
            false => color,
        };
        self.render_debug_marker(camera, self.debug_renderer.light_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_sound_marker(&mut self, camera: &dyn Camera, position: Vector3<f32>, hovered: bool) {
        let color = match hovered {
            true => Color::monochrome(60),
            false => Color::monochrome(150),
        };
        self.render_debug_marker(camera, self.debug_renderer.sound_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_effect_marker(&mut self, camera: &dyn Camera, position: Vector3<f32>, hovered: bool) {
        let color = match hovered {
            true => Color::rgb(200, 100, 255),
            false => Color::rgb(100, 255, 100),
        };
        self.render_debug_marker(camera, self.debug_renderer.effect_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_particle_marker(&mut self, camera: &dyn Camera, position: Vector3<f32>, hovered: bool) {
        let color = match hovered {
            true => Color::rgb(255, 200, 20),
            false => Color::rgb(255, 20, 20),
        };
        self.render_debug_marker(camera, self.debug_renderer.particle_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_entity_marker(&mut self, camera: &dyn Camera, position: Vector3<f32>, hovered: bool) {
        let color = match hovered {
            true => Color::rgb(255, 160, 255),
            false => Color::rgb(255, 100, 255),
        };
        self.render_debug_marker(camera, self.debug_renderer.entity_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_buffers(&mut self, camera: &dyn Camera, render_settings: &RenderSettings) {

        if let Some(fence) = self.picker_fence.take() {
            fence.wait(None).unwrap()
        }

        let current_frame = self.current_frame.as_mut().unwrap();
        self.debug_renderer.render_buffers(&mut current_frame.builder, camera, self.diffuse_buffer.clone(), self.normal_buffer.clone(), self.water_buffer.clone(), self.depth_buffer.clone(), self.picker_images[current_frame.image_num].clone(), self.screen_vertex_buffer.clone(), render_settings);
    }

    pub fn render_interface(&mut self, render_settings: &RenderSettings) {
        let current_frame = self.current_frame.as_mut().unwrap();
        self.interface_renderer.render(&mut current_frame.builder, self.interface_buffer.clone(), self.screen_vertex_buffer.clone(), render_settings);
    }

    pub fn stop_frame(&mut self) {
        let mut current_frame = self.current_frame.take().unwrap();

        current_frame.builder.end_render_pass().unwrap();

        let command_buffer = current_frame.builder.build().unwrap();
        let future = current_frame.shadow_frame_end
            .take().unwrap()
            .join(current_frame.swapchain_future)
            .then_execute(self.queue.clone(), command_buffer).unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), current_frame.image_num)
            .then_signal_fence_and_flush();

        match future {

            Ok(future) => {
                self.previous_frame_end = future.boxed().into();
            }

            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = now(self.device.clone()).boxed().into();
            }

            Err(error) => {
                println!("Failed to flush future: {:?}", error);
                self.previous_frame_end = now(self.device.clone()).boxed().into();
            }
        }

        self.last_image_num = current_frame.image_num;
    }
}
