mod geometry;
mod entity;
mod water;
mod ambient;
mod directional;
mod point;
mod water_light;
mod overlay;
mod rectangle;
mod sprite;
#[cfg(feature = "debug")]
mod buffer;

use std::sync::Arc;
use vulkano::device::Queue;
use vulkano::image::SwapchainImage;
use vulkano::ordered_passes_renderpass;
use vulkano::{device::Device, render_pass::RenderPass};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::buffer::{ CpuAccessibleBuffer, BufferUsage };
use vulkano::render_pass::Subpass;
use vulkano::format::Format;
use vulkano::sync::{ now, GpuFuture };
use winit::window::Window;

use crate::loaders::TextureLoader;
use crate::{types::maths::*, graphics::ImageBuffer};

use super::{ DeferredRenderTarget, Renderer, Camera, GeometryRenderer as GeometryRendererTrait, EntityRenderer as EntityRendererTrait, RenderSettings };
use crate::graphics::{ Texture, ModelVertexBuffer, WaterVertexBuffer, Color, ScreenVertexBuffer, ScreenVertex };

use self::geometry::GeometryRenderer;
use self::entity::EntityRenderer;
use self::water::WaterRenderer;
use self::ambient::AmbientLightRenderer;
use self::directional::DirectionalLightRenderer;
use self::point::PointLightRenderer;
use self::water_light::WaterLightRenderer;
use self::rectangle::RectangleRenderer;
use self::overlay::OverlayRenderer;
use self::sprite::SpriteRenderer;
#[cfg(feature = "debug")]
use self::buffer::BufferRenderer;

#[derive(PartialEq)]
pub enum DeferredSubrenderer {
    Geometry,
    Entity,
    PointLight,
}

pub struct DeferredRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    water_renderer: WaterRenderer,
    ambient_light_renderer: AmbientLightRenderer,
    directional_light_renderer: DirectionalLightRenderer,
    point_light_renderer: PointLightRenderer,
    water_light_renderer: WaterLightRenderer,
    overlay_renderer: OverlayRenderer,
    rectangle_renderer: RectangleRenderer,
    sprite_renderer: SpriteRenderer,
    #[cfg(feature = "debug")]
    buffer_renderer: BufferRenderer,
    screen_vertex_buffer: ScreenVertexBuffer,
    billboard_vertex_buffer: ScreenVertexBuffer,
    font_map: Texture,
    dimensions: [u32; 2],
}

impl DeferredRenderer {

    pub fn new(device: Arc<Device>, queue: Arc<Queue>, swapchain_format: Format, viewport: Viewport, dimensions: [u32; 2], texture_loader: &mut TextureLoader) -> Self {

        let render_pass = ordered_passes_renderpass!(device.clone(),
            attachments: {
                output: {
                    load: Clear,
                    store: Store,
                    format: swapchain_format,
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
                    format: Format::R8G8B8A8_SRGB,
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
                    color: [diffuse, normal, water],
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

        let geometry_subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_subpass = Subpass::from(render_pass.clone(), 1).unwrap();

        let geometry_renderer = GeometryRenderer::new(device.clone(), geometry_subpass.clone(), viewport.clone());
        let entity_renderer = EntityRenderer::new(device.clone(), geometry_subpass.clone(), viewport.clone());
        let water_renderer = WaterRenderer::new(device.clone(), geometry_subpass.clone(), viewport.clone());
        let ambient_light_renderer = AmbientLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let directional_light_renderer = DirectionalLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let point_light_renderer = PointLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let water_light_renderer = WaterLightRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let overlay_renderer = OverlayRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let rectangle_renderer = RectangleRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        let sprite_renderer = SpriteRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        let buffer_renderer = BufferRenderer::new(device.clone(), lighting_subpass.clone(), viewport.clone());

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

        let mut texture_future = now(device.clone()).boxed();
        let font_map = texture_loader.get("assets/font.png", &mut texture_future).unwrap();

        texture_future.flush().unwrap();
        texture_future.cleanup_finished();

        Self {
            device,
            queue,
            render_pass,
            geometry_renderer,
            entity_renderer,
            water_renderer,
            ambient_light_renderer,
            directional_light_renderer,
            point_light_renderer,
            water_light_renderer,
            overlay_renderer,
            rectangle_renderer,
            sprite_renderer,
            #[cfg(feature = "debug")]
            buffer_renderer,
            screen_vertex_buffer,
            billboard_vertex_buffer,
            font_map,
            dimensions,
        }
    }

    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2]) {

        let geometry_subpass = Subpass::from(self.render_pass.clone(), 0).unwrap();
        let lighting_subpass = Subpass::from(self.render_pass.clone(), 1).unwrap();

        self.geometry_renderer.recreate_pipeline(self.device.clone(), geometry_subpass.clone(), viewport.clone(), false); // set wireframe dynamically
        self.entity_renderer.recreate_pipeline(self.device.clone(), geometry_subpass.clone(), viewport.clone());
        self.water_renderer.recreate_pipeline(self.device.clone(), geometry_subpass.clone(), viewport.clone());
        self.ambient_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.directional_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.point_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.water_light_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.overlay_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.rectangle_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.sprite_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        self.buffer_renderer.recreate_pipeline(self.device.clone(), lighting_subpass.clone(), viewport.clone());
        self.dimensions = dimensions;
    }

    pub fn create_render_target(&self, swapchain_image: Arc<SwapchainImage<Window>>) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(self.device.clone(), self.queue.clone(), self.render_pass.clone(), swapchain_image, self.dimensions)
    }

    pub fn render_water(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: WaterVertexBuffer, day_timer: f32) {
        render_target.unbind_subrenderer();
        self.water_renderer.render(render_target, camera, vertex_buffer, day_timer);
    }

    pub fn ambient_light(&self, render_target: &mut <Self as Renderer>::Target, color: Color) {
        render_target.unbind_subrenderer();
        self.ambient_light_renderer.render(render_target, self.screen_vertex_buffer.clone(), color);
    }

    pub fn directional_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, light_image: ImageBuffer, light_matrix: Matrix4<f32>, direction: Vector3<f32>, color: Color, intensity: f32) {
        render_target.unbind_subrenderer();
        self.directional_light_renderer.render(render_target, camera, light_image, light_matrix, self.screen_vertex_buffer.clone(), direction, color, intensity);
    }

    pub fn point_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, position: Vector3<f32>, color: Color, range: f32) {

        if render_target.bind_subrenderer(DeferredSubrenderer::PointLight) {
            self.point_light_renderer.bind_pipeline(render_target, camera, self.billboard_vertex_buffer.clone());
        }

        self.point_light_renderer.render(render_target, camera, position, color, range);
    }

    pub fn water_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, water_level: f32) {
        render_target.unbind_subrenderer();
        self.water_light_renderer.render(render_target, camera, self.screen_vertex_buffer.clone(), water_level);
    }

    pub fn overlay_interface(&self, render_target: &mut <Self as Renderer>::Target, interface_image: ImageBuffer) {
        render_target.unbind_subrenderer();
        self.overlay_renderer.render(render_target, interface_image, self.screen_vertex_buffer.clone());
    }

    pub fn render_sprite(&self, render_target: &mut <Self as Renderer>::Target, texture: Texture, position: Vector2<f32>, size: Vector2<f32>, color: Color) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);
        self.sprite_renderer.render_indexed(render_target, texture, window_size, position, size, color, 1, 0, true);
    }

    pub fn render_text(&self, render_target: &mut <Self as Renderer>::Target, text: &str, mut position: Vector2<f32>, color: Color, font_size: f32) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);

        for character in text.as_bytes() {
            let index = (*character as usize).saturating_sub(31);
            self.sprite_renderer.render_indexed(render_target, self.font_map.clone(), window_size, position, Vector2::new(font_size, font_size), color, 10, index, true);
            position.x += font_size / 2.0;
        }
    }

    pub fn render_rectangle(&self, render_target: &mut <Self as Renderer>::Target, position: Vector2<f32>, size: Vector2<f32>, color: Color) {
        let window_size = Vector2::new(self.dimensions[0] as usize, self.dimensions[1] as usize);
        self.rectangle_renderer.render(render_target, window_size, position, size, color);
    }

    pub fn render_bar(&self, render_target: &mut <Self as Renderer>::Target, position: Vector2<f32>, color: Color, maximum: f32, current: f32) {
        const BAR_SIZE: f32 = 70.0;
        let offset = Vector2::new(BAR_SIZE / 2.0, 0.0);
        self.render_rectangle(render_target, position - offset, Vector2::new(BAR_SIZE, 5.0), Color::monochrome(40));
        self.render_rectangle(render_target, position - offset, Vector2::new((BAR_SIZE / maximum) * current, 5.0), color);
    }

    #[cfg(feature = "debug")]
    pub fn overlay_buffers(&self, render_target: &mut <Self as Renderer>::Target, light_image: ImageBuffer, picker_image: ImageBuffer, render_settings: &RenderSettings) {
        render_target.unbind_subrenderer();
        self.buffer_renderer.render(render_target, light_image, picker_image, self.screen_vertex_buffer.clone(), render_settings);
    }
}

impl Renderer for DeferredRenderer {
    type Target = DeferredRenderTarget;
}

impl GeometryRendererTrait for DeferredRenderer {

    fn render_geometry(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>)
        where Self: Renderer
    {

        if render_target.bind_subrenderer(DeferredSubrenderer::Geometry) {
            self.geometry_renderer.bind_pipeline(render_target, camera);
        }

        self.geometry_renderer.render(render_target, camera, vertex_buffer.clone(), textures, world_matrix);
    }
}

impl EntityRendererTrait for DeferredRenderer {

    fn render_entity(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, scale: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>, _entity_id: usize)
        where Self: Renderer
    {

        if render_target.bind_subrenderer(DeferredSubrenderer::Entity) {
            self.entity_renderer.bind_pipeline(render_target, camera);
        }

        self.entity_renderer.render(render_target, camera, texture, position, origin, scale, cell_count, cell_position);
    }
}
