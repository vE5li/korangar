mod skill_effect;

use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
use korangar_interface::application::Clip;
use ragnarok_packets::{EntityId, QuestColor, QuestEffectPacket};
use rand_aes::tls::rand_f32;

pub use self::skill_effect::*;
use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::{FontSize, ImageType, Scaling, Sprite, TextureLoader};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Actions, Camera};
use crate::{Entity, Map};

pub trait Particle {
    fn update(&mut self, delta_time: f32) -> bool;

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize);
}

fn random_velocity() -> f32 {
    rand_f32() * 40.0 - 20.0
}

/// Particle feedback derived from a signed `ZC_NOTIFY_SKILL` damage value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillDamageDisplay {
    /// Negative values are protocol sentinels and produce no number.
    Suppressed,
    Miss,
    Damage {
        amount: u32,
        is_critical: bool,
    },
}

impl SkillDamageDisplay {
    pub fn from_packet(damage: i32, action: i8) -> Self {
        match damage {
            ..=-1 => Self::Suppressed,
            0 => Self::Miss,
            amount => Self::Damage {
                amount: amount as u32,
                // e_damage_type::DMG_CRITICAL and
                // e_damage_type::DMG_MULTI_HIT_CRITICAL.
                is_critical: matches!(action, 10 | 13),
            },
        }
    }

    pub fn split_for_hits(self, hit_count: i16) -> Vec<Self> {
        let Self::Damage { amount, is_critical } = self else {
            return (self != Self::Suppressed).then_some(self).into_iter().collect();
        };
        let hit_count = usize::try_from(hit_count).unwrap_or(1).clamp(1, 32);
        let base_amount = amount / hit_count as u32;
        let remainder = amount % hit_count as u32;

        (0..hit_count)
            .map(|index| Self::Damage {
                amount: base_amount + u32::from(index < remainder as usize),
                is_critical,
            })
            .collect()
    }
}

#[cfg(test)]
mod skill_damage_display_tests {
    use super::SkillDamageDisplay;

    #[test]
    fn suppresses_signed_sentinels_and_distinguishes_misses() {
        assert_eq!(SkillDamageDisplay::from_packet(-30000, 0), SkillDamageDisplay::Suppressed);
        assert_eq!(SkillDamageDisplay::from_packet(-1, 0), SkillDamageDisplay::Suppressed);
        assert_eq!(SkillDamageDisplay::from_packet(0, 0), SkillDamageDisplay::Miss);
    }

    #[test]
    fn only_critical_actions_receive_critical_styling() {
        assert_eq!(SkillDamageDisplay::from_packet(123, 10), SkillDamageDisplay::Damage {
            amount: 123,
            is_critical: true,
        });
        assert_eq!(SkillDamageDisplay::from_packet(123, 13), SkillDamageDisplay::Damage {
            amount: 123,
            is_critical: true,
        });
        assert_eq!(SkillDamageDisplay::from_packet(123, 127), SkillDamageDisplay::Damage {
            amount: 123,
            is_critical: false,
        });
    }

    #[test]
    fn multi_hit_damage_is_split_without_losing_the_total() {
        let displays = SkillDamageDisplay::Damage {
            amount: 103,
            is_critical: false,
        }
        .split_for_hits(4);
        let total: u32 = displays
            .iter()
            .map(|display| match display {
                SkillDamageDisplay::Damage { amount, .. } => *amount,
                _ => 0,
            })
            .sum();

        assert_eq!(displays.len(), 4);
        assert_eq!(total, 103);
        assert_eq!(SkillDamageDisplay::Miss.split_for_hits(10), vec![SkillDamageDisplay::Miss]);
        assert!(SkillDamageDisplay::Suppressed.split_for_hits(10).is_empty());
    }
}

pub struct DamageNumber {
    position: Point3<f32>,
    damage_amount: String,
    velocity_y: f32,
    velocity_x: f32,
    velocity_z: f32,
    timer: f32,
    is_critical: bool,
}

impl DamageNumber {
    pub fn new(position: Point3<f32>, damage_amount: String, is_critical: bool) -> Self {
        Self {
            position,
            damage_amount,
            velocity_y: 50.0,
            velocity_x: random_velocity(),
            velocity_z: random_velocity(),
            timer: 0.6,
            is_critical,
        }
    }
}

impl Particle for DamageNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 200.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;
        self.position.x += self.velocity_x * delta_time;
        self.position.z += self.velocity_z * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        let color = match self.is_critical {
            true => Color::rgb_u8(255, 180, 0),
            false => Color::WHITE,
        };

        renderer.render_damage_text(&self.damage_amount, final_position, color, FontSize(16.0));
    }
}

pub struct Miss {
    position: Point3<f32>,
    timer: f32,
}

impl Miss {
    pub fn new(position: Point3<f32>) -> Self {
        Self { position, timer: 0.6 }
    }
}

impl Particle for Miss {
    fn update(&mut self, delta_time: f32) -> bool {
        self.position.y += (self.timer - 0.1).max(0.0) * 70.0 * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };
        let alpha = (self.timer * 10.0).min(1.0);

        renderer.render_damage_text("miss", final_position, Color::rgba(1.0, 0.0, 0.0, alpha), FontSize(20.0));
    }
}

pub struct HealNumber {
    position: Point3<f32>,
    heal_amount: String,
    velocity_y: f32,
    timer: f32,
}

impl HealNumber {
    pub fn new(position: Point3<f32>, heal_amount: String) -> Self {
        Self {
            position,
            heal_amount,
            velocity_y: 50.0,
            timer: 1.0,
        }
    }
}

impl Particle for HealNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 50.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_damage_text(&self.heal_amount, final_position, Color::rgb_u8(30, 255, 30), FontSize(16.0));
    }
}

/// An emotion played above an entity, rendered from the animation frames of
/// `emotion.act`. Despawns after playing through the animation once.
pub struct Emote {
    position: Point3<f32>,
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
    emotion_id: usize,
    timer: f32,
}

impl Emote {
    /// Offset above the entity position so the emote hovers over the head.
    const POSITION_OFFSET: Vector3<f32> = Vector3::new(0.0, 30.0, 0.0);

    pub fn new(entity_position: Point3<f32>, sprite: Arc<Sprite>, actions: Arc<Actions>, emotion_id: usize) -> Self {
        Self {
            position: entity_position + Self::POSITION_OFFSET,
            sprite,
            actions,
            emotion_id,
            timer: 0.0,
        }
    }

    /// Milliseconds each animation frame is displayed for, scaled by the
    /// per-action delay of the loaded actions.
    fn frame_time(&self) -> f32 {
        let delay = self.actions.delays[self.emotion_id % self.actions.delays.len()];
        delay * 50.0
    }

    fn frame_count(&self) -> usize {
        let action = &self.actions.actions[self.emotion_id % self.actions.actions.len()];
        action.motions.len()
    }
}

impl Particle for Emote {
    fn update(&mut self, delta_time: f32) -> bool {
        self.timer += delta_time * 1000.0;
        self.timer < self.frame_count() as f32 * self.frame_time()
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        let frame = (self.timer / self.frame_time()) as usize;

        self.actions.render_sprite_frame(
            renderer,
            &self.sprite,
            self.emotion_id,
            frame,
            final_position,
            ScreenClip::unbound(),
            Color::WHITE,
            1.0,
        );
    }
}

/// A one-shot or repeating ACT/SPR animation that follows an entity.
pub struct EntityAttachedSprite {
    entity_id: EntityId,
    attachment_key: Option<u32>,
    position: Point3<f32>,
    position_offset: Vector3<f32>,
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
    action_index: usize,
    timer: f32,
    repeating: bool,
    maximum_duration: Option<f32>,
    scaling: f32,
}

impl EntityAttachedSprite {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        entity_id: EntityId,
        attachment_key: Option<u32>,
        entity_position: Point3<f32>,
        position_offset: Vector3<f32>,
        sprite: Arc<Sprite>,
        actions: Arc<Actions>,
        action_index: usize,
        repeating: bool,
        maximum_duration: Option<f32>,
        scaling: f32,
    ) -> Self {
        Self {
            entity_id,
            attachment_key,
            position: entity_position,
            position_offset,
            sprite,
            actions,
            action_index,
            timer: 0.0,
            repeating,
            maximum_duration,
            scaling,
        }
    }

    fn action_index(&self) -> Option<usize> {
        (!self.actions.actions.is_empty()).then(|| self.action_index % self.actions.actions.len())
    }

    fn frame_time(&self) -> Option<f32> {
        let action_index = self.action_index()?;
        let delay = self.actions.delays.get(action_index).copied().unwrap_or_default();
        Some((delay * 50.0).max(1.0))
    }

    fn frame_count(&self) -> Option<usize> {
        let action_index = self.action_index()?;
        let frame_count = self.actions.actions[action_index].motions.len();
        (frame_count > 0).then_some(frame_count)
    }

    fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool {
        if let Some((_, position)) = local_entity.filter(|(entity_id, _)| *entity_id == self.entity_id) {
            self.position = position;
        } else if let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == self.entity_id) {
            self.position = entity.get_position();
        }

        self.timer += delta_time * 1000.0;

        if self
            .maximum_duration
            .is_some_and(|maximum_duration| self.timer >= maximum_duration * 1000.0)
        {
            return false;
        }

        let Some(frame_count) = self.frame_count() else {
            return false;
        };
        let Some(frame_time) = self.frame_time() else {
            return false;
        };

        self.repeating || self.timer < frame_count as f32 * frame_time
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        let Some(action_index) = self.action_index() else {
            return;
        };
        let Some(frame_count) = self.frame_count() else {
            return;
        };
        let Some(frame_time) = self.frame_time() else {
            return;
        };

        let clip_space_position = camera.view_projection_matrix() * (self.position + self.position_offset).to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };
        let frame = (self.timer / frame_time) as usize;
        let frame = match self.repeating {
            true => frame % frame_count,
            false => frame.min(frame_count.saturating_sub(1)),
        };

        self.actions.render_sprite_frame(
            renderer,
            &self.sprite,
            action_index,
            frame,
            final_position,
            ScreenClip::unbound(),
            Color::WHITE,
            self.scaling,
        );
    }
}

pub struct QuestIcon {
    position: Point3<f32>,
    texture: Arc<Texture>,
    color: Color,
}

impl QuestIcon {
    pub fn new(texture_loader: &TextureLoader, map: &Map, quest_effect: QuestEffectPacket) -> Option<Self> {
        // TODO: Use the height of the entity as offset.
        let icon_offset = Vector3::new(0.0, 25.0, 0.0);
        let Some(entity_position) = map.get_world_position(quest_effect.position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] quest icon is out of map bounds", "error".red());
            return None;
        };

        let position = entity_position + icon_offset;
        let effect_id = quest_effect.effect as usize;
        let texture = texture_loader
            .get_or_load(
                &format!("유저인터페이스\\minimap\\quest_{}_{}.bmp", effect_id, 1), /* 1 - 3 */
                ImageType::Color,
            )
            .unwrap();
        let color = match quest_effect.color {
            QuestColor::Yellow => Color::rgb_u8(200, 200, 30),
            QuestColor::Orange => Color::rgb_u8(200, 100, 30),
            QuestColor::Green => Color::rgb_u8(30, 200, 30),
            QuestColor::Purple => Color::rgb_u8(200, 30, 200),
        };

        Some(Self { position, texture, color })
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize, scaling_factor: f32) {
        let clip_space_position = camera.view_projection_matrix() * self.position.to_homogeneous();
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_sprite(
            self.texture.clone(),
            final_position - ScreenSize::uniform(15.0 * scaling_factor),
            ScreenSize::uniform(30.0 * scaling_factor),
            ScreenClip::unbound(),
            self.color,
            true,
        );
    }
}

#[derive(Default)]
pub struct ParticleHolder {
    particles: Vec<Box<dyn Particle + Send + Sync>>,
    entity_particles: Vec<Box<dyn EntityParticle + Send + Sync>>,
    attached_sprites: Vec<EntityAttachedSprite>,
    cast_rings: Vec<CastRing>,
    quest_icons: HashMap<EntityId, QuestIcon>,
}

impl ParticleHolder {
    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        self.particles.push(particle);
    }

    pub fn spawn_entity_particle(&mut self, particle: Box<dyn EntityParticle + Send + Sync>) {
        self.entity_particles.push(particle);
    }

    pub fn spawn_cast_ring(&mut self, ring: CastRing) {
        // A new cast replaces the caster's ring of the same kind.
        self.cast_rings
            .retain(|existing| existing.caster_entity_id() != ring.caster_entity_id() || existing.kind() != ring.kind());
        self.cast_rings.push(ring);
    }

    /// Removes every ring of the caster's current cast: the aura at their
    /// feet and the lock-on at their target.
    pub fn remove_cast_rings(&mut self, caster_entity_id: EntityId) {
        self.cast_rings.retain(|ring| ring.caster_entity_id() != caster_entity_id);
    }

    pub fn spawn_attached_sprite(&mut self, sprite: EntityAttachedSprite) {
        if let Some(attachment_key) = sprite.attachment_key {
            self.remove_attached_sprite(sprite.entity_id, attachment_key);
        }
        self.attached_sprites.push(sprite);
    }

    pub fn remove_attached_sprite(&mut self, entity_id: EntityId, attachment_key: u32) {
        self.attached_sprites
            .retain(|sprite| sprite.entity_id != entity_id || sprite.attachment_key != Some(attachment_key));
    }

    pub fn has_attached_sprite(&self, entity_id: EntityId, attachment_key: u32) -> bool {
        self.attached_sprites
            .iter()
            .any(|sprite| sprite.entity_id == entity_id && sprite.attachment_key == Some(attachment_key))
    }

    pub fn add_quest_icon(&mut self, texture_loader: &TextureLoader, map: &Map, quest_effect: QuestEffectPacket) {
        let entity_id = quest_effect.entity_id;

        if let Some(quest_icon) = QuestIcon::new(texture_loader, map, quest_effect) {
            self.quest_icons.insert(entity_id, quest_icon);
        }
    }

    pub fn remove_quest_icon(&mut self, entity_id: EntityId) {
        self.quest_icons.remove(&entity_id);
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.entity_particles.clear();
        self.attached_sprites.clear();
        self.cast_rings.clear();
        self.quest_icons.clear();
    }

    #[allow(dead_code)]
    pub fn update(&mut self, delta_time: f32) {
        self.update_with_local_entity(&[], None, delta_time);
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("update particles"))]
    pub fn update_with_local_entity(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) {
        self.particles.retain_mut(|particle| particle.update(delta_time));
        self.entity_particles
            .retain_mut(|particle| particle.update(entities, local_entity, delta_time));
        self.attached_sprites
            .retain_mut(|sprite| sprite.update(entities, local_entity, delta_time));
        self.cast_rings.retain_mut(|ring| ring.update(entities, local_entity, delta_time));
    }

    /// Cast rings render through the effect pipeline, which can rotate and
    /// blend additively, unlike the interface sprite path the other
    /// particles use.
    pub fn render_cast_rings(&self, renderer: &mut crate::renderer::EffectRenderer, camera: &dyn Camera) {
        self.cast_rings.iter().for_each(|ring| ring.render(renderer, camera));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render particles"))]
    pub fn render(
        &self,
        renderer: &GameInterfaceRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
        scaling: Scaling,
        entities: &[Entity],
    ) {
        self.particles
            .iter()
            .for_each(|particle| particle.render(renderer, camera, window_size));
        self.entity_particles
            .iter()
            .for_each(|particle| particle.render(renderer, camera, window_size));
        self.attached_sprites
            .iter()
            .for_each(|sprite| sprite.render(renderer, camera, window_size));

        entities
            .iter()
            .filter_map(|entity| self.quest_icons.get(&entity.get_entity_id()))
            .for_each(|quest_icon| quest_icon.render(renderer, camera, window_size, scaling.get_factor()));
    }
}
