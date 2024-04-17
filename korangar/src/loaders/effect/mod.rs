use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Vector2, Vector3};
use derive_new::new;
#[cfg(feature = "debug")]
use korangar_debug::Colorize;
use korangar_interface::elements::PrototypeElement;
use ragnarok_bytes::{ByteStream, FromBytes};
use ragnarok_networking::EntityId;
use vulkano::image::view::ImageView;

use super::version::InternalVersion;
use super::{MajorFirst, TextureLoader};
use crate::graphics::{Camera, Color, DeferredRenderer, Renderer};
use crate::loaders::{GameFileLoader, Version};

#[derive(Debug, FromBytes, PrototypeElement)]
struct TextureName {
    #[length_hint(128)]
    pub name: String,
}

#[derive(Debug, Clone, FromBytes, PrototypeElement)]
pub struct Frame {
    pub frame_index: i32,
    pub frame_type: i32,
    pub offset: Vector2<f32>,
    pub uv: [f32; 8],
    pub xy: [f32; 8],
    pub texture_index: f32,
    pub animation_type: i32,
    pub delay: f32,
    pub angle: f32,
    pub color: [f32; 4],
    // Needs to actually set the attachment blend mode of the source alpha
    pub source_alpha: i32,
    // Needs to actually set the attachment blend mode of the destination alpha
    pub destination_alpha: i32,
    pub mt_present: i32,
}

impl Frame {
    fn ease_interpolate(start_value: f32, end_value: f32, time: f32, bias: f32, sub_multiplier: f32) -> f32 {
        if bias > 0.0 {
            (end_value - start_value) * time.powf(1.0 + bias / 5.0) + start_value * sub_multiplier
        } else if bias < 0.0 {
            (end_value - start_value) * (1.0 - (1.0 - time).powf(-bias / 5.0 + 1.0)) + start_value * sub_multiplier
        } else {
            (end_value - start_value) * time + start_value * sub_multiplier
        }
    }

    pub fn interpolate(&self, other: &Frame, frame_index: usize) -> Frame {
        let time = 1.0 / (other.frame_index as f32 - self.frame_index as f32) * (frame_index as f32 - self.frame_index as f32);
        let sub_mult = 1.0;

        // TODO: angle bias
        let angle = Self::ease_interpolate(self.angle, other.angle, time, 0.0, sub_mult);
        let color = [
            (other.color[0] - self.color[0]) * time + self.color[0] * sub_mult,
            (other.color[1] - self.color[1]) * time + self.color[1] * sub_mult,
            (other.color[2] - self.color[2]) * time + self.color[2] * sub_mult,
            (other.color[3] - self.color[3]) * time + self.color[3] * sub_mult,
        ];

        let uv = (0..8)
            .map(|index| (other.uv[index] - self.uv[index]) * time + self.uv[index] * sub_mult)
            .next_chunk()
            .unwrap();

        // TODO: scale bias
        let xy = (0..8)
            .map(|index| Self::ease_interpolate(self.xy[index], other.xy[index], time, 0.0, sub_mult))
            .next_chunk()
            .unwrap();

        // TODO: additional logic for animation type 2 and 3
        let texture_index = self.texture_index;

        // TODO: bezier curves
        let offset_x = (other.offset.x - self.offset.x) * time + self.offset.x * sub_mult;
        let offset_y = (other.offset.y - self.offset.y) * time + self.offset.y * sub_mult;

        Frame {
            frame_index: frame_index as i32,
            frame_type: self.frame_type,
            offset: Vector2::new(offset_x, offset_y),
            uv,
            xy,
            texture_index,
            animation_type: self.animation_type,
            delay: self.delay,
            angle,
            color,
            source_alpha: self.source_alpha,
            destination_alpha: self.destination_alpha,
            mt_present: self.mt_present,
        }
    }
}

#[derive(Debug, FromBytes, PrototypeElement)]
struct LayerData {
    pub texture_count: i32,
    #[repeating(self.texture_count)]
    pub texture_names: Vec<TextureName>,
    pub frame_count: i32,
    #[repeating(self.frame_count)]
    pub frames: Vec<Frame>,
}

#[derive(Debug, FromBytes, PrototypeElement)]
struct EffectData {
    pub version: Version<MajorFirst>,
    pub _skip0: [u8; 2],
    pub frames_per_second: u32,
    pub max_key: u32,
    pub layer_count: u32,
    pub _skip1: [u8; 16],
    #[repeating(self.layer_count)]
    pub layers: Vec<LayerData>,
}

#[derive(Default)]
pub struct EffectLoader {
    cache: HashMap<String, Arc<Effect>>,
}

pub struct Layer {
    pub textures: Vec<Arc<ImageView>>,
    pub frames: Vec<Frame>,
    pub indices: Vec<Option<usize>>,
}

impl Layer {
    fn interpolate(&self, frame_timer: &FrameTimer) -> Option<Frame> {
        if let Some(frame_index) = self.indices[frame_timer.current_frame] {
            if let Some(next_frame) = self.frames.get(frame_index + 2) {
                return Some(self.frames[frame_index].interpolate(next_frame, frame_timer.current_frame));
            } else {
                return Some(self.frames[frame_index].clone());
            }
        }

        None
    }
}

pub struct Effect {
    frames_per_second: usize,
    max_key: usize,
    layers: Vec<Layer>,
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

impl Effect {
    pub fn new_frame_timer(&self) -> FrameTimer {
        FrameTimer {
            total_timer: 0.0,
            frames_per_second: self.frames_per_second,
            max_key: self.max_key,
            current_frame: 0,
        }
    }

    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        frame_timer: &FrameTimer,
        position: Vector3<f32>,
    ) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let world_to_screen_matrix = projection_matrix * view_matrix;

        let clip_space_position = world_to_screen_matrix * position.extend(1.0);
        let screen_space_position = Vector2::new(
            clip_space_position.x / clip_space_position.w + 1.0,
            clip_space_position.y / clip_space_position.w + 1.0,
        );

        for layer in &self.layers {
            let Some(frame) = layer.interpolate(frame_timer) else {
                continue;
            };

            if frame.texture_index < 0.0 || frame.texture_index as usize > layer.textures.len() {
                continue;
            }

            renderer.render_effect(
                render_target,
                layer.textures[frame.texture_index as usize].clone(),
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
                screen_space_position,
                frame.offset,
                frame.angle,
                Color::rgba(
                    frame.color[0] / 255.0,
                    frame.color[1] / 255.0,
                    frame.color[2] / 255.0,
                    frame.color[3] / 255.0,
                ),
            );
        }
    }
}

impl EffectLoader {
    fn load(
        &mut self,
        path: &str,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Effect>, String> {
        #[cfg(feature = "debug")]
        let timer = korangar_debug::Timer::new_dynamic(format!("load effect from {}", path.magenta()));

        let bytes = game_file_loader.get(&format!("data\\texture\\effect\\{path}"))?;
        let mut byte_stream: ByteStream<Option<InternalVersion>> = ByteStream::without_metadata(&bytes);

        if <[u8; 4]>::from_bytes(&mut byte_stream).unwrap() != [b'S', b'T', b'R', b'M'] {
            return Err(format!("failed to read magic number from {path}"));
        }

        // TODO: Add fallback
        let effect_data = EffectData::from_bytes(&mut byte_stream).unwrap();

        let prefix = match path.chars().rev().position(|character| character == '\\') {
            Some(offset) => path.split_at(path.len() - offset).0,
            None => "",
        };

        let effect = Arc::new(Effect {
            frames_per_second: effect_data.frames_per_second as usize,
            max_key: effect_data.max_key as usize,
            layers: effect_data
                .layers
                .into_iter()
                .map(|layer_data| Layer {
                    textures: layer_data
                        .texture_names
                        .into_iter()
                        .map(|name| {
                            let path = format!("effect\\{}{}", prefix, name.name);
                            texture_loader.get(&path, game_file_loader).unwrap()
                        })
                        .collect(),
                    indices: {
                        let frame_count = layer_data.frames.len();
                        let mut map = Vec::with_capacity(frame_count);
                        let mut list_index = 0;

                        if frame_count > 0 {
                            let mut previous = None;

                            for _ in 0..layer_data.frames[0].frame_index {
                                map.push(None);
                                list_index += 1;
                            }

                            for (index, frame) in layer_data.frames.iter().skip(1).enumerate() {
                                for _ in list_index..frame.frame_index as usize {
                                    map.push(previous);
                                    list_index += 1;
                                }

                                previous = Some(index);
                            }

                            // TODO: conditional
                            map.push(previous);
                            list_index += 1;
                        }

                        for _ in list_index..effect_data.max_key as usize {
                            map.push(None)
                        }

                        map
                    },
                    frames: layer_data.frames,
                })
                .collect(),
        });

        self.cache.insert(path.to_string(), effect.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(effect)
    }

    pub fn get(
        &mut self,
        path: &str,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
    ) -> Result<Arc<Effect>, String> {
        match self.cache.get(path) {
            Some(effect) => Ok(effect.clone()),
            None => self.load(path, game_file_loader, texture_loader),
        }
    }
}

pub enum EffectCenter {
    Entity(EntityId, Vector3<f32>),
    Position(Vector3<f32>),
}

impl EffectCenter {
    fn to_position(&self) -> Vector3<f32> {
        match self {
            EffectCenter::Entity(_, position) | EffectCenter::Position(position) => *position,
        }
    }
}

pub trait EffectBase {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool;

    fn mark_for_deletion(&mut self);

    fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera);
}

#[derive(new)]
pub struct EffectWithLight {
    effect: Arc<Effect>,
    frame_timer: FrameTimer,
    center: EffectCenter,
    effect_offset: Vector3<f32>,
    light_offset: Vector3<f32>,
    light_color: Color,
    light_intensity: f32,
    repeating: bool,
    #[new(default)]
    current_light_intensity: f32,
    #[new(default)]
    gets_deleted: bool,
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

    fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera) {
        if !self.gets_deleted {
            self.effect.render(
                render_target,
                renderer,
                camera,
                &self.frame_timer,
                self.center.to_position() + self.effect_offset,
            );
        }

        renderer.point_light(
            render_target,
            camera,
            self.center.to_position() + self.light_offset,
            self.light_color,
            self.current_light_intensity,
        );
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

    pub fn render(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera) {
        self.effects
            .iter()
            .for_each(|(effect, _)| effect.render(render_target, renderer, camera));
    }
}
