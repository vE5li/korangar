use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Vector2, Vector3};
use derive_new::new;
use korangar_interface::application::ClipTraitExt;
use ragnarok_packets::{EntityId, QuestColor, QuestEffectPacket};
use rand::{thread_rng, Rng};
use wgpu::RenderPass;

use crate::graphics::*;
use crate::interface::layout::{ScreenClip, ScreenPosition, ScreenSize};
use crate::loaders::{GameFileLoader, TextureLoader};
use crate::world::*;

pub trait Particle {
    fn update(&mut self, delta_time: f32) -> bool;

    fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
    );
}

#[derive(new)]
pub struct DamageNumber {
    position: Vector3<f32>,
    damage_amount: String,
    #[new(value = "50.0")]
    velocity_y: f32,
    #[new(value = "thread_rng().gen_range(-20.0..20.0)")]
    velocity_x: f32,
    #[new(value = "thread_rng().gen_range(-20.0..20.0)")]
    velocity_z: f32,
    #[new(value = "0.6")]
    timer: f32,
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

    fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
    ) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let clip_space_position = (projection_matrix * view_matrix) * self.position.extend(1.0);
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_damage_text(
            render_target,
            render_pass,
            &self.damage_amount,
            final_position,
            Color::monochrome_u8(255),
            16.0,
        );
    }
}

#[derive(new)]
pub struct HealNumber {
    position: Vector3<f32>,
    heal_amount: String,
    #[new(value = "50.0")]
    velocity_y: f32,
    #[new(value = "1.0")]
    timer: f32,
}

impl Particle for HealNumber {
    fn update(&mut self, delta_time: f32) -> bool {
        self.velocity_y -= 50.0 * delta_time;

        self.position.y += self.velocity_y * delta_time;

        self.timer -= delta_time;
        self.timer > 0.0
    }

    fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
    ) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let clip_space_position = (projection_matrix * view_matrix) * self.position.extend(1.0);
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_damage_text(
            render_target,
            render_pass,
            &self.heal_amount,
            final_position,
            Color::rgb_u8(30, 255, 30),
            16.0,
        );
    }
}

pub struct QuestIcon {
    position: Vector3<f32>,
    texture: Arc<Texture>,
    color: Color,
}

impl QuestIcon {
    pub fn new(
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        map: &Map,
        quest_effect: QuestEffectPacket,
    ) -> Self {
        let position = map.get_world_position(Vector2::new(quest_effect.position.x as usize, quest_effect.position.y as usize))
            + Vector3::new(0.0, 25.0, 0.0); // TODO: get height of the entity as offset
        let effect_id = quest_effect.effect as usize;
        let texture = texture_loader
            .get(
                &format!("À¯ÀúÀÎÅÍÆäÀÌ½º\\minimap\\quest_{}_{}.bmp", effect_id, 1 /* 1 - 3 */),
                game_file_loader,
            )
            .unwrap();
        let color = match quest_effect.color {
            QuestColor::Yellow => Color::rgb_u8(200, 200, 30),
            QuestColor::Orange => Color::rgb_u8(200, 100, 30),
            QuestColor::Green => Color::rgb_u8(30, 200, 30),
            QuestColor::Purple => Color::rgb_u8(200, 30, 200),
        };

        Self { position, texture, color }
    }

    fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
        scaling_factor: f32,
    ) {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let clip_space_position = (projection_matrix * view_matrix) * self.position.extend(1.0);
        let screen_position = camera.clip_to_screen_space(clip_space_position);
        let final_position = ScreenPosition {
            left: screen_position.x * window_size.width,
            top: screen_position.y * window_size.height,
        };

        renderer.render_sprite(
            render_target,
            render_pass,
            &self.texture,
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
    quest_icons: HashMap<EntityId, QuestIcon>,
}

impl ParticleHolder {
    pub fn spawn_particle(&mut self, particle: Box<dyn Particle + Send + Sync>) {
        self.particles.push(particle);
    }

    pub fn add_quest_icon(
        &mut self,
        game_file_loader: &mut GameFileLoader,
        texture_loader: &mut TextureLoader,
        map: &Map,
        quest_effect: QuestEffectPacket,
    ) {
        self.quest_icons.insert(
            quest_effect.entity_id,
            QuestIcon::new(game_file_loader, texture_loader, map, quest_effect),
        );
    }

    pub fn remove_quest_icon(&mut self, entity_id: EntityId) {
        self.quest_icons.remove(&entity_id);
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.quest_icons.clear();
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("update particles"))]
    pub fn update(&mut self, delta_time: f32) {
        self.particles.retain_mut(|particle| particle.update(delta_time));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile("render particles"))]
    pub fn render(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        render_pass: &mut RenderPass,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: ScreenSize,
        scaling_factor: f32,
        entities: &[Entity],
    ) {
        self.particles
            .iter()
            .for_each(|particle| particle.render(render_target, render_pass, renderer, camera, window_size));

        entities
            .iter()
            .filter_map(|entity| self.quest_icons.get(&entity.get_entity_id()))
            .for_each(|quest_icon| quest_icon.render(render_target, render_pass, renderer, camera, window_size, scaling_factor));
    }
}
