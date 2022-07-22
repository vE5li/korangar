mod geometry;
mod entity;
mod water;
mod ambient;
mod directional;
mod point;
mod water_light;

use std::sync::Arc;
use vulkano::device::Queue;
use vulkano::image::SwapchainImage;
use vulkano::ordered_passes_renderpass;
use vulkano::{device::Device, render_pass::RenderPass};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::buffer::{ CpuAccessibleBuffer, BufferUsage };
use vulkano::render_pass::Subpass;
use vulkano::format::Format;
use winit::window::Window;

use crate::{types::maths::*, graphics::ImageBuffer};

use super::{ DeferredRenderTarget, Renderer, Camera, GeometryRenderer as GeometryRendererTrait, EntityRenderer as EntityRendererTrait };
use crate::graphics::{ Texture, ModelVertexBuffer, WaterVertexBuffer, Color, ScreenVertexBuffer, ScreenVertex };

use self::geometry::GeometryRenderer;
use self::entity::EntityRenderer;
use self::water::WaterRenderer;
use self::ambient::AmbientLightRenderer;
use self::directional::DirectionalLightRenderer;
use self::point::PointLightRenderer;
use self::water_light::WaterLightRenderer;

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
    screen_vertex_buffer: ScreenVertexBuffer,
    billboard_vertex_buffer: ScreenVertexBuffer,
    dimensions: [u32; 2],
}

impl DeferredRenderer {

    pub fn new(device: Arc<Device>, queue: Arc<Queue>, swapchain_format: Format, viewport: Viewport, dimensions: [u32; 2]) -> Self {

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
            screen_vertex_buffer,
            billboard_vertex_buffer,
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
        self.dimensions = dimensions;
    }

    pub fn create_render_target(&self, swapchain_image: Arc<SwapchainImage<Window>>) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(self.device.clone(), self.queue.clone(), self.render_pass.clone(), swapchain_image, self.dimensions)
    }

    pub fn render_water(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: WaterVertexBuffer, day_timer: f32) {
        self.water_renderer.render(render_target, camera, vertex_buffer, day_timer);
    }

    pub fn ambient_light(&self, render_target: &mut <Self as Renderer>::Target, color: Color) {
        self.ambient_light_renderer.render(render_target, self.screen_vertex_buffer.clone(), color);
    }

    pub fn directional_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, light_image: ImageBuffer, light_matrix: Matrix4<f32>, direction: Vector3<f32>, color: Color, intensity: f32) {
        self.directional_light_renderer.render(render_target, camera, light_image, light_matrix, self.screen_vertex_buffer.clone(), direction, color, intensity);
    }

    pub fn point_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, position: Vector3<f32>, color: Color, range: f32) {
        self.point_light_renderer.render(render_target, camera, self.billboard_vertex_buffer.clone(), position, color, range);
    }

    pub fn water_light(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, water_level: f32) {
        self.water_light_renderer.render(render_target, camera, self.screen_vertex_buffer.clone(), water_level);
    }

    pub fn overlay_interface(&self, render_target: &mut <Self as Renderer>::Target, interface_image: ImageBuffer) {
        //self.water_light_renderer.render(render_target, camera, self.screen_vertex_buffer.clone(), water_level);
    }

    pub fn overlay_buffers(&self, render_target: &mut <Self as Renderer>::Target, light_image: ImageBuffer) {
        //self.water_light_renderer.render(render_target, camera, self.screen_vertex_buffer.clone(), water_level);
    }
}

impl Renderer for DeferredRenderer {
    type Target = DeferredRenderTarget;
}

impl GeometryRendererTrait for DeferredRenderer {

    fn render_geometry(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>)
        where Self: Renderer
    {
        self.geometry_renderer.render(render_target, camera, vertex_buffer.clone(), textures, world_matrix);
    }
}

impl EntityRendererTrait for DeferredRenderer {

    fn render_entity(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>)
        where Self: Renderer
    {
        self.entity_renderer.render(render_target, camera, texture, position, origin, size, cell_count, cell_position);
    }
}
