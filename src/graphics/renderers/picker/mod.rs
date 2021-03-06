mod tile;
mod geometry;
mod entity;

use std::sync::Arc;
use vulkano::device::{ Device, Queue };
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::format::Format;
use vulkano::render_pass::RenderPass;

use crate::graphics::TileVertexBuffer;
use crate::types::maths::*;
use super::{ Renderer, Camera, GeometryRenderer as GeometryRendererTrait, EntityRenderer as EntityRendererTrait, Texture, ModelVertexBuffer, PickerRenderTarget };

use self::tile::TileRenderer;
use self::geometry::GeometryRenderer;
use self::entity::EntityRenderer;

#[derive(PartialEq)]
pub enum PickerSubrenderer {
    Geometry,
    Entity,
}

pub struct PickerRenderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    tile_renderer: TileRenderer,
    dimensions: [u32; 2],
}

impl PickerRenderer {

    pub fn new(device: Arc<Device>, queue: Arc<Queue>, viewport: Viewport, dimensions: [u32; 2]) -> Self {

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: Format::R32_UINT,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .unwrap();

        let subpass = render_pass.clone().first_subpass();
        let geometry_renderer = GeometryRenderer::new(device.clone(), subpass.clone(), viewport.clone());
        let entity_renderer = EntityRenderer::new(device.clone(), subpass.clone(), viewport.clone());
        let tile_renderer = TileRenderer::new(device.clone(), subpass.clone(), viewport.clone());

        Self {
            device,
            queue,
            render_pass,
            geometry_renderer,
            entity_renderer,
            tile_renderer,
            dimensions,
        }
    }

    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2]) {
        let subpass = self.render_pass.clone().first_subpass();
        self.geometry_renderer.recreate_pipeline(self.device.clone(), subpass.clone(), viewport.clone(), false);
        self.entity_renderer.recreate_pipeline(self.device.clone(), subpass.clone(), viewport.clone());
        self.tile_renderer.recreate_pipeline(self.device.clone(), subpass.clone(), viewport.clone());
        self.dimensions = dimensions;
    }

    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(self.device.clone(), self.queue.clone(), self.render_pass.clone(), self.dimensions)
    }

    pub fn render_tiles(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: TileVertexBuffer) {
        render_target.unbind_subrenderer();
        self.tile_renderer.render(render_target, camera, vertex_buffer);
    }
}

impl Renderer for PickerRenderer {
    type Target = PickerRenderTarget;
}

impl GeometryRendererTrait for PickerRenderer {

    fn render_geometry(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, vertex_buffer: ModelVertexBuffer, textures: &Vec<Texture>, world_matrix: Matrix4<f32>)
        where Self: Renderer
    {
        if render_target.bind_subrenderer(PickerSubrenderer::Geometry) {
            self.geometry_renderer.bind_pipeline(render_target, camera);
        }

        self.geometry_renderer.render(render_target, camera, vertex_buffer.clone(), textures, world_matrix);
    }
}

impl EntityRendererTrait for PickerRenderer {

    fn render_entity(&self, render_target: &mut <Self as Renderer>::Target, camera: &dyn Camera, texture: Texture, position: Vector3<f32>, origin: Vector3<f32>, size: Vector2<f32>, cell_count: Vector2<usize>, cell_position: Vector2<usize>, entity_id: usize)
        where Self: Renderer
    {
        if render_target.bind_subrenderer(PickerSubrenderer::Entity) {
            self.entity_renderer.bind_pipeline(render_target, camera);
        }

        self.entity_renderer.render(render_target, camera, texture, position, origin, size, cell_count, cell_position, entity_id);
    }
}
