use std::f32::consts::PI;
use std::sync::Arc;

use cgmath::{Point3, Vector3};
use korangar_interface::application::Clip;
use ragnarok_packets::EntityId;

use crate::Entity;
use crate::graphics::{Color, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::renderer::{GameInterfaceRenderer, SpriteRenderer};
use crate::world::Camera;

/// The classic 128x128 Cold Bolt shard stored in `data.grf`.
///
/// Paths exposed by this module are relative to `data\texture`, matching
/// [`crate::loaders::TextureLoader`].
pub const ICE_ARROW_TEXTURE_PATH: &str = "effect\\icearrow.tga";

/// A 128x128 frost cloud selected from `data.grf` for the procedural Frost
/// Diver approximation. The archive has no skill-specific world animation.
pub const FROST_DIVER_TEXTURE_PATH: &str = "effect\\ice.tga";

/// A 64x64 ice fragment used for classic water-skill impacts.
pub const ICE_IMPACT_TEXTURE_PATH: &str = "effect\\iceparticle.bmp";

pub const COLD_BOLT_PARTICLE_DURATION: f32 = 0.44;
pub const FROST_DIVER_TRAVEL_DURATION: f32 = 0.64;
pub const FROST_DIVER_IMPACT_DURATION: f32 = 0.46;

/// A particle that may follow one or more entities while it is alive.
///
/// The separate local entity snapshot keeps player-attached effects working
/// when the local player is not part of the regular entity slice. A snapshot,
/// rather than a reference, also avoids borrowing the client state twice.
pub trait EntityParticle {
    fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool;

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize);
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Timeline {
    elapsed: f32,
    duration: f32,
}

impl Timeline {
    fn new(duration: f32) -> Self {
        Self {
            elapsed: 0.0,
            duration: duration.max(f32::EPSILON),
        }
    }

    fn advance(&mut self, delta_time: f32) -> bool {
        self.elapsed += delta_time.max(0.0);
        self.elapsed < self.duration
    }

    fn progress(&self) -> f32 {
        normalized_progress(self.elapsed, self.duration)
    }
}

fn normalized_progress(elapsed: f32, duration: f32) -> f32 {
    if !elapsed.is_finite() || !duration.is_finite() || duration <= 0.0 {
        return 0.0;
    }

    (elapsed / duration).clamp(0.0, 1.0)
}

fn phase_progress(elapsed: f32, start: f32, duration: f32) -> f32 {
    normalized_progress(elapsed - start, duration)
}

fn smoothstep(progress: f32) -> f32 {
    let progress = progress.clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

fn resolve_entity_position(entity_id: EntityId, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>) -> Option<Point3<f32>> {
    local_entity
        .filter(|(local_entity_id, _)| *local_entity_id == entity_id)
        .map(|(_, position)| position)
        .or_else(|| {
            entities
                .iter()
                .find(|entity| entity.get_entity_id() == entity_id)
                .map(|entity| entity.get_position())
        })
}

fn to_screen_position(position: Point3<f32>, camera: &dyn Camera, window_size: ScreenSize) -> ScreenPosition {
    let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();
    let screen_position = camera.clip_to_screen_space(clip_space_position);

    ScreenPosition {
        left: screen_position.x * window_size.width,
        top: screen_position.y * window_size.height,
    }
}

fn render_centered(
    renderer: &GameInterfaceRenderer,
    texture: Arc<Texture>,
    position: Point3<f32>,
    camera: &dyn Camera,
    window_size: ScreenSize,
    size: ScreenSize,
    color: Color,
) {
    let center = to_screen_position(position, camera, window_size);
    let top_left = ScreenPosition {
        left: center.left - size.width * 0.5,
        top: center.top - size.height * 0.5,
    };

    renderer.render_sprite(texture, top_left, size, ScreenClip::unbound(), color, true);
}

fn cold_bolt_offset(bolt_index: usize) -> Vector3<f32> {
    const OFFSETS: [(f32, f32); 8] = [
        (0.0, 0.0),
        (-5.0, 2.0),
        (5.0, -2.0),
        (-3.0, -5.0),
        (3.0, 5.0),
        (-7.0, -3.0),
        (7.0, 3.0),
        (0.0, 7.0),
    ];

    let (x, z) = OFFSETS[bolt_index % OFFSETS.len()];
    let ring = 1.0 + (bolt_index / OFFSETS.len()) as f32 * 0.35;
    Vector3::new(x * ring, 0.0, z * ring)
}

/// One descending Cold Bolt shard followed by an ice impact.
///
/// Multi-hit skills create one particle per scheduled hit. Each particle
/// follows its target for its complete lifetime.
pub struct ColdBoltParticle {
    target_entity_id: EntityId,
    target_position: Point3<f32>,
    arrow_texture: Arc<Texture>,
    impact_texture: Arc<Texture>,
    offset: Vector3<f32>,
    timeline: Timeline,
}

impl ColdBoltParticle {
    pub fn new(
        target_entity_id: EntityId,
        target_position: Point3<f32>,
        arrow_texture: Arc<Texture>,
        impact_texture: Arc<Texture>,
        bolt_index: usize,
    ) -> Self {
        Self {
            target_entity_id,
            target_position,
            arrow_texture,
            impact_texture,
            offset: cold_bolt_offset(bolt_index),
            timeline: Timeline::new(COLD_BOLT_PARTICLE_DURATION),
        }
    }

    pub fn impact(
        target_entity_id: EntityId,
        target_position: Point3<f32>,
        arrow_texture: Arc<Texture>,
        impact_texture: Arc<Texture>,
    ) -> Self {
        let mut particle = Self::new(target_entity_id, target_position, arrow_texture, impact_texture, 0);
        particle.timeline.elapsed = COLD_BOLT_PARTICLE_DURATION * 0.68;
        particle
    }
}

impl EntityParticle for ColdBoltParticle {
    fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool {
        if let Some(position) = resolve_entity_position(self.target_entity_id, entities, local_entity) {
            self.target_position = position;
        }

        self.timeline.advance(delta_time)
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        const IMPACT_START: f32 = 0.68;

        let progress = self.timeline.progress();
        let descent_progress = smoothstep((progress / IMPACT_START).clamp(0.0, 1.0));

        if progress < IMPACT_START {
            let position = self.target_position
                + self.offset * (1.0 - descent_progress * 0.55)
                + Vector3::new(0.0, 38.0 * (1.0 - descent_progress) + 5.0, 0.0);
            let alpha = (1.0 - descent_progress * 0.25).clamp(0.0, 1.0);

            render_centered(
                renderer,
                self.arrow_texture.clone(),
                position,
                camera,
                window_size,
                ScreenSize { width: 28.0, height: 14.0 },
                Color::rgba(0.68, 0.90, 1.0, alpha),
            );
        }

        if progress >= IMPACT_START {
            let impact_progress = smoothstep((progress - IMPACT_START) / (1.0 - IMPACT_START));
            let size = 14.0 + impact_progress * 34.0;
            let alpha = (1.0 - impact_progress).clamp(0.0, 1.0);

            render_centered(
                renderer,
                self.impact_texture.clone(),
                self.target_position + Vector3::new(0.0, 5.0, 0.0),
                camera,
                window_size,
                ScreenSize { width: size, height: size },
                Color::rgba(0.62, 0.88, 1.0, alpha),
            );
        }
    }
}

fn frost_diver_position(source: Point3<f32>, destination: Point3<f32>, progress: f32) -> Point3<f32> {
    if progress <= 0.0 {
        return source;
    }
    if progress >= 1.0 {
        return destination;
    }

    let progress = smoothstep(progress);
    let mut position = source + (destination - source) * progress;
    position.y += (PI * progress).sin() * 9.0;
    position
}

/// A Frost Diver projectile and its destination impact.
///
/// Entity IDs are optional so callers can either use server snapshots or
/// allow the projectile endpoints to follow live entities.
pub struct FrostDiverParticle {
    source_entity_id: Option<EntityId>,
    destination_entity_id: Option<EntityId>,
    source_position: Point3<f32>,
    destination_position: Point3<f32>,
    projectile_texture: Arc<Texture>,
    impact_texture: Option<Arc<Texture>>,
    timeline: Timeline,
}

impl FrostDiverParticle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_entity_id: Option<EntityId>,
        destination_entity_id: Option<EntityId>,
        source_position: Point3<f32>,
        destination_position: Point3<f32>,
        projectile_texture: Arc<Texture>,
        impact_texture: Arc<Texture>,
    ) -> Self {
        Self {
            source_entity_id,
            destination_entity_id,
            source_position,
            destination_position,
            projectile_texture,
            impact_texture: Some(impact_texture),
            timeline: Timeline::new(FROST_DIVER_TRAVEL_DURATION + FROST_DIVER_IMPACT_DURATION),
        }
    }

    pub fn travel_only(
        source_position: Point3<f32>,
        destination_entity_id: EntityId,
        destination_position: Point3<f32>,
        projectile_texture: Arc<Texture>,
    ) -> Self {
        Self {
            source_entity_id: None,
            destination_entity_id: Some(destination_entity_id),
            source_position,
            destination_position,
            projectile_texture,
            impact_texture: None,
            timeline: Timeline::new(FROST_DIVER_TRAVEL_DURATION),
        }
    }

    pub fn impact(destination_entity_id: EntityId, destination_position: Point3<f32>, impact_texture: Arc<Texture>) -> Self {
        let mut particle = Self::new(
            None,
            Some(destination_entity_id),
            destination_position,
            destination_position,
            impact_texture.clone(),
            impact_texture,
        );
        particle.timeline.elapsed = FROST_DIVER_TRAVEL_DURATION;
        particle
    }

    fn travel_progress(&self) -> f32 {
        normalized_progress(self.timeline.elapsed, FROST_DIVER_TRAVEL_DURATION)
    }

    fn impact_progress(&self) -> f32 {
        phase_progress(self.timeline.elapsed, FROST_DIVER_TRAVEL_DURATION, FROST_DIVER_IMPACT_DURATION)
    }
}

impl EntityParticle for FrostDiverParticle {
    fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool {
        if let Some(entity_id) = self.source_entity_id
            && let Some(position) = resolve_entity_position(entity_id, entities, local_entity)
        {
            self.source_position = position;
        }
        if let Some(entity_id) = self.destination_entity_id
            && let Some(position) = resolve_entity_position(entity_id, entities, local_entity)
        {
            self.destination_position = position;
        }

        self.timeline.advance(delta_time)
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        if self.timeline.elapsed < FROST_DIVER_TRAVEL_DURATION {
            let source = self.source_position + Vector3::new(0.0, 7.0, 0.0);
            let destination = self.destination_position + Vector3::new(0.0, 5.0, 0.0);
            let progress = self.travel_progress();

            for trail_index in 0..3 {
                let trail_progress = (progress - trail_index as f32 * 0.075).clamp(0.0, 1.0);
                let position = frost_diver_position(source, destination, trail_progress);
                let scale = 1.0 - trail_index as f32 * 0.22;
                let alpha = (0.92 - trail_index as f32 * 0.23) * (0.4 + progress * 0.6);

                render_centered(
                    renderer,
                    self.projectile_texture.clone(),
                    position,
                    camera,
                    window_size,
                    ScreenSize {
                        width: 13.0 * scale,
                        height: 26.0 * scale,
                    },
                    Color::rgba(0.58, 0.84, 1.0, alpha),
                );
            }
        } else if let Some(impact_texture) = &self.impact_texture {
            let impact_progress = smoothstep(self.impact_progress());
            let size = 18.0 + impact_progress * 46.0;
            let alpha = (1.0 - impact_progress).clamp(0.0, 1.0);

            render_centered(
                renderer,
                impact_texture.clone(),
                self.destination_position + Vector3::new(0.0, 4.0, 0.0),
                camera,
                window_size,
                ScreenSize { width: size, height: size },
                Color::rgba(0.55, 0.82, 1.0, alpha),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_timeline_math_is_bounded() {
        assert_eq!(normalized_progress(-1.0, 1.0), 0.0);
        assert_eq!(normalized_progress(0.25, 1.0), 0.25);
        assert_eq!(normalized_progress(2.0, 1.0), 1.0);
        assert_eq!(normalized_progress(1.0, 0.0), 0.0);
        assert_eq!(normalized_progress(f32::NAN, 1.0), 0.0);
    }

    #[test]
    fn timeline_ignores_negative_delta_and_expires_at_duration() {
        let mut timeline = Timeline::new(0.5);

        assert!(timeline.advance(-1.0));
        assert_eq!(timeline.elapsed, 0.0);
        assert!(timeline.advance(0.49));
        assert!(!timeline.advance(0.01));
        assert_eq!(timeline.progress(), 1.0);
    }

    #[test]
    fn frost_diver_path_starts_and_ends_at_the_authoritative_positions() {
        let source = Point3::new(1.0, 2.0, 3.0);
        let destination = Point3::new(11.0, 4.0, 23.0);

        assert_eq!(frost_diver_position(source, destination, 0.0), source);
        assert_eq!(frost_diver_position(source, destination, 1.0), destination);

        let midpoint = frost_diver_position(source, destination, 0.5);
        assert!(midpoint.y > source.y + (destination.y - source.y) * 0.5);
    }

    #[test]
    fn cold_bolt_pattern_is_deterministic_and_bounded() {
        assert_eq!(cold_bolt_offset(0), Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(cold_bolt_offset(1), Vector3::new(-5.0, 0.0, 2.0));
        assert_eq!(cold_bolt_offset(8), Vector3::new(0.0, 0.0, 0.0));
        assert!(cold_bolt_offset(31).x.abs() <= 14.5);
        assert!(cold_bolt_offset(31).z.abs() <= 14.5);
    }

    #[test]
    fn phase_progress_stays_zero_before_impact() {
        assert_eq!(phase_progress(0.2, 0.45, 0.3), 0.0);
        assert!((phase_progress(0.6, 0.45, 0.3) - 0.5).abs() < 1.0e-6);
        assert_eq!(phase_progress(1.0, 0.45, 0.3), 1.0);
    }

    #[test]
    fn frost_diver_hit_sound_starts_with_the_impact_phase() {
        assert_eq!(FROST_DIVER_TRAVEL_DURATION, crate::world::FROST_DIVER_FOLLOWUP_DELAY);
    }
}
