mod entity;
mod geometry;
#[cfg(feature = "debug")]
mod marker;
mod target;
mod tile;

use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use korangar_procedural::profile;
use vulkano::device::{DeviceOwned, Queue};
use vulkano::format::Format;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::RenderPass;

use self::entity::EntityRenderer;
use self::geometry::GeometryRenderer;
#[cfg(feature = "debug")]
use self::marker::MarkerRenderer;
pub use self::target::PickerTarget;
use self::tile::TileRenderer;
use super::SubpassAttachments;
#[cfg(feature = "debug")]
use crate::graphics::MarkerRenderer as MarkerRendererTrait;
use crate::graphics::{EntityRenderer as EntityRendererTrait, GeometryRenderer as GeometryRendererTrait, *};
use crate::network::EntityId;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(PartialEq, Eq)]
pub enum PickerSubrenderer {
    Geometry,
    Entity,
    Tile,
    #[cfg(feature = "debug")]
    Marker,
}

pub struct PickerRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    tile_renderer: TileRenderer,
    #[cfg(feature = "debug")]
    marker_renderer: MarkerRenderer,
    dimensions: [u32; 2],
}

unsafe impl Send for PickerRenderer {}
unsafe impl Sync for PickerRenderer {}

impl PickerRenderer {
    const fn subpass() -> SubpassAttachments {
        SubpassAttachments { color: 1, depth: 1 }
    }

    pub fn new(memory_allocator: Arc<MemoryAllocator>, queue: Arc<Queue>, viewport: Viewport, dimensions: [u32; 2]) -> Self {
        let device = memory_allocator.device().clone();
        let render_pass = vulkano::single_pass_renderpass!(
            device,
            attachments: {
                color: {
                    format: Format::R32_UINT,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                depth: {
                    format: Format::D16_UNORM,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {depth}
            }
        )
        .unwrap();

        let subpass = render_pass.clone().first_subpass();
        let geometry_renderer = GeometryRenderer::new(memory_allocator.clone(), subpass.clone(), viewport.clone());
        let entity_renderer = EntityRenderer::new(memory_allocator.clone(), subpass.clone(), viewport.clone());
        let tile_renderer = TileRenderer::new(memory_allocator.clone(), subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        let marker_renderer = MarkerRenderer::new(memory_allocator.clone(), subpass, viewport);

        Self {
            memory_allocator,
            queue,
            render_pass,
            geometry_renderer,
            entity_renderer,
            tile_renderer,
            #[cfg(feature = "debug")]
            marker_renderer,
            dimensions,
        }
    }

    #[profile("recreate picker pipeline")]
    pub fn recreate_pipeline(&mut self, viewport: Viewport, dimensions: [u32; 2]) {
        let device = self.memory_allocator.device().clone();
        let subpass = self.render_pass.clone().first_subpass();
        self.geometry_renderer
            .recreate_pipeline(device.clone(), subpass.clone(), viewport.clone(), false);
        self.entity_renderer
            .recreate_pipeline(device.clone(), subpass.clone(), viewport.clone());
        self.tile_renderer
            .recreate_pipeline(device.clone(), subpass.clone(), viewport.clone());
        #[cfg(feature = "debug")]
        self.marker_renderer.recreate_pipeline(device, subpass, viewport);
        self.dimensions = dimensions;
    }

    #[profile("create picker render target")]
    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            self.memory_allocator.clone(),
            self.queue.clone(),
            self.render_pass.clone(),
            self.dimensions,
        )
    }

    pub fn render_tiles(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[TileVertex]>,
    ) {
        self.tile_renderer.render(render_target, camera, vertex_buffer);
    }
}

impl Renderer for PickerRenderer {
    type Target = PickerRenderTarget;
}

impl GeometryRendererTrait for PickerRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
        world_matrix: Matrix4<f32>,
        _time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer
            .render(render_target, camera, vertex_buffer, textures, world_matrix);
    }
}

impl EntityRendererTrait for PickerRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        texture: Arc<ImageView>,
        position: Vector3<f32>,
        origin: Vector3<f32>,
        scale: Vector2<f32>,
        cell_count: Vector2<usize>,
        cell_position: Vector2<usize>,
        mirror: bool,
        entity_id: EntityId,
    ) where
        Self: Renderer,
    {
        self.entity_renderer.render(
            render_target,
            camera,
            texture,
            position,
            origin,
            scale,
            cell_count,
            cell_position,
            entity_id,
            mirror,
        );
    }
}

#[cfg(feature = "debug")]
impl MarkerRendererTrait for PickerRenderer {
    fn render_marker(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        position: Vector3<f32>,
        _hovered: bool,
    ) where
        Self: Renderer,
    {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MarkerIdentifier::SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(bottom_right_position, top_left_position); // WHY ARE THESE INVERTED ???

            self.marker_renderer
                .render(render_target, screen_position, screen_size, marker_identifier);
        }
    }
}
