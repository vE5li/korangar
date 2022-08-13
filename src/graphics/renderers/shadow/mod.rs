mod geometry;
mod entity;

use std::sync::Arc;
use vulkano::render_pass::RenderPass;
use vulkano::{device::Device, format::Format};
use vulkano::device::Queue;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::image::{ImageUsage, SampleCount};

use crate::types::maths::*;
use super::{ Renderer, Camera, GeometryRenderer as GeometryRendererTrait, EntityRenderer as EntityRendererTrait, SingleRenderTarget, Texture, ModelVertexBuffer };

use self::geometry::GeometryRenderer;
use self::entity::EntityRenderer;

#[derive(PartialEq)]
pub enum ShadowSubrenderer {
    Geometry,
    Entity,
}

pub struct ShadowRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
}

impl ShadowRenderer {

    pub fn new(device: Arc<Device>, queue: Arc<Queue>) -> Self {

        let render_pass = vulkano::single_pass_renderpass!(
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

        let subpass = render_pass.clone().first_subpass();
        let geometry_renderer = GeometryRenderer::new(device.clone(), subpass.clone());
        let entity_renderer = EntityRenderer::new(device.clone(), subpass.clone());

        Self {
            device,
            queue,
            render_pass,
            geometry_renderer,
            entity_renderer,
        }
    }

    pub fn recreate_pipeline(&mut self) {
        let subpass = self.render_pass.clone().first_subpass();
        self.geometry_renderer.recreate_pipeline(self.device.clone(), subpass.clone());
        self.entity_renderer.recreate_pipeline(self.device.clone(), subpass.clone());
    }

    pub fn create_render_target(&self, size: u32) -> <Self as Renderer>::Target {

        let image_usage = ImageUsage {
            sampled: true,
            depth_stencil_attachment: true,
            ..ImageUsage::none()
        };

        <Self as Renderer>::Target::new(self.device.clone(), self.queue.clone(), self.render_pass.clone(), [size; 2], SampleCount::Sample1, image_usage, vulkano::format::ClearValue::Depth(1.0))
    }
}

impl Renderer for ShadowRenderer {
    type Target = SingleRenderTarget<{ Format::D32_SFLOAT }, ShadowSubrenderer>;
}

impl GeometryRendererTrait for ShadowRenderer {

    fn render_geometry(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>)
        where Self: Renderer
    {
        if render_target.bind_subrenderer(ShadowSubrenderer::Geometry) {
            self.geometry_renderer.bind_pipeline(render_target, camera);
        }

        self.geometry_renderer.render(render_target, camera, vertex_buffer.clone(), textures, world_matrix);
    }
}

impl EntityRendererTrait for ShadowRenderer {

    fn render_entity(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, scale: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>, _entity_id: usize)
        where Self: Renderer
    {
        if render_target.bind_subrenderer(ShadowSubrenderer::Entity) {
            self.entity_renderer.bind_pipeline(render_target, camera);
        }

        self.entity_renderer.render(render_target, camera, texture, position, origin, scale, cell_count, cell_position);
    }
}
