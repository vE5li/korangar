#[macro_use]
mod sampler;
mod deferred;
mod lighting;
mod sprite;

use std::sync::Arc;

use cgmath::{ Vector3, Vector2 };

use vulkano::device::physical::PhysicalDevice;
use vulkano::command_buffer::{ AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents };
use vulkano::device::{ Device, Queue };
use vulkano::swapchain::{ AcquireError, Swapchain, SwapchainCreationError, SwapchainAcquireFuture, Surface, acquire_next_image };
use vulkano::image::{ ImageUsage, SwapchainImage };
use vulkano::image::view::ImageView;
use vulkano::image::attachment::AttachmentImage;
use vulkano::pipeline::viewport::Viewport;
use vulkano::render_pass::{ Framebuffer, RenderPass, Subpass };
use vulkano::buffer::BufferUsage;
use vulkano::format::Format;
use vulkano::sync::{ FlushError, GpuFuture, now };

use winit::window::Window;

#[cfg(feature = "debug")]
use debug::*;
use graphics::*;
use managers::TextureManager;

use self::deferred::DeferredRenderer;
use self::lighting::*;
use self::sprite::SpriteRenderer;

#[cfg(feature = "debug")]
const MARKER_SIZE: f32 = 1.25;

struct CurrentFrame {
    pub builder: CommandBuilder,
    pub image_num: usize,
    pub swapchain_future: SwapchainAcquireFuture<Window>,
}

impl CurrentFrame {

    pub fn new(builder: CommandBuilder, image_num: usize, swapchain_future: SwapchainAcquireFuture<Window>) -> Self {
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
    sprite_renderer: SpriteRenderer,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Framebuffers,
    diffuse_buffer: ImageBuffer,
    normal_buffer: ImageBuffer,
    depth_buffer: ImageBuffer,
    screen_vertex_buffer: ScreenVertexBuffer,
    billboard_vertex_buffer: ScreenVertexBuffer,
    current_frame: Option<CurrentFrame>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    recreate_swapchain: bool,
    window_size: Vector2<usize>,
    #[cfg(feature = "debug")]
    object_texture: Texture,
    #[cfg(feature = "debug")]
    light_texture: Texture,
    #[cfg(feature = "debug")]
    sound_texture: Texture,
    #[cfg(feature = "debug")]
    effect_texture: Texture,
    #[cfg(feature = "debug")]
    particle_texture: Texture,
}

impl Renderer {

    pub fn new(physical_device: &PhysicalDevice, device: Arc<Device>, queue: Arc<Queue>, surface: Arc<Surface<Window>>, _texture_manager: &mut TextureManager) -> Self {

        let capabilities = surface.capabilities(*physical_device).expect("failed to get surface capabilities");
        let composite_alpha = capabilities.supported_composite_alpha.iter().next().unwrap();
        let format = capabilities.supported_formats[0].0;
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        let window_size = Vector2::new(dimensions[0] as usize, dimensions[1] as usize);

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
                    diffuse: {
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
                        color: [diffuse, normal],
                        depth_stencil: {depth},
                        input: []
                    },
                    {
                        color: [output],
                        depth_stencil: {},
                        input: [diffuse, normal, depth]
                    }
                ]
            )
            .unwrap(),
        );

        #[cfg(feature = "debug")]
        print_debug!("created {}render pass{}", magenta(), none());

        let (deferred_renderer, ambient_light_renderer, directional_light_renderer, point_light_renderer, sprite_renderer, framebuffers, diffuse_buffer, normal_buffer, depth_buffer) = Self::window_size_dependent_setup(device.clone(), &images, render_pass.clone());

        #[cfg(feature = "debug")]
        print_debug!("created {}pipeline{}", magenta(), none());

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

        let previous_frame_end = Some(now(device.clone()).boxed());

        #[cfg(feature = "debug")]
        let (object_texture, mut future) = _texture_manager.get(String::from("assets/object.png"));
        #[cfg(feature = "debug")]
        future.cleanup_finished();

        #[cfg(feature = "debug")]
        let (light_texture, mut future) = _texture_manager.get(String::from("assets/light.png"));
        #[cfg(feature = "debug")]
        future.cleanup_finished();

        #[cfg(feature = "debug")]
        let (sound_texture, mut future) = _texture_manager.get(String::from("assets/sound.png"));
        #[cfg(feature = "debug")]
        future.cleanup_finished();

        #[cfg(feature = "debug")]
        let (effect_texture, mut future) = _texture_manager.get(String::from("assets/effect.png"));
        #[cfg(feature = "debug")]
        future.cleanup_finished();

        #[cfg(feature = "debug")]
        let (particle_texture, mut future) = _texture_manager.get(String::from("assets/particle.png"));
        #[cfg(feature = "debug")]
        future.cleanup_finished();

        return Self {
            queue: queue,
            device: device,
            deferred_renderer: deferred_renderer,
            ambient_light_renderer: ambient_light_renderer,
            directional_light_renderer: directional_light_renderer,
            point_light_renderer: point_light_renderer,
            sprite_renderer: sprite_renderer,
            swapchain: swapchain,
            render_pass: render_pass,
            framebuffers: framebuffers,
            diffuse_buffer: diffuse_buffer,
            normal_buffer: normal_buffer,
            depth_buffer: depth_buffer,
            screen_vertex_buffer: screen_vertex_buffer,
            billboard_vertex_buffer: billboard_vertex_buffer,
            current_frame: None,
            previous_frame_end: previous_frame_end,
            recreate_swapchain: false,
            window_size: window_size,
            #[cfg(feature = "debug")]
            object_texture: object_texture,
            #[cfg(feature = "debug")]
            light_texture: light_texture,
            #[cfg(feature = "debug")]
            sound_texture: sound_texture,
            #[cfg(feature = "debug")]
            effect_texture: effect_texture,
            #[cfg(feature = "debug")]
            particle_texture: particle_texture,
        }
    }

    fn window_size_dependent_setup(device: Arc<Device>, images: &[Arc<SwapchainImage<Window>>], render_pass: Arc<RenderPass>) -> (DeferredRenderer, AmbientLightRenderer, DirectionalLightRenderer, PointLightRenderer, SpriteRenderer, Framebuffers, ImageBuffer, ImageBuffer, ImageBuffer) {

        let dimensions = images[0].dimensions();

        let diffuse_buffer = ImageView::new(AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::A2B10G10R10_UNORM_PACK32).unwrap()).unwrap();
        let normal_buffer = ImageView::new(AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::R16G16B16A16_SFLOAT).unwrap()).unwrap();
        let depth_buffer = ImageView::new(AttachmentImage::transient_input_attachment(device.clone(), dimensions, Format::D32_SFLOAT).unwrap()).unwrap();

        let framebuffers = images
            .iter()
            .map(|image| {
                let image_buffer = ImageView::new(image.clone()).unwrap();
                let framebuffer = Framebuffer::start(render_pass.clone())
                    .add(image_buffer).unwrap()
                    .add(diffuse_buffer.clone()).unwrap()
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
        let point_light_renderer = PointLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let sprite_renderer = SpriteRenderer::new(device.clone(), lighting_subpass, viewport);

        return (deferred_renderer, ambient_light_renderer, directional_light_renderer, point_light_renderer, sprite_renderer, framebuffers, diffuse_buffer, normal_buffer, depth_buffer);
    }

    pub fn invalidate_swapchain(&mut self) {
        self.recreate_swapchain = true;
    }

    pub fn get_window_size(&mut self) -> Vector2<usize> {
        return self.window_size;
    }

    pub fn start_frame(&mut self, surface: &Arc<Surface<Window>>) {
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {

            #[cfg(feature = "debug")]
            let timer = Timer::new("recreating swapchain");

            let new_dimensions: [u32; 2] = surface.window().inner_size().into();
            let new_window_size = Vector2::new(new_dimensions[0] as usize, new_dimensions[1] as usize);

            let (new_swapchain, new_images) =  match self.swapchain.recreate().dimensions(new_dimensions).build() {
                Ok(r) => r,
                Err(SwapchainCreationError::UnsupportedDimensions) => return,
                Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
            };

            let (new_deferred_renderer, new_ambient_light_renderer, new_directional_light_renderer, new_point_light_renderer, new_sprite_renderer, new_framebuffers, new_color_buffer, new_normal_buffer, new_depth_buffer) = Self::window_size_dependent_setup(self.device.clone(), &new_images, self.render_pass.clone());

            self.deferred_renderer = new_deferred_renderer;
            self.ambient_light_renderer = new_ambient_light_renderer;
            self.directional_light_renderer = new_directional_light_renderer;
            self.point_light_renderer = new_point_light_renderer;
            self.sprite_renderer = new_sprite_renderer;
            self.swapchain = new_swapchain;
            self.diffuse_buffer = new_color_buffer;
            self.normal_buffer = new_normal_buffer;
            self.depth_buffer = new_depth_buffer;
            self.framebuffers = new_framebuffers;
            self.recreate_swapchain = false;
            self.window_size = new_window_size;

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

    pub fn render_geomitry(&mut self, camera: &dyn Camera, vertex_buffer: VertexBuffer, textures: &Vec<Texture>, transform: &Transform) {
        if let Some(current_frame) = &mut self.current_frame {
            self.deferred_renderer.render_geometry(camera, &mut current_frame.builder, vertex_buffer, textures, transform);
        }
    }

    pub fn ambient_light(&mut self, color: Color) {
        if let Some(current_frame) = &mut self.current_frame {
            self.ambient_light_renderer.render(&mut current_frame.builder, self.diffuse_buffer.clone(), self.screen_vertex_buffer.clone(), color);
        }
    }

    pub fn directional_light(&mut self, direction: Vector3<f32>, color: Color) {
        if let Some(current_frame) = &mut self.current_frame {
            self.directional_light_renderer.render(&mut current_frame.builder, self.diffuse_buffer.clone(), self.normal_buffer.clone(), self.screen_vertex_buffer.clone(), direction, color);
        }
    }

    pub fn point_light(&mut self, camera: &dyn Camera, position: Vector3<f32>, color: Color, range: f32) {
        if let Some(current_frame) = &mut self.current_frame {
            self.point_light_renderer.render(&mut current_frame.builder, camera, self.diffuse_buffer.clone(), self.normal_buffer.clone(), self.depth_buffer.clone(), self.billboard_vertex_buffer.clone(), position, color, range);
        }
    }

    pub fn render_sprite_indexed(&mut self, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, color: Color, column_count: usize, cell_index: usize, smooth: bool) {
        if let Some(current_frame) = &mut self.current_frame {
            self.sprite_renderer.render_indexed(&mut current_frame.builder, self.window_size, texture, position, size, color, column_count, cell_index, smooth);
        }
    }

    pub fn render_text(&mut self, font_map: Texture, text: &str, mut position: Vector2<f32>, color: Color, font_size: f32) {
        for character in text.as_bytes() {

            let index = match (*character as usize) < 31 {
                true => 0,
                false => *character as usize - 31,
            };

            self.render_sprite_indexed(font_map.clone(), position, Vector2::new(font_size, font_size), color, 10, index, false);
            position.x += font_size / 2.0;
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug_icon(&mut self, camera: &dyn Camera, icon: Texture, position: Vector3<f32>, color: Color) {

        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MARKER_SIZE);

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 {
            return;
        }

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

        let window_size = Vector2::new(self.window_size.x as f32, self.window_size.y as f32);
        let scaled_position = Vector2::new(screen_position.x * window_size.x, screen_position.y * window_size.y);
        let scaled_size = Vector2::new(screen_size.x * window_size.x, screen_size.y * window_size.y);

        self.render_sprite_indexed(icon, scaled_position, scaled_size, color, 1, 0, true);
    }

    #[cfg(feature = "debug")]
    pub fn render_object_icon(&mut self, camera: &dyn Camera, position: Vector3<f32>) {
        self.render_debug_icon(camera, self.object_texture.clone(), position, Color::new(255, 100, 100));
    }

    #[cfg(feature = "debug")]
    pub fn render_light_icon(&mut self, camera: &dyn Camera, position: Vector3<f32>, color: Color) {
        self.render_debug_icon(camera, self.light_texture.clone(), position, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_sound_icon(&mut self, camera: &dyn Camera, position: Vector3<f32>) {
        self.render_debug_icon(camera, self.sound_texture.clone(), position, Color::new(150, 150, 150));
    }

    #[cfg(feature = "debug")]
    pub fn render_effect_icon(&mut self, camera: &dyn Camera, position: Vector3<f32>) {
        self.render_debug_icon(camera, self.effect_texture.clone(), position, Color::new(100, 255, 100));
    }

    #[cfg(feature = "debug")]
    pub fn render_particle_icon(&mut self, camera: &dyn Camera, position: Vector3<f32>) {
        self.render_debug_icon(camera, self.particle_texture.clone(), position, Color::new(255, 20, 20));
    }

    pub fn stop_frame(&mut self) {
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
