mod entity;
mod geometry;
mod indicator;

use std::sync::Arc;

use cgmath::{Point3, Vector2};
use ragnarok_packets::EntityId;
use wgpu::{Device, Queue, RenderPass, TextureFormat, TextureUsages};

use self::entity::EntityRenderer;
use self::geometry::GeometryRenderer;
use self::indicator::IndicatorRenderer;
use crate::graphics::{
    EntityRenderer as EntityRendererTrait, GeometryRenderer as GeometryRendererTrait, IndicatorRenderer as IndicatorRendererTrait, *,
};
use crate::loaders::TextureLoader;

#[derive(PartialEq, Eq)]
pub enum PointShadowSubRenderer {
    Geometry,
    Entity,
    Indicator,
}

pub struct PointShadowRenderer {
    device: Arc<Device>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    indicator_renderer: IndicatorRenderer,
    walk_indicator: Arc<Texture>,
    light_position: Point3<f32>,
}

impl PointShadowRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, texture_loader: &mut TextureLoader) -> Self {
        let output_depth_format = <Self as Renderer>::Target::output_texture_format();

        let geometry_renderer = GeometryRenderer::new(device.clone(), queue, output_depth_format);
        let entity_renderer = EntityRenderer::new(device.clone(), output_depth_format);
        let indicator_renderer = IndicatorRenderer::new(device.clone(), output_depth_format);

        let walk_indicator = texture_loader.get("grid.tga").unwrap();
        let light_position = Point3::new(0.0, 0.0, 0.0);

        Self {
            device,
            geometry_renderer,
            entity_renderer,
            indicator_renderer,
            walk_indicator,
            light_position,
        }
    }

    pub fn set_light_position(&mut self, light_position: Point3<f32>) {
        self.light_position = light_position;
    }

    pub fn create_render_target(&self, size: u32) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(
            &self.device,
            "point shadow",
            [size; 2],
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        )
    }
}

pub struct PointShadowFormat {}

impl IntoFormat for PointShadowFormat {
    fn into_format() -> TextureFormat {
        TextureFormat::Depth32Float
    }
}

impl Renderer for PointShadowRenderer {
    type Target = CubeRenderTarget<PointShadowSubRenderer>;
}

impl GeometryRendererTrait for PointShadowRenderer {
    fn render_geometry(
        &mut self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        _camera: &dyn Camera,
        instructions: &[GeometryInstruction],
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer.render(
            render_target,
            render_pass,
            self.light_position,
            instructions,
            vertex_buffer,
            textures,
            time,
        );
    }
}

impl EntityRendererTrait for PointShadowRenderer {
    fn render_entity(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        texture: &Texture,
        position: Point3<f32>,
        origin: Point3<f32>,
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
            self.light_position,
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

impl IndicatorRendererTrait for PointShadowRenderer {
    fn render_walk_indicator(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        _color: Color,
        upper_left: Point3<f32>,
        upper_right: Point3<f32>,
        lower_left: Point3<f32>,
        lower_right: Point3<f32>,
    ) where
        Self: Renderer,
    {
        self.indicator_renderer.render_ground_indicator(
            render_target,
            render_pass,
            camera,
            self.light_position,
            &self.walk_indicator,
            upper_left,
            upper_right,
            lower_left,
            lower_right,
        );
    }
}
