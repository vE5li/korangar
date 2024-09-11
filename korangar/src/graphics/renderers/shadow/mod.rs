mod entity;
mod geometry;
mod indicator;

use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use ragnarok_packets::EntityId;
use serde::{Deserialize, Serialize};
use wgpu::{Device, Queue, RenderPass, TextureFormat, TextureUsages};

use self::entity::EntityRenderer;
use self::geometry::GeometryRenderer;
use self::indicator::IndicatorRenderer;
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
pub enum ShadowSubRenderer {
    Geometry,
    Entity,
    Indicator,
}

pub struct ShadowRenderer {
    device: Arc<Device>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    indicator_renderer: IndicatorRenderer,
    walk_indicator: Arc<Texture>,
}

impl ShadowRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, game_file_loader: &mut GameFileLoader, texture_loader: &mut TextureLoader) -> Self {
        let output_depth_format = <Self as Renderer>::Target::output_texture_format();

        let geometry_renderer = GeometryRenderer::new(device.clone(), queue.clone(), output_depth_format);
        let entity_renderer = EntityRenderer::new(device.clone(), queue.clone(), output_depth_format);
        let indicator_renderer = IndicatorRenderer::new(device.clone(), output_depth_format);

        let walk_indicator = texture_loader.get("grid.tga", game_file_loader).unwrap();

        Self {
            device,
            geometry_renderer,
            entity_renderer,
            indicator_renderer,
            walk_indicator,
        }
    }

    pub fn create_render_target(&self, size: u32) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            &self.device,
            "shadow",
            [size; 2],
            1,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            1.0,
        )
    }
}

pub struct ShadowFormat {}

impl IntoFormat for ShadowFormat {
    fn into_format() -> TextureFormat {
        TextureFormat::Depth32Float
    }
}

impl Renderer for ShadowRenderer {
    type Target = SingleRenderTarget<ShadowFormat, ShadowSubRenderer, f32>;
}

impl GeometryRendererTrait for ShadowRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer
            .render(render_target, render_pass, camera, vertex_buffer, textures, world_matrix, time);
    }
}

impl EntityRendererTrait for ShadowRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        texture: &Texture,
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
            render_pass,
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
        render_pass: &mut RenderPass,
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
            render_pass,
            camera,
            &self.walk_indicator,
            upper_left,
            upper_right,
            lower_left,
            lower_right,
        );
    }
}
