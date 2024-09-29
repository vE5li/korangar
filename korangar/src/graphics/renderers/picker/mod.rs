mod entity;
mod geometry;
#[cfg(feature = "debug")]
mod marker;
mod selector;
mod target;
mod tile;

use std::sync::Arc;

use cgmath::{Matrix4, Vector2, Vector3};
use ragnarok_packets::EntityId;
use wgpu::{ComputePass, Device, Queue, RenderPass};

use self::entity::EntityRenderer;
use self::geometry::GeometryRenderer;
#[cfg(feature = "debug")]
use self::marker::MarkerRenderer;
use self::selector::Selector;
pub use self::target::PickerTarget;
use self::tile::TileRenderer;
#[cfg(feature = "debug")]
use crate::graphics::MarkerRenderer as MarkerRendererTrait;
use crate::graphics::{EntityRenderer as EntityRendererTrait, GeometryRenderer as GeometryRendererTrait, *};
use crate::interface::layout::{ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

#[derive(PartialEq, Eq)]
pub enum PickerSubRenderer {
    Geometry,
    Entity,
    Tile,
    #[cfg(feature = "debug")]
    Marker,
    Selector,
}

pub struct PickerRenderer {
    device: Arc<Device>,
    geometry_renderer: GeometryRenderer,
    entity_renderer: EntityRenderer,
    tile_renderer: TileRenderer,
    #[cfg(feature = "debug")]
    marker_renderer: MarkerRenderer,
    selector: Selector,
    dimensions: [u32; 2],
}

impl PickerRenderer {
    pub fn new(device: Arc<Device>, queue: Arc<Queue>, dimensions: [u32; 2]) -> Self {
        let output_color_format = <Self as Renderer>::Target::output_color_format();
        let output_depth_format = <Self as Renderer>::Target::depth_texture_format();

        let geometry_renderer = GeometryRenderer::new(device.clone(), queue.clone(), output_color_format, output_depth_format);
        let entity_renderer = EntityRenderer::new(device.clone(), queue.clone(), output_color_format, output_depth_format);
        let tile_renderer = TileRenderer::new(device.clone(), queue.clone(), output_color_format, output_depth_format);
        #[cfg(feature = "debug")]
        let marker_renderer = MarkerRenderer::new(device.clone(), output_color_format, output_depth_format);
        let selector = Selector::new(device.clone());

        Self {
            device,
            geometry_renderer,
            entity_renderer,
            tile_renderer,
            #[cfg(feature = "debug")]
            marker_renderer,
            selector,
            dimensions,
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("reconfigure picker pipeline"))]
    pub fn reconfigure_pipeline(&mut self, dimensions: [u32; 2]) {
        self.dimensions = dimensions;
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("create picker render target"))]
    pub fn create_render_target(&self) -> <Self as Renderer>::Target {
        <Self as Renderer>::Target::new(&self.device, self.dimensions)
    }

    pub fn render_tiles(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<TileVertex>,
    ) {
        self.tile_renderer.render(render_target, render_pass, camera, vertex_buffer);
    }

    pub fn dispatch_selector(
        &self,
        render_target: &mut <PickerRenderer as Renderer>::Target,
        compute_pass: &mut ComputePass,
        window_size: ScreenSize,
        pointer_position: ScreenPosition,
    ) {
        let clamped_pointer_position = Vector2::new(
            pointer_position.left.clamp(0.0, window_size.width) as u32,
            pointer_position.top.clamp(0.0, window_size.height) as u32,
        );
        self.selector.dispatch(render_target, compute_pass, clamped_pointer_position);
    }
}

impl Renderer for PickerRenderer {
    type Target = PickerRenderTarget;
}

impl GeometryRendererTrait for PickerRenderer {
    fn render_geometry(
        &self,
        render_target: &mut <Self as Renderer>::Target,
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        vertex_buffer: &Buffer<ModelVertex>,
        textures: &TextureGroup,
        world_matrix: Matrix4<f32>,
        _time: f32,
    ) where
        Self: Renderer,
    {
        self.geometry_renderer
            .render(render_target, render_pass, camera, vertex_buffer, textures, world_matrix);
    }
}

impl EntityRendererTrait for PickerRenderer {
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
        entity_id: EntityId,
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
        render_pass: &mut RenderPass,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        position: Vector3<f32>,
        _hovered: bool,
    ) where
        Self: Renderer,
    {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MarkerIdentifier::SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

            self.marker_renderer
                .render(render_target, render_pass, screen_position, screen_size, marker_identifier);
        }
    }
}
