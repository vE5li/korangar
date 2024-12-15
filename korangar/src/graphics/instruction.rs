use std::sync::Arc;

use cgmath::{Matrix4, Point3, Vector2, Vector3, Vector4};
use korangar_util::Rectangle;
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use super::color::Color;
#[cfg(feature = "debug")]
use super::settings::RenderSettings;
use super::vertices::ModelVertex;
use super::{Buffer, ShadowQuality, Texture, TileVertex};
use crate::interface::layout::{CornerRadius, ScreenClip, ScreenPosition, ScreenSize};
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;

pub struct RenderInstruction<'a> {
    pub clear_interface: bool,
    pub show_interface: bool,
    pub picker_position: ScreenPosition,
    pub uniforms: Uniforms,
    pub indicator: Option<IndicatorInstruction>,
    pub interface: &'a [InterfaceRectangleInstruction],
    /// Between 3D world and effects.
    pub bottom_layer_rectangles: &'a [RectangleInstruction],
    /// Between effects and interface.
    pub middle_layer_rectangles: &'a [RectangleInstruction],
    /// On top of everything else.
    pub top_layer_rectangles: &'a [RectangleInstruction],
    pub directional_light_with_shadow: DirectionalShadowCasterInstruction,
    pub point_light_shadow_caster: &'a [PointShadowCasterInstruction],
    pub point_light: &'a [PointLightInstruction],
    pub model_batches: &'a [ModelBatch],
    pub models: &'a mut [ModelInstruction],
    pub entities: &'a mut [EntityInstruction],
    pub directional_model_batches: &'a [ModelBatch],
    pub directional_shadow_models: &'a [ModelInstruction],
    pub directional_shadow_entities: &'a [EntityInstruction],
    pub point_shadow_models: &'a [ModelInstruction],
    pub point_shadow_entities: &'a [EntityInstruction],
    pub effects: &'a [EffectInstruction],
    pub water: Option<WaterInstruction<'a>>,
    pub map_picker_tile_vertex_buffer: &'a Buffer<TileVertex>,
    pub font_atlas_texture: &'a Texture,
    #[cfg(feature = "debug")]
    pub render_settings: RenderSettings,
    #[cfg(feature = "debug")]
    pub aabb: &'a [DebugAabbInstruction],
    #[cfg(feature = "debug")]
    pub circles: &'a [DebugCircleInstruction],
    #[cfg(feature = "debug")]
    pub rectangles: &'a [DebugRectangleInstruction],
    #[cfg(feature = "debug")]
    pub marker: &'a [MarkerInstruction],
}

#[derive(Clone, Debug)]
pub struct Uniforms {
    pub view_matrix: Matrix4<f32>,
    pub projection_matrix: Matrix4<f32>,
    pub camera_position: Vector4<f32>,
    pub animation_timer: f32,
    pub day_timer: f32,
    pub ambient_light_color: Color,
    pub enhanced_lighting: bool,
    pub shadow_quality: ShadowQuality,
}

#[derive(Clone, Debug)]
pub struct WaterInstruction<'a> {
    pub water_texture: &'a Texture,
    pub water_bounds: Rectangle<f32>,
    pub texture_repeat: f32,
    pub water_level: f32,
    pub wave_amplitude: f32,
    pub wave_speed: f32,
    pub wave_length: f32,
    pub water_opacity: f32,
}

#[derive(Clone, Debug)]
pub struct DirectionalShadowCasterInstruction {
    pub view_projection_matrix: Matrix4<f32>,
    pub view_matrix: Matrix4<f32>,
    pub direction: Vector3<f32>,
    pub color: Color,
}

/// Right now point shadows can't cast shadows of models that are not part of
/// the map.
#[derive(Clone, Debug)]
pub struct PointShadowCasterInstruction {
    pub view_projection_matrices: [Matrix4<f32>; 6],
    pub view_matrices: [Matrix4<f32>; 6],
    pub position: Point3<f32>,
    pub color: Color,
    pub range: f32,
    pub model_texture: Arc<Texture>,
    pub model_vertex_buffer: Arc<Buffer<ModelVertex>>,
    /// Start point inside the point_shadow_entities.
    pub entity_offset: [usize; 6],
    /// Model count inside the point_shadow_entities.
    pub entity_count: [usize; 6],
    /// Start point inside the point_shadow_models.
    pub model_offset: [usize; 6],
    /// Model count inside the point_shadow_models.
    pub model_count: [usize; 6],
}

#[derive(Clone, Debug)]
pub struct PointLightInstruction {
    pub position: Point3<f32>,
    pub color: Color,
    pub range: f32,
}

#[derive(Clone, Debug)]
pub enum RectangleInstruction {
    Solid {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
    },
    Sprite {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        color: Color,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
        linear_filtering: bool,
        texture: Arc<Texture>,
    },
}

#[derive(Clone, Debug)]
pub enum InterfaceRectangleInstruction {
    Solid {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        corner_radius: CornerRadius,
    },
    Sprite {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        corner_radius: CornerRadius,
        texture: Arc<Texture>,
        smooth: bool,
    },
    Text {
        screen_position: ScreenPosition,
        screen_size: ScreenSize,
        screen_clip: ScreenClip,
        color: Color,
        texture_position: Vector2<f32>,
        texture_size: Vector2<f32>,
    },
}

#[cfg(feature = "debug")]
#[derive(Clone, Debug)]
pub struct MarkerInstruction {
    pub screen_position: ScreenPosition,
    pub screen_size: ScreenSize,
    pub identifier: MarkerIdentifier,
}

#[derive(Clone, Debug)]
pub struct IndicatorInstruction {
    pub upper_left: Point3<f32>,
    pub upper_right: Point3<f32>,
    pub lower_left: Point3<f32>,
    pub lower_right: Point3<f32>,
    pub color: Color,
}

pub struct ModelBatch {
    pub offset: usize,
    pub count: usize,
    pub texture: Arc<Texture>,
    pub vertex_buffer: Arc<Buffer<ModelVertex>>,
}

#[derive(Clone, Debug)]
pub struct ModelInstruction {
    pub model_matrix: Matrix4<f32>,
    pub vertex_offset: usize,
    pub vertex_count: usize,
    pub distance: f32,
    pub transparent: bool,
}

#[derive(Clone, Debug)]
pub struct EntityInstruction {
    pub world: Matrix4<f32>,
    pub frame_part_transform: Matrix4<f32>,
    pub texture_position: Vector2<f32>,
    pub texture_size: Vector2<f32>,
    pub frame_size: Vector2<f32>,
    pub extra_depth_offset: f32,
    pub depth_offset: f32,
    pub curvature: f32,
    pub color: Color,
    pub mirror: bool,
    pub entity_id: EntityId,
    pub add_to_picker: bool,
    pub texture: Arc<Texture>,
    pub distance: f32,
}

#[derive(Clone, Debug)]
pub struct EffectInstruction {
    pub top_left: ScreenPosition,
    pub bottom_left: ScreenPosition,
    pub top_right: ScreenPosition,
    pub bottom_right: ScreenPosition,
    pub texture_top_left: Vector2<f32>,
    pub texture_bottom_left: Vector2<f32>,
    pub texture_top_right: Vector2<f32>,
    pub texture_bottom_right: Vector2<f32>,
    pub color: Color,
    pub source_blend_factor: BlendFactor,
    pub destination_blend_factor: BlendFactor,
    pub texture: Arc<Texture>,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugAabbInstruction {
    pub world: Matrix4<f32>,
    pub color: Color,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugCircleInstruction {
    pub position: Point3<f32>,
    pub color: Color,
    pub screen_position: ScreenPosition,
    pub screen_size: ScreenSize,
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug)]
pub struct DebugRectangleInstruction {
    pub world: Matrix4<f32>,
    pub color: Color,
}
