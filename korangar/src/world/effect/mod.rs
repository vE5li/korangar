mod lookup;

use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use derive_new::new;
use korangar_util::collision::{Frustum, Sphere};
use ragnarok_formats::map::EffectSource;
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use crate::graphics::{Camera, Color, Texture};
use crate::renderer::EffectRenderer;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::world::MarkerIdentifier;
use crate::world::{point_light_extent, PointLightId, PointLightManager};

pub trait EffectBase {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool;

    fn mark_for_deletion(&mut self);

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera);

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera);
}

pub trait EffectSourceExt {
    fn offset(&mut self, offset: Vector3<f32>);

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool);
}

impl EffectSourceExt for EffectSource {
    fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    fn render_marker(&self, renderer: &mut impl MarkerRenderer, camera: &dyn Camera, marker_identifier: MarkerIdentifier, hovered: bool) {
        renderer.render_marker(camera, marker_identifier, self.position, hovered);
    }
}

#[derive(new)]
pub struct Effect {
    frames_per_second: usize,
    max_key: usize,
    layers: Vec<Layer>,
}

impl Effect {
    pub fn new_frame_timer(&self) -> FrameTimer {
        FrameTimer {
            total_timer: 0.0,
            frames_per_second: self.frames_per_second,
            max_key: self.max_key,
            current_frame: 0,
        }
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, frame_timer: &FrameTimer, position: Point3<f32>) {
        for layer in &self.layers {
            let Some(frame) = layer.interpolate_frame(frame_timer) else {
                continue;
            };

            if frame.texture_index > layer.textures.len() {
                continue;
            }

            renderer.render_effect(
                camera,
                position,
                layer.textures[frame.texture_index].clone(),
                [
                    Vector2::new(frame.xy[0], frame.xy[4]),
                    Vector2::new(frame.xy[1], frame.xy[5]),
                    Vector2::new(frame.xy[3], frame.xy[7]),
                    Vector2::new(frame.xy[2], frame.xy[6]),
                ],
                [
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[3] + frame.uv[1]),
                    Vector2::new(frame.uv[0] + frame.uv[2], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[1]),
                    Vector2::new(frame.uv[0], frame.uv[3] + frame.uv[1]),
                ],
                frame.offset,
                frame.angle,
                frame.color,
                frame.source_blend_factor,
                frame.destination_blend_factor,
            );
        }
    }
}

#[derive(new)]
pub struct Layer {
    textures: Vec<Arc<Texture>>,
    indices: Vec<Option<usize>>,
    frames: Vec<Frame>,
}

impl Layer {
    fn interpolate_frame(&self, frame_timer: &FrameTimer) -> Option<Frame> {
        if let Some(frame_index) = self.indices[frame_timer.current_frame] {
            if let Some(next_frame) = self.frames.get(frame_index + 2) {
                Some(Self::interpolate(
                    &self.frames[frame_index],
                    next_frame,
                    frame_timer.current_frame,
                ))
            } else {
                Some(self.frames[frame_index].clone())
            }
        } else {
            None
        }
    }

    fn interpolate(first: &Frame, second: &Frame, frame_index: usize) -> Frame {
        let time = 1.0 / (second.frame_index - first.frame_index) as f32 * (frame_index - first.frame_index) as f32;
        let sub_mult = 1.0;

        // TODO: angle bias
        let angle = Self::ease_interpolate(first.angle.0, second.angle.0, time, 0.0, sub_mult);
        let color = (second.color - first.color) * time + first.color * sub_mult;

        let uv = (0..8)
            .map(|index| (second.uv[index] - first.uv[index]) * time + first.uv[index] * sub_mult)
            .next_chunk()
            .unwrap();

        // TODO: scale bias
        let xy = (0..8)
            .map(|index| Self::ease_interpolate(first.xy[index], second.xy[index], time, 0.0, sub_mult))
            .next_chunk()
            .unwrap();

        // TODO: additional logic for animation type 2 and 3
        let texture_index = first.texture_index;

        // TODO: bezier curves
        let offset_x = (second.offset.x - first.offset.x) * time + first.offset.x * sub_mult;
        let offset_y = (second.offset.y - first.offset.y) * time + first.offset.y * sub_mult;

        Frame {
            frame_index,
            frame_type: first.frame_type,
            offset: Vector2::new(offset_x, offset_y),
            uv,
            xy,
            texture_index,
            animation_type: first.animation_type,
            delay: first.delay,
            angle: Rad(angle),
            color,
            source_blend_factor: first.source_blend_factor,
            destination_blend_factor: first.destination_blend_factor,
            mt_present: first.mt_present,
        }
    }

    fn ease_interpolate(start_value: f32, end_value: f32, time: f32, bias: f32, sub_multiplier: f32) -> f32 {
        if bias > 0.0 {
            (end_value - start_value) * time.powf(1.0 + bias / 5.0) + start_value * sub_multiplier
        } else if bias < 0.0 {
            (end_value - start_value) * (1.0 - (1.0 - time).powf(-bias / 5.0 + 1.0)) + start_value * sub_multiplier
        } else {
            (end_value - start_value) * time + start_value * sub_multiplier
        }
    }
}

#[derive(Debug, Clone, new)]
pub struct Frame {
    frame_index: usize,
    frame_type: FrameType,
    offset: Vector2<f32>,
    uv: [f32; 8],
    xy: [f32; 8],
    texture_index: usize,
    animation_type: AnimationType,
    delay: f32,
    angle: Rad<f32>,
    color: Color,
    source_blend_factor: BlendFactor,
    destination_blend_factor: BlendFactor,
    mt_present: MultiTexturePresent,
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationType {
    Type0,
    Type1,
    Type2,
    Type3,
}

#[derive(Debug, Clone, Copy)]
pub enum FrameType {
    Basic,
    Morphing,
}

#[derive(Debug, Clone, Copy)]
pub enum MultiTexturePresent {
    None,
}

pub struct FrameTimer {
    total_timer: f32,
    frames_per_second: usize,
    max_key: usize,
    current_frame: usize,
}

impl FrameTimer {
    pub fn update(&mut self, delta_time: f32) -> bool {
        self.total_timer += delta_time;
        self.current_frame = (self.total_timer / (1.0 / self.frames_per_second as f32)) as usize;

        if self.current_frame >= self.max_key {
            // TODO: better wrapping
            self.total_timer = 0.0;
            self.current_frame = 0;
            return false;
        }

        true
    }
}

pub enum EffectCenter {
    Entity(EntityId, Point3<f32>),
    Position(Point3<f32>),
}

impl EffectCenter {
    fn to_position(&self) -> Point3<f32> {
        match self {
            EffectCenter::Entity(_, position) | EffectCenter::Position(position) => *position,
        }
    }
}

pub struct EffectWithLight {
    effect: Arc<Effect>,
    frame_timer: FrameTimer,
    center: EffectCenter,
    effect_offset: Vector3<f32>,
    point_light_id: PointLightId,
    light_offset: Vector3<f32>,
    light_color: Color,
    light_intensity: f32,
    repeating: bool,
    current_light_intensity: f32,
    gets_deleted: bool,
}

impl EffectWithLight {
    pub fn new(
        effect: Arc<Effect>,
        frame_timer: FrameTimer,
        center: EffectCenter,
        effect_offset: Vector3<f32>,
        point_light_id: PointLightId,
        light_offset: Vector3<f32>,
        light_color: Color,
        light_intensity: f32,
        repeating: bool,
    ) -> Self {
        Self {
            effect,
            frame_timer,
            center,
            effect_offset,
            point_light_id,
            light_offset,
            light_color,
            light_intensity,
            repeating,
            current_light_intensity: 0.0,
            gets_deleted: false,
        }
    }
}

impl EffectBase for EffectWithLight {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        const FADE_SPEED: f32 = 5.0;

        if let EffectCenter::Entity(entity_id, position) = &mut self.center
            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == *entity_id)
        {
            let new_position = entity.get_position();
            *position = new_position;
        }

        if !self.gets_deleted && !self.frame_timer.update(delta_time) && !self.repeating {
            self.gets_deleted = true;
        }

        let (target, clamping_function): (f32, fn(f32, f32) -> f32) = match self.gets_deleted {
            true => (0.0, f32::max),
            false => (self.light_intensity, f32::min),
        };

        self.current_light_intensity += (target - self.current_light_intensity) * FADE_SPEED * delta_time;
        self.current_light_intensity = clamping_function(self.current_light_intensity, target);

        !self.gets_deleted || self.current_light_intensity > 0.1
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let frustum = Frustum::new(projection_matrix * view_matrix);

        let extent = point_light_extent(self.light_color, self.current_light_intensity);
        let light_position = self.center.to_position() + self.light_offset;

        if frustum.intersects_sphere(&Sphere::new(light_position, extent)) {
            point_light_manager.register_fading(
                self.point_light_id,
                light_position,
                self.light_color,
                self.current_light_intensity,
                self.light_intensity,
            )
        }
    }

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if !self.gets_deleted {
            self.effect.render(
                renderer,
                camera,
                &self.frame_timer,
                self.center.to_position() + self.effect_offset,
            );
        }
    }
}

#[derive(Default)]
pub struct EffectHolder {
    effects: Vec<(Box<dyn EffectBase + Send + Sync>, Option<EntityId>)>,
}

impl EffectHolder {
    pub fn add_effect(&mut self, effect: Box<dyn EffectBase + Send + Sync>) {
        self.effects.push((effect, None));
    }

    pub fn add_unit(&mut self, effect: Box<dyn EffectBase + Send + Sync>, entity_id: EntityId) {
        self.effects.push((effect, Some(entity_id)));
    }

    pub fn remove_unit(&mut self, removed_entity_id: EntityId) {
        self.effects
            .iter_mut()
            .filter(|(_, entity_id)| entity_id.is_some_and(|entity_id| entity_id == removed_entity_id))
            .for_each(|(effect, _)| effect.mark_for_deletion());
    }

    pub fn clear(&mut self) {
        self.effects.clear();
    }

    pub fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) {
        self.effects.retain_mut(|(effect, _)| effect.update(entities, delta_time));
    }

    pub fn register_point_lights(&self, point_light_manager: &mut PointLightManager, camera: &dyn Camera) {
        self.effects
            .iter()
            .for_each(|(effect, _)| effect.register_point_lights(point_light_manager, camera));
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        self.effects.iter().for_each(|(effect, _)| effect.render(renderer, camera));
    }
}
