mod entity;
mod geometry;
mod indicator;

use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use ragnarok_packets::EntityId;
use serde::{Deserialize, Serialize};
use vulkano::device::{DeviceOwned, Queue};
use vulkano::format::{ClearValue, Format};
use vulkano::image::{ImageUsage, SampleCount};
use vulkano::render_pass::RenderPass;

use self::entity::EntityRenderer;
use self::geometry::GeometryRenderer;
use self::indicator::IndicatorRenderer;
use super::SubpassAttachments;
use crate::graphics::{
    EntityRenderer as EntityRendererTrait, GeometryRenderer as GeometryRendererTrait, IndicatorRenderer as IndicatorRendererTrait, *,
};
use crate::loaders::{GameFileLoader, TextureLoader};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShadowDetail {
    Low,
    Medium,
    High,
    Ultra,
}

impl ShadowDetail {
    pub fn into_resolution(self) -> u32 {
        match self {
            ShadowDetail::Low => 512,
            ShadowDetail::Medium => 1024,
            ShadowDetail::High => 2048,
            ShadowDetail::Ultra => 8192,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum ShadowSubrenderer {
    Geometry,
    Entity,
    Indicator,
}

pub struct ShadowRenderer {
    memory_allocator: Arc<MemoryAllocator>,
    queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    indicator_renderer: IndicatorRenderer,
    walk_indicator: Arc<ImageView>,
}

unsafe impl Send for ShadowRenderer {}
unsafe impl Sync for ShadowRenderer {}

impl ShadowRenderer {
    const fn subpass() -> SubpassAttachments {
        SubpassAttachments { color: 0, depth: 1 }
    }

    pub fn new(
        memory_allocator: Arc<MemoryAllocator>,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        queue: Arc<Queue>,
    ) -> Self {
        let device = memory_allocator.device().clone();
        let render_pass = vulkano::single_pass_renderpass!(
            device,
            attachments: {
                depth: {
                    format: Format::D32_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                }
            },
            pass: {
                color: [],
                depth_stencil: {depth}
            }
        )
        .unwrap();

        let subpass = render_pass.clone().first_subpass();
        let geometry_renderer = GeometryRenderer::new(memory_allocator.clone(), subpass.clone());
        let entity_renderer = EntityRenderer::new(memory_allocator.clone(), subpass.clone());
        let indicator_renderer = IndicatorRenderer::new(memory_allocator.clone(), subpass);

        let walk_indicator = texture_loader.get("grid.tga", game_file_loader).unwrap();

        Self {
            memory_allocator,
            queue,
            render_pass,
            geometry_renderer,
            entity_renderer,
            indicator_renderer,
            walk_indicator,
        }
    }

    pub fn create_render_target(&self, size: u32) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            self.memory_allocator.clone(),
            self.queue.clone(),
            self.render_pass.clone(),
            [size; 2],
            SampleCount::Sample1,
            ImageUsage::SAMPLED | ImageUsage::DEPTH_STENCIL_ATTACHMENT,
            ClearValue::Depth(1.0),
        )
    }
}

pub struct ShadowFormat {}

impl IntoFormat for ShadowFormat {
    fn into_format() -> Format {
        Format::D32_SFLOAT
    }
}

impl Renderer for ShadowRenderer {
    type Target = SingleRenderTarget<ShadowFormat, ShadowSubrenderer, ClearValue>;
}

impl GeometryRendererTrait for ShadowRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        vertex_buffer: Subbuffer<[ModelVertex]>,
        textures: &[Arc<ImageView>],
        world_matrix: Matrix4<f32>,
        time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer
            .render(render_target, camera, vertex_buffer, textures, world_matrix, time);
    }
}

impl EntityRendererTrait for ShadowRenderer {
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
        _entity_id: EntityId,
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
            mirror,
        );
    }
}

impl IndicatorRendererTrait for ShadowRenderer {
    fn render_walk_indicator(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        camera: &dyn Camera,
        _color: Color,
        upper_left: Vector3<f32>,
        upper_right: Vector3<f32>,
        lower_left: Vector3<f32>,
        lower_right: Vector3<f32>,
    ) where
        Self: Renderer,
    {
        self.indicator_renderer.render_ground_indicator(
            render_target,
            camera,
            self.walk_indicator.clone(),
            upper_left,
            upper_right,
            lower_left,
            lower_right,
        );
    }
}
