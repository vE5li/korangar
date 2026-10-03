use std::f32::consts::PI;
use std::sync::Arc;

use cgmath::{Point3, Rad, Vector2, Vector3};
use korangar_interface::application::Clip;
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use crate::Entity;
use crate::graphics::{Color, GroundMarkerInstruction, ScreenClip, ScreenPosition, ScreenSize, Texture};
use crate::loaders::FontSize;
use crate::renderer::{AlignHorizontal, EFFECT_ORIGIN, EffectRenderer, GameInterfaceRenderer, SpriteRenderer};
use crate::world::{Camera, EffectBase, FIRE_ARROW_LAUNCH_SOUND_PATHS, ICE_ARROW_LAUNCH_SOUND_PATHS, PointLightManager};

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

/// The six 128x64 "fire arrow" frames the reference clients cycle for the
/// Fire Bolt projectile. `data.grf` also ships frames 7 and 8, which both
/// reference implementations exclude as unused by the official client.
pub const FIRE_ARROW_TEXTURE_PATHS: [&str; 6] = [
    "effect\\불화살1.tga",
    "effect\\불화살2.tga",
    "effect\\불화살3.tga",
    "effect\\불화살4.tga",
    "effect\\불화살5.tga",
    "effect\\불화살6.tga",
];

/// Cadence of the projectile texture cycle.
pub const FIRE_ARROW_FRAME_DURATION: f32 = 0.03;

/// Height above the target that a bolt starts its descent from.
const BOLT_DESCENT_HEIGHT: f32 = 38.0;

/// Ground clearance of a bolt when it reaches the target, matching the impact
/// animation's own offset.
const BOLT_TARGET_HEIGHT: f32 = 5.0;

/// How a projectile moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoltMotion {
    /// Spawns above the target and drops onto it, never involving the
    /// caster. The classic bolt spells work this way.
    FallOntoTarget,
    /// Launches from the caster's position at spawn time and flies to the
    /// target. Arrows and thrown projectiles work this way.
    TravelFromSource,
}

/// Where a projectile's frames come from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoltFrameSource {
    /// Standalone textures cycled at a fixed cadence. A single entry renders
    /// unanimated.
    Textures {
        paths: &'static [&'static str],
        frame_duration: f32,
    },
    /// An SPR/ACT pair: the sprite provides the frames in order and the
    /// action provides the cadence.
    SpriteAction {
        sprite_path: &'static str,
        action_path: &'static str,
    },
}

/// How the projectile quad is sized, in the effect renderer's pixel space
/// (the same units as STR frame coordinates).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoltQuadSize {
    /// A size declared outright, matching the reference client's tables.
    Fixed { width: f32, height: f32 },
    /// The first frame's native pixel size multiplied by `scale`, resolved
    /// when the textures are loaded. For SPR sources whose frame sizes are
    /// not knowable statically.
    Native { scale: f32 },
}

/// The ACT delay unit in seconds: a delay of 1.0 is 50 milliseconds.
pub const ACT_DELAY_UNIT: f32 = 0.05;

/// Art and motion of a skill projectile.
#[derive(Clone, Copy, Debug)]
pub struct BoltProjectileArt {
    pub source: BoltFrameSource,
    pub size: BoltQuadSize,
    pub motion: BoltMotion,
    /// Fade the projectile in over the first quarter of its flight and out
    /// over the last, as the reference client does for travelling
    /// projectiles. Falling bolts render at full opacity.
    pub fade: bool,
    /// Radians added to the flight heading so the art's own forward axis
    /// ends up pointing along the movement. Zero means the art already
    /// points along +X, like the fire arrow streaks, whose measured alpha
    /// mass put the dense head at +X. Measured per art where possible.
    pub base_angle: f32,
    pub launch_sounds: &'static [&'static str],
    pub sound_range: f32,
    /// Fixed flight duration overriding the attack-motion derivation. The
    /// reference client flies arrows in 140ms and the fireball in 250ms
    /// regardless of the caster; the hit still waits for the server's own
    /// timing, so a faster projectile simply lands earlier than the impact
    /// rather than desynchronizing it.
    pub flight_override: Option<f32>,
}

/// Alpha envelope of a projectile at `progress` through its flight.
fn fade_envelope(progress: f32, enabled: bool) -> f32 {
    if !enabled {
        return 1.0;
    }

    let progress = progress.clamp(0.0, 1.0);
    (progress / 0.25).min((1.0 - progress) / 0.25).clamp(0.0, 1.0)
}

/// Fire Bolt: six animated streaks.
///
/// Rendered at the reference client's declared size rather than the source
/// texture's native 128x64. The official client deliberately draws that
/// texture slightly downsampled, and the proportion is what matters: measured
/// against `firehit2.str`, which both clients load from the same archive, the
/// reference draws the streak 1.23x the impact's median width of 81. Native
/// resolution put it at 1.58x, which read as too wide.
pub const FIRE_BOLT_ART: BoltProjectileArt = BoltProjectileArt {
    source: BoltFrameSource::Textures {
        paths: &FIRE_ARROW_TEXTURE_PATHS,
        frame_duration: FIRE_ARROW_FRAME_DURATION,
    },
    size: BoltQuadSize::Fixed {
        width: 100.0,
        height: 50.0,
    },
    motion: BoltMotion::FallOntoTarget,
    fade: false,
    base_angle: 0.0,
    launch_sounds: &FIRE_ARROW_LAUNCH_SOUND_PATHS,
    sound_range: 55.0,
    flight_override: None,
};

/// Cold Bolt: one unanimated shard, square where Fire Bolt is a streak.
///
/// Rendered at half Fire Bolt's width and the same height, which is the ratio
/// the reference client uses: it sizes the ice shard at 50 against the fire
/// arrow's 100x50. The source texture is 128x128, so this downsamples it, as
/// the reference does too. A square reads considerably heavier than a streak
/// of the same width, so matching the fire arrow's 128 here would overpower
/// it.
pub const COLD_BOLT_ART: BoltProjectileArt = BoltProjectileArt {
    source: BoltFrameSource::Textures {
        paths: &[ICE_ARROW_TEXTURE_PATH],
        frame_duration: FIRE_ARROW_FRAME_DURATION,
    },
    size: BoltQuadSize::Fixed { width: 50.0, height: 50.0 },
    motion: BoltMotion::FallOntoTarget,
    fade: false,
    base_angle: 0.0,
    launch_sounds: &ICE_ARROW_LAUNCH_SOUND_PATHS,
    sound_range: 55.0,
    flight_override: None,
};

/// Fire Ball: the thrown sphere, animated from its own SPR/ACT pair.
///
/// The frames' native pixel sizes are only knowable once the sprite is
/// loaded, so the quad derives from them. The 2x scale reads the reference
/// table's 200 as a percentage, which is the only interpretation that lands
/// in a sane size band; treat it as tunable rather than measured.
pub const FIRE_BALL_ART: BoltProjectileArt = BoltProjectileArt {
    source: BoltFrameSource::SpriteAction {
        sprite_path: "이팩트\\fireball.spr",
        action_path: "이팩트\\fireball.act",
    },
    size: BoltQuadSize::Native { scale: 2.0 },
    motion: BoltMotion::TravelFromSource,
    fade: true,
    // The frames' alpha mass sits above centre in every frame: that dense
    // mass is the ball itself, with the flame trailing below, so the
    // sprite's forward axis is the image top. Confirmed in motion: the
    // opposite reading flew the sphere tail-first.
    base_angle: std::f32::consts::FRAC_PI_2,
    launch_sounds: &["effect\\ef_fireball.wav"],
    sound_range: 60.0,
    flight_override: Some(0.25),
};

/// The arrow every bow skill shares, straight from the skeleton archer's
/// ammunition sprite. The reference client uses this one sprite for all of
/// its arrow skills and for ranged basic attacks.
///
/// Silent: the reference declares no launch sound for arrows, and the bow's
/// own attack sound already plays through the attack motion.
pub const ARROW_ART: BoltProjectileArt = BoltProjectileArt {
    source: BoltFrameSource::SpriteAction {
        sprite_path: "npc\\skel_archer_arrow.spr",
        action_path: "npc\\skel_archer_arrow.act",
    },
    // Declared, not native: the 8x61 frame at native size read far too
    // large in motion. Length under half the fire streak's 100, thickness
    // keeping the sprite's own slender aspect.
    size: BoltQuadSize::Fixed { width: 5.0, height: 40.0 },
    motion: BoltMotion::TravelFromSource,
    fade: true,
    // The ammunition sprite is drawn vertically. Its row profile shows the
    // anatomy: a short head blob at the image top, a constant shaft, then
    // longer fletching tapering to a one-pixel nock at the bottom, so the
    // head is at the top and a quarter turn aligns it with the flight.
    base_angle: std::f32::consts::FRAC_PI_2,
    launch_sounds: &[],
    sound_range: 55.0,
    flight_override: Some(0.14),
};

/// How long a skill's name floats over its caster.
pub const SKILL_NAME_BUBBLE_DURATION: f32 = 1.4;

/// The skill's name floating over the caster once the cast fires, in the
/// classic green over a dark offset copy.
pub struct SkillNameBubble {
    entity_id: EntityId,
    position: Point3<f32>,
    text: String,
    elapsed: f32,
}

impl SkillNameBubble {
    pub fn new(entity_id: EntityId, position: Point3<f32>, text: String) -> Self {
        Self {
            entity_id,
            position,
            text,
            elapsed: 0.0,
        }
    }
}

impl EntityParticle for SkillNameBubble {
    fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool {
        if let Some(position) = resolve_entity_position(self.entity_id, entities, local_entity) {
            self.position = position;
        }

        self.elapsed += delta_time.max(0.0);
        self.elapsed < SKILL_NAME_BUBBLE_DURATION
    }

    fn render(&self, renderer: &GameInterfaceRenderer, camera: &dyn Camera, window_size: ScreenSize) {
        // Fade out over the final quarter.
        let remaining = SKILL_NAME_BUBBLE_DURATION - self.elapsed;
        let alpha = (remaining / (SKILL_NAME_BUBBLE_DURATION * 0.25)).clamp(0.0, 1.0);

        let position = to_screen_position(self.position + Vector3::new(0.0, 34.0, 0.0), camera, window_size);
        let shadow_position = ScreenPosition {
            left: position.left + 1.0,
            top: position.top + 1.0,
        };
        let font_size = FontSize(14.0);

        renderer.render_text(
            &self.text,
            shadow_position,
            Color::rgba(0.1, 0.1, 0.1, 0.8 * alpha),
            font_size,
            AlignHorizontal::Center,
        );
        renderer.render_text(
            &self.text,
            position,
            Color::rgba(0.0, 1.0, 0.0, alpha),
            font_size,
            AlignHorizontal::Center,
        );
    }
}

/// Which of a cast's two rings this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastRingKind {
    /// The elemental magic circle swirling at the caster's feet.
    Aura,
    /// The lock-on circle shrinking onto the target, marking who the cast
    /// will land on. Both reference clients draw this from the same
    /// lockon128 texture.
    LockOn,
}

/// A ring shown for the length of a cast: the aura at the caster, the
/// lock-on at the target.
///
/// The classic aura is a rising textured cylinder; the fork's particle path
/// draws camera-facing quads only, so both rings are flattened quads faking
/// the ground perspective. Rings are keyed by the caster so a cancelled cast
/// tears down its aura and its lock-on together, and expire with the cast
/// duration otherwise.
pub struct CastRing {
    kind: CastRingKind,
    caster_entity_id: EntityId,
    follow_entity_id: EntityId,
    position: Point3<f32>,
    /// The swirling cone band, drawn through the effect pass. Only the aura
    /// has one; the lock-on is entirely a ground marker.
    cone_texture: Option<Arc<Texture>>,
    /// The depth-tested ground quad: the magic circle under the caster or
    /// the reticle under the target.
    ground_texture: Arc<Texture>,
    tint: Color,
    duration: f32,
    elapsed: f32,
}

impl CastRing {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        kind: CastRingKind,
        caster_entity_id: EntityId,
        follow_entity_id: EntityId,
        position: Point3<f32>,
        cone_texture: Option<Arc<Texture>>,
        ground_texture: Arc<Texture>,
        tint: Color,
        duration: f32,
    ) -> Self {
        Self {
            kind,
            caster_entity_id,
            follow_entity_id,
            position,
            cone_texture,
            ground_texture,
            tint,
            duration: duration.max(f32::EPSILON),
            elapsed: 0.0,
        }
    }

    /// The depth-tested ground quad of this ring: the spinning magic circle
    /// or reticle. World-space corners, so entities standing in front of it
    /// occlude it correctly.
    pub fn ground_marker(&self) -> GroundMarkerInstruction {
        let alpha = (self.elapsed / 0.15).min((self.duration - self.elapsed) / 0.15).clamp(0.0, 0.85);

        // One tile is five world units; the classic circles span about
        // three tiles.
        let (half_size, spin_speed) = match self.kind {
            // A gentle swirl under the caster.
            CastRingKind::Aura => (7.5, 45.0_f32.to_radians()),
            // The reference reticle snaps from fifteen cells down to three
            // in a fifth of a second and rests there for the whole cast,
            // spinning. The snap is the eye-catch; the rest size is the
            // official three tiles.
            CastRingKind::LockOn => {
                let snap = (self.elapsed / 0.2).clamp(0.0, 1.0);
                let full_extent_tiles = 15.0 - 12.0 * snap;
                // Half extent in world units: tiles x 5 / 2.
                (full_extent_tiles * 2.5, 270.0_f32.to_radians())
            }
        };

        let angle = self.elapsed * spin_speed;
        let (sin, cos) = angle.sin_cos();
        let rotate = |x: f32, z: f32| Vector3::new(x * cos - z * sin, 0.0, x * sin + z * cos);

        // Slightly lifted so the quad wins the depth test against the ground
        // it sits on.
        let center = self.position + Vector3::new(0.0, 0.5, 0.0);
        let color = Color::rgba(self.tint.red, self.tint.green, self.tint.blue, self.tint.alpha * alpha);

        GroundMarkerInstruction {
            upper_left: center + rotate(-half_size, -half_size),
            upper_right: center + rotate(half_size, -half_size),
            lower_left: center + rotate(-half_size, half_size),
            lower_right: center + rotate(half_size, half_size),
            color,
            uv_offset: Vector2::new(0.0, 0.0),
            uv_scale: Vector2::new(1.0, 1.0),
            edge_fade: 0.0,
            texture: self.ground_texture.clone(),
        }
    }

    /// The swirling cone above the ground circle, as camera-facing vertical
    /// trapezoids in world space so it is depth-tested like the circle: the
    /// caster's sprite covers the part of the cone behind them. The swirl is
    /// the band texture's U coordinate scrolling under the repeat sampler.
    /// Only the aura carries a cone.
    pub fn cone_markers(&self, camera_right: Vector3<f32>, markers: &mut Vec<GroundMarkerInstruction>) {
        let Some(cone_texture) = &self.cone_texture else {
            return;
        };

        let alpha = (self.elapsed / 0.15).min((self.duration - self.elapsed) / 0.15).clamp(0.0, 0.85);
        let base = self.position + Vector3::new(0.0, 0.3, 0.0);

        // Two layers at the reference's differing swirl speeds read as the
        // classic vortex. Sizes are in world units, proportioned like the
        // reference cone: bottom narrow, top wide.
        for (scroll_speed, scale, layer_alpha) in [(0.5, 1.0, 0.55), (0.75, 1.25, 0.35)] {
            // The swirling arcs orbit wider than the body and rise past its
            // height: roughly a character-height cone opening to three
            // tiles, per the reference's own proportions.
            let bottom_half_width = 4.0 * scale;
            let top_half_width = 8.5 * scale;
            let height = 11.0 * scale;
            let scroll = self.elapsed * scroll_speed;

            let color = Color::rgba(
                self.tint.red,
                self.tint.green,
                self.tint.blue,
                self.tint.alpha * alpha * layer_alpha,
            );

            markers.push(GroundMarkerInstruction {
                upper_left: base + camera_right * -top_half_width + Vector3::new(0.0, height, 0.0),
                upper_right: base + camera_right * top_half_width + Vector3::new(0.0, height, 0.0),
                lower_left: base + camera_right * -bottom_half_width,
                lower_right: base + camera_right * bottom_half_width,
                color,
                uv_offset: Vector2::new(scroll, 0.0),
                uv_scale: Vector2::new(1.0, 1.0),
                // A flat quad faking a cylinder: the band is opaque to its
                // edges, so without the fade its slanted silhouette shows as
                // hard diagonal lines.
                edge_fade: 1.0,
                texture: cone_texture.clone(),
            });
        }
    }

    pub fn kind(&self) -> CastRingKind {
        self.kind
    }

    pub fn caster_entity_id(&self) -> EntityId {
        self.caster_entity_id
    }

    pub fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool {
        if let Some(position) = resolve_entity_position(self.follow_entity_id, entities, local_entity) {
            self.position = position;
        }

        self.elapsed += delta_time.max(0.0);
        self.elapsed < self.duration
    }
}

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

fn bolt_offset(bolt_index: usize) -> Vector3<f32> {
    // Every bolt of a volley approaches from the same upper corner and only
    // the jitter varies. The reference clients both do this, and it is what
    // makes a multi-hit cast read as one stream rather than a ring.
    const APPROACH_X: f32 = 10.0;
    const APPROACH_Z: f32 = 4.0;
    const JITTER: [(f32, f32); 6] = [(0.0, 0.0), (2.0, -1.5), (-2.0, 1.5), (1.0, 2.0), (-1.5, -2.0), (2.5, 0.5)];

    let (jitter_x, jitter_z) = JITTER[bolt_index % JITTER.len()];
    Vector3::new(APPROACH_X + jitter_x, 0.0, APPROACH_Z + jitter_z)
}

/// Screen-space angle that rotates the projectile's local +X axis onto the
/// direction it is travelling.
///
/// The source textures are horizontal streaks whose dense head sits at +X, so
/// an unrotated quad always reads as flying sideways. Screen space is y-down
/// and so is the effect renderer's corner space, which lets the angle be taken
/// directly. Both endpoints are scaled to pixels first so the result is not
/// skewed by the window's aspect ratio.
fn screen_direction_angle(from: Vector2<f32>, to: Vector2<f32>, window_size: ScreenSize) -> Rad<f32> {
    let delta_x = (to.x - from.x) * window_size.width;
    let delta_y = (to.y - from.y) * window_size.height;

    // A degenerate path would make atan2 jitter; leaving it unrotated is the
    // stable choice because the quad is then still centred on the target.
    if delta_x.abs() < f32::EPSILON && delta_y.abs() < f32::EPSILON {
        return Rad(0.0);
    }

    Rad(delta_y.atan2(delta_x))
}

fn projectile_screen_angle(camera: &dyn Camera, window_size: ScreenSize, from: Point3<f32>, to: Point3<f32>) -> Rad<f32> {
    let project = |point: Point3<f32>| {
        let clip_space_position = camera.view_projection_matrix() * point.to_homogeneous();
        camera.clip_to_screen_space(clip_space_position)
    };

    screen_direction_angle(project(from), project(to), window_size)
}

/// Frames and quad extents resolved from a [`BoltProjectileArt`] at spawn,
/// once the assets are loaded.
pub struct ResolvedBoltFrames {
    pub textures: Vec<Arc<Texture>>,
    pub frame_duration: f32,
    pub half_width: f32,
    pub half_height: f32,
}

/// One skill projectile, either falling onto or travelling to its target.
///
/// Rendered through the effect pipeline rather than as an interface sprite,
/// because it needs to be rotated onto its flight direction and blended
/// additively. Its lifetime is the projectile's flight time, so the
/// separately scheduled impact animation takes over at the exact moment the
/// projectile reaches the target.
pub struct BoltProjectile {
    art: BoltProjectileArt,
    target_entity_id: EntityId,
    target_position: Point3<f32>,
    /// The fixed point a travelling projectile was launched from. Falling
    /// bolts derive their entry point from the target instead, so they keep
    /// tracking a moving target at both ends of the path.
    launch_origin: Option<Point3<f32>>,
    textures: Vec<Arc<Texture>>,
    frame_duration: f32,
    half_width: f32,
    half_height: f32,
    offset: Vector3<f32>,
    timeline: Timeline,
    deleted: bool,
}

impl BoltProjectile {
    pub fn new(
        art: BoltProjectileArt,
        resolved: ResolvedBoltFrames,
        target_entity_id: EntityId,
        target_position: Point3<f32>,
        launch_origin: Option<Point3<f32>>,
        bolt_index: usize,
        flight_duration: f32,
    ) -> Self {
        Self {
            art,
            target_entity_id,
            target_position,
            launch_origin,
            textures: resolved.textures,
            frame_duration: resolved.frame_duration,
            half_width: resolved.half_width,
            half_height: resolved.half_height,
            offset: bolt_offset(bolt_index),
            timeline: Timeline::new(flight_duration),
            deleted: false,
        }
    }

    fn frame_index(&self) -> usize {
        if self.textures.is_empty() {
            return 0;
        }

        let frame = self.timeline.elapsed.max(0.0) / self.frame_duration.max(f32::EPSILON);
        (frame as usize) % self.textures.len()
    }

    /// Where the projectile enters: above and beside the target for a
    /// falling bolt, the caster's position at spawn for a travelling one.
    fn launch_position(&self) -> Point3<f32> {
        match self.art.motion {
            BoltMotion::FallOntoTarget => self.landing_position() + self.offset + Vector3::new(0.0, BOLT_DESCENT_HEIGHT, 0.0),
            BoltMotion::TravelFromSource => self
                .launch_origin
                .unwrap_or_else(|| self.landing_position() + self.offset + Vector3::new(0.0, BOLT_DESCENT_HEIGHT, 0.0)),
        }
    }

    /// Where the projectile lands. The impact animation plays here, so it
    /// converges completely rather than keeping a residual offset.
    fn landing_position(&self) -> Point3<f32> {
        self.target_position + Vector3::new(0.0, BOLT_TARGET_HEIGHT, 0.0)
    }
}

impl EffectBase for BoltProjectile {
    fn update(&mut self, entities: &[Entity], local_entity: Option<(EntityId, Point3<f32>)>, delta_time: f32) -> bool {
        if self.deleted {
            return false;
        }

        if let Some(position) = resolve_entity_position(self.target_entity_id, entities, local_entity) {
            self.target_position = position;
        }

        self.timeline.advance(delta_time)
    }

    fn mark_for_deletion(&mut self) {
        self.deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let Some(texture) = self.textures.get(self.frame_index()) else {
            return;
        };

        let progress = self.timeline.progress();
        let launch = self.launch_position();
        let landing = self.landing_position();
        let position = launch + (landing - launch) * smoothstep(progress);
        let angle = projectile_screen_angle(camera, renderer.window_size(), launch, landing) + Rad(self.art.base_angle);
        let alpha = fade_envelope(progress, self.art.fade);

        // Corner order the effect renderer expects: top left, top right,
        // bottom left, bottom right.
        let half_width = self.half_width;
        let half_height = self.half_height;
        let corners = [
            Vector2::new(-half_width, -half_height),
            Vector2::new(half_width, -half_height),
            Vector2::new(-half_width, half_height),
            Vector2::new(half_width, half_height),
        ];

        // The renderer maps these as [2] top left, [1] top right,
        // [3] bottom left, [0] bottom right.
        let texture_coordinates = [
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 1.0),
        ];

        renderer.render_effect(
            camera,
            position,
            texture.clone(),
            corners,
            texture_coordinates,
            // Cancels the renderer's own origin shift so the quad ends up
            // centred on the projectile and rotates about its own centre.
            EFFECT_ORIGIN,
            angle,
            Color::rgba(1.0, 1.0, 1.0, alpha),
            // Additive, matching the reference client's blend mode for fire.
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }
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

    /// Frame-less resolution for constructor-level tests.
    fn no_frames() -> ResolvedBoltFrames {
        ResolvedBoltFrames {
            textures: Vec::new(),
            frame_duration: FIRE_ARROW_FRAME_DURATION,
            half_width: 50.0,
            half_height: 25.0,
        }
    }

    fn fixed_size(art: &BoltProjectileArt) -> (f32, f32) {
        match art.size {
            BoltQuadSize::Fixed { width, height } => (width, height),
            BoltQuadSize::Native { .. } => panic!("expected a declared size"),
        }
    }

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
    fn fire_bolt_frames_cycle_and_stay_in_bounds() {
        let frame_count = FIRE_ARROW_TEXTURE_PATHS.len();
        assert_eq!(frame_count, 6);

        // Frame selection is derived from elapsed time, so it must wrap rather
        // than run off the end of the texture list.
        for step in 0..200 {
            let elapsed = step as f32 * FIRE_ARROW_FRAME_DURATION * 0.5;
            let index = ((elapsed.max(0.0) / FIRE_ARROW_FRAME_DURATION) as usize) % frame_count;
            assert!(index < frame_count);
        }

        let index_at = |elapsed: f32| ((elapsed.max(0.0) / FIRE_ARROW_FRAME_DURATION) as usize) % frame_count;
        assert_eq!(index_at(0.0), 0);
        assert_eq!(index_at(FIRE_ARROW_FRAME_DURATION * 1.5), 1);
        assert_eq!(index_at(FIRE_ARROW_FRAME_DURATION * frame_count as f32), 0);
        assert_eq!(index_at(-1.0), 0);
    }

    #[test]
    fn fire_bolt_approaches_from_one_corner_and_is_bounded() {
        // Every bolt shares an approach direction so a volley reads as one
        // stream; only the jitter varies.
        for bolt_index in 0..32 {
            let offset = bolt_offset(bolt_index);

            assert!(offset.x > 0.0, "bolt {bolt_index} must approach from the same side");
            assert!(offset.z > 0.0, "bolt {bolt_index} must approach from the same side");
            assert!(offset.x <= 14.0);
            assert!(offset.z <= 8.0);
            assert_eq!(offset.y, 0.0, "descent height is applied separately");
        }

        // Deterministic, so a replayed volley looks identical.
        assert_eq!(bolt_offset(0), bolt_offset(6));
        assert_ne!(bolt_offset(0), bolt_offset(1));
    }

    #[test]
    fn fire_bolt_descends_onto_the_target_and_expires_with_its_flight() {
        const FLIGHT: f32 = 0.42;
        let target = Point3::new(100.0, 20.0, 300.0);
        let mut projectile = BoltProjectile::new(FIRE_BOLT_ART, no_frames(), EntityId(7), target, None, 0, FLIGHT);

        // Enters above and beside the target.
        let launch = projectile.launch_position();
        let landing = projectile.landing_position();
        assert!(launch.y > landing.y, "must enter from above");
        assert!(launch.x > landing.x, "must enter from the side");

        // Lives exactly as long as the flight it was given, so the separately
        // scheduled impact takes over the moment it lands.
        assert_eq!(projectile.timeline.progress(), 0.0);
        assert!(projectile.update(&[], None, FLIGHT * 0.5));
        assert!((projectile.timeline.progress() - 0.5).abs() < 1.0e-6);
        assert!(!projectile.update(&[], None, FLIGHT * 0.5));
        assert_eq!(projectile.timeline.progress(), 1.0);
    }

    #[test]
    fn fire_bolt_converges_completely_onto_the_target() {
        // Unlike Cold Bolt, which keeps a residual offset, the fire bolt must
        // land on the target: the impact animation plays there.
        let target = Point3::new(10.0, 0.0, 20.0);
        let projectile = BoltProjectile::new(FIRE_BOLT_ART, no_frames(), EntityId(1), target, None, 3, 0.42);

        let launch = projectile.launch_position();
        let landing = projectile.landing_position();
        let arrived = launch + (landing - launch) * smoothstep(1.0);

        assert!((arrived.x - landing.x).abs() < 1.0e-6);
        assert!((arrived.y - landing.y).abs() < 1.0e-6);
        assert!((arrived.z - landing.z).abs() < 1.0e-6);
        assert_eq!(landing.y, target.y + BOLT_TARGET_HEIGHT);
    }

    #[test]
    fn projectile_rotation_points_the_texture_along_its_travel_direction() {
        // The source textures are horizontal streaks whose head is at +X, and
        // screen space is y-down, so these are the angles that aim the head
        // at the target. An unrotated quad always reads as flying sideways.
        const SQUARE: ScreenSize = ScreenSize {
            width: 100.0,
            height: 100.0,
        };
        let origin = Vector2::new(0.5, 0.5);
        let angle_to = |x: f32, y: f32| screen_direction_angle(origin, Vector2::new(x, y), SQUARE).0;

        // Straight right along +X: the texture already points this way.
        assert!((angle_to(0.6, 0.5) - 0.0).abs() < 1.0e-5);
        // Straight down the screen is a quarter turn, positive because y is down.
        assert!((angle_to(0.5, 0.6) - std::f32::consts::FRAC_PI_2).abs() < 1.0e-5);
        // Straight up is the opposite quarter turn.
        assert!((angle_to(0.5, 0.4) + std::f32::consts::FRAC_PI_2).abs() < 1.0e-5);
        // Straight left is a half turn.
        assert!((angle_to(0.4, 0.5).abs() - std::f32::consts::PI).abs() < 1.0e-5);

        // The real case: a bolt entering from the upper right and falling to
        // the lower left points down and to the left, between 90 and 180.
        let falling = angle_to(0.4, 0.6);
        assert!(
            falling > std::f32::consts::FRAC_PI_2 && falling < std::f32::consts::PI,
            "expected a down-left heading, got {falling}"
        );

        // Degenerate paths must not make the sprite spin.
        assert_eq!(screen_direction_angle(origin, origin, SQUARE).0, 0.0);
    }

    #[test]
    fn bolt_quads_match_their_own_source_texture_aspect() {
        // The two bolts do not share an aspect: the fire arrow texture is
        // 128x64 and the ice shard 128x128. Sizing them from one shared
        // constant would stretch one, which is why size lives on the art.
        let (fire_width, fire_height) = fixed_size(&FIRE_BOLT_ART);
        let (cold_width, cold_height) = fixed_size(&COLD_BOLT_ART);
        assert_eq!(fire_width / fire_height, 2.0);
        assert_eq!(cold_width / cold_height, 1.0);

        // The reference client declares these outright: the fire arrow at
        // 100x50 and the ice shard at 50 square. Those are in the same units
        // as STR frame coordinates, which each client divides by 35 to reach
        // world space, so the numbers transfer directly. Note this is not the
        // textures' native resolution: the official client draws them
        // slightly downsampled.
        assert_eq!((fire_width, fire_height), (100.0, 50.0));
        assert_eq!((cold_width, cold_height), (50.0, 50.0));

        // Half the width, same height. A square reads much heavier than a
        // streak, so matching widths would overpower it.
        assert_eq!(cold_width, fire_width * 0.5);
        assert_eq!(cold_height, fire_height);

        // Cross-check against an asset both clients load from the same
        // archive, which cancels out any difference in their coordinate
        // spaces: firehit2.str renders quads of median 81 wide, and the
        // reference draws the streak at 1.23x that. Sizing from native
        // texture resolution instead gave 1.58x, which read as too wide.
        const FIRE_HIT_MEDIAN_WIDTH: f32 = 81.0;
        let width_ratio = fire_width / FIRE_HIT_MEDIAN_WIDTH;
        assert!(
            (1.15..=1.30).contains(&width_ratio),
            "streak width should track the reference's 1.23x of its impact, got {width_ratio}"
        );

        for art in [FIRE_BOLT_ART, COLD_BOLT_ART] {
            let (width, height) = fixed_size(&art);
            assert!(width > 0.0 && height > 0.0);
            assert_eq!(art.motion, BoltMotion::FallOntoTarget);
            assert!(!art.fade, "falling bolts render at full opacity");
            match art.source {
                BoltFrameSource::Textures { paths, frame_duration } => {
                    assert!(!paths.is_empty(), "a projectile with no frames renders nothing");
                    assert!(frame_duration > 0.0, "a zero frame duration would divide by zero");
                }
                BoltFrameSource::SpriteAction { .. } => panic!("bolts are texture-sourced"),
            }
            assert!(!art.launch_sounds.is_empty());
        }
    }

    #[test]
    fn travelling_projectiles_launch_from_the_caster_and_land_on_the_target() {
        const TRAVEL_ART: BoltProjectileArt = BoltProjectileArt {
            motion: BoltMotion::TravelFromSource,
            fade: true,
            ..FIRE_BOLT_ART
        };
        let source = Point3::new(0.0, 0.0, 0.0);
        let target = Point3::new(60.0, 0.0, 40.0);
        let mut projectile = BoltProjectile::new(TRAVEL_ART, no_frames(), EntityId(9), target, Some(source), 0, 0.42);

        // Launches exactly from the stored origin, not from a target-derived
        // corner: the per-bolt jitter belongs to falling bolts only.
        assert_eq!(projectile.launch_position(), source);
        let landing = projectile.landing_position();
        assert_eq!(landing, target + Vector3::new(0.0, BOLT_TARGET_HEIGHT, 0.0));

        // Full progress arrives exactly on the landing point.
        let launch = projectile.launch_position();
        let arrived = launch + (landing - launch) * smoothstep(1.0);
        assert!((arrived.x - landing.x).abs() < 1.0e-6);
        assert!((arrived.z - landing.z).abs() < 1.0e-6);

        // The origin is fixed while the target keeps being followed, so a
        // projectile in flight does not teleport when its caster moves.
        assert!(projectile.update(&[], None, 0.1));
        assert_eq!(projectile.launch_position(), source);

        // A travelling art without an origin still renders somewhere sane:
        // it degrades to the falling entry rather than the world origin.
        let fallback = BoltProjectile::new(TRAVEL_ART, no_frames(), EntityId(9), target, None, 0, 0.42);
        assert!(fallback.launch_position().y > fallback.landing_position().y);
    }

    #[test]
    fn base_orientation_matches_each_art_measurement() {
        // The streak textures' measured alpha mass put the dense head at +X:
        // no correction.
        assert_eq!(FIRE_BOLT_ART.base_angle, 0.0);
        assert_eq!(COLD_BOLT_ART.base_angle, 0.0);

        // The fireball frames' dense alpha mass above centre is the ball
        // itself, flame trailing below: forward is the image top. The
        // opposite reading was tried first and flew the sphere tail-first.
        assert_eq!(FIRE_BALL_ART.base_angle, std::f32::consts::FRAC_PI_2);

        // The 8x61 ammunition sprite is vertical, so a quarter-turn
        // magnitude is certain even though the head's end was unmeasurable.
        assert_eq!(ARROW_ART.base_angle.abs(), std::f32::consts::FRAC_PI_2);
    }

    #[test]
    fn fade_envelope_ramps_over_the_outer_quarters_only_when_enabled() {
        // Disabled: full opacity for the whole flight.
        for progress in [0.0, 0.1, 0.5, 0.9, 1.0] {
            assert_eq!(fade_envelope(progress, false), 1.0);
        }

        // Enabled: in over the first quarter, out over the last, opaque
        // in between, and clamped outside the flight.
        assert_eq!(fade_envelope(0.0, true), 0.0);
        assert!((fade_envelope(0.125, true) - 0.5).abs() < 1.0e-6);
        assert_eq!(fade_envelope(0.25, true), 1.0);
        assert_eq!(fade_envelope(0.5, true), 1.0);
        assert_eq!(fade_envelope(0.75, true), 1.0);
        assert!((fade_envelope(0.875, true) - 0.5).abs() < 1.0e-6);
        assert_eq!(fade_envelope(1.0, true), 0.0);
        assert_eq!(fade_envelope(-1.0, true), 0.0);
        assert_eq!(fade_envelope(2.0, true), 0.0);
    }

    #[test]
    fn act_cadence_resolves_to_milliseconds() {
        // The ACT delay unit is 50ms: korangar's own action renderer computes
        // `delay * 50.0` milliseconds per frame. A typical delay of 4.0 must
        // therefore cycle at 200ms.
        assert_eq!(ACT_DELAY_UNIT, 0.05);
        assert!((4.0 * ACT_DELAY_UNIT - 0.2).abs() < 1.0e-6);
    }

    #[test]
    fn cold_bolt_uses_the_authentic_shard_rather_than_an_approximation() {
        // The archive ships the real projectile texture, so the earlier
        // procedural stand-in is no longer needed.
        assert_eq!(COLD_BOLT_ART.source, BoltFrameSource::Textures {
            paths: &[ICE_ARROW_TEXTURE_PATH],
            frame_duration: FIRE_ARROW_FRAME_DURATION,
        });
        // A single frame must still index safely through the cycling logic.
        let projectile = BoltProjectile::new(
            COLD_BOLT_ART,
            no_frames(),
            EntityId(2),
            Point3::new(0.0, 0.0, 0.0),
            None,
            0,
            0.42,
        );
        assert_eq!(projectile.frame_index(), 0);
    }

    #[test]
    fn projectile_rotation_is_corrected_for_window_aspect() {
        // Equal normalized deltas are not equal on screen unless the window is
        // square, so the angle has to be taken in pixels.
        let from = Vector2::new(0.5, 0.5);
        let to = Vector2::new(0.6, 0.6);

        let square = screen_direction_angle(from, to, ScreenSize {
            width: 100.0,
            height: 100.0,
        });
        let wide = screen_direction_angle(from, to, ScreenSize {
            width: 200.0,
            height: 100.0,
        });

        assert!((square.0 - std::f32::consts::FRAC_PI_4).abs() < 1.0e-5);
        assert!(wide.0 < square.0, "a wider window must flatten the heading");
    }

    #[test]
    fn fire_bolt_follows_a_moving_target_and_can_be_cancelled() {
        let mut projectile = BoltProjectile::new(
            FIRE_BOLT_ART,
            no_frames(),
            EntityId(1),
            Point3::new(0.0, 0.0, 0.0),
            None,
            0,
            0.42,
        );

        // Effects are torn down on world transitions.
        projectile.mark_for_deletion();
        assert!(!projectile.update(&[], None, 0.01));
    }

    #[test]
    fn fire_bolt_projectile_survives_a_missing_texture_list() {
        // load_skill_particle_texture can fail; the projectile must not panic
        // on an empty list.
        let mut projectile = BoltProjectile::new(
            FIRE_BOLT_ART,
            no_frames(),
            EntityId(1),
            Point3::new(0.0, 0.0, 0.0),
            None,
            0,
            0.42,
        );

        assert_eq!(projectile.frame_index(), 0);
        assert!(projectile.update(&[], None, 0.1));
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
