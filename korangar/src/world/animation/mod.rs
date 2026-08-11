use std::num::NonZeroU32;
use std::sync::Arc;

use cgmath::{Array, Matrix4, Point3, Transform, Vector2, Vector3, Zero};
use korangar_container::Cacheable;
use korangar_interface::element::StateElement;
use ragnarok_packets::{ClientTick, Direction, EntityId};
use rust_state::RustState;

#[cfg(feature = "debug")]
use crate::graphics::DebugRectangleInstruction;
use crate::graphics::{Color, EntityInstruction};
use crate::loaders::Sprite;
use crate::world::{ActionEvent, Actions, Camera, EntityType};

const TILE_SIZE: f32 = 10.0;
const SPRITE_SCALE: f32 = 1.4;
const PICKUP_ANIMATION_FACTOR: f32 = 25.0;

#[allow(dead_code)]
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum AnimationActionType {
    Attack1,
    Attack2,
    Attack3,
    Die,
    Freeze1,
    Freeze2,
    Hurt,
    #[default]
    Idle,
    Pickup,
    ReadyFight,
    Sit,
    Skill,
    Special,
    Walk,
}

impl AnimationActionType {
    pub fn action_base_offset(&self, entity_type: EntityType) -> usize {
        match entity_type {
            EntityType::Hidden | EntityType::Player => match self {
                AnimationActionType::Idle => 0,
                AnimationActionType::Walk => 1,
                AnimationActionType::Sit => 2,
                AnimationActionType::Pickup => 3,
                AnimationActionType::ReadyFight => 4,
                AnimationActionType::Attack1 => 5,
                AnimationActionType::Hurt => 6,
                AnimationActionType::Freeze1 => 7,
                AnimationActionType::Die => 8,
                AnimationActionType::Freeze2 => 9,
                AnimationActionType::Attack2 => 10,
                AnimationActionType::Attack3 => 11,
                AnimationActionType::Skill => 12,
                _ => 0,
            },
            EntityType::Npc | EntityType::Monster => match self {
                AnimationActionType::Idle => 0,
                AnimationActionType::Walk => 1,
                AnimationActionType::Attack1 => 2,
                AnimationActionType::Hurt => 3,
                AnimationActionType::Die => 4,
                _ => 0,
            },
            EntityType::Warp => 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AnimationState {
    action_type: AnimationActionType,
    action_base_offset: usize,
    start_time: ClientTick,
    time: u32,
    duration: Option<u32>,
    factor: Option<f32>,
    looping: bool,
    casting: Option<CastingState>,
}

#[derive(Clone, Debug)]
struct CastingState {
    start_time: ClientTick,
    elapsed: u32,
    duration: NonZeroU32,
}

impl AnimationState {
    pub fn new(entity_type: EntityType, start_time: ClientTick) -> Self {
        let action_type = AnimationActionType::Idle;
        Self {
            action_type,
            action_base_offset: action_type.action_base_offset(entity_type),
            start_time,
            time: 0,
            duration: None,
            factor: None,
            looping: true,
            casting: None,
        }
    }

    pub fn idle(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Idle;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = true;
    }

    pub fn attack(&mut self, entity_type: EntityType, attack_duration: u32, critical: bool, client_tick: ClientTick) {
        self.action_type = match critical {
            true => AnimationActionType::Attack3,
            false => AnimationActionType::Attack1,
        };
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = Some(attack_duration);
        self.factor = None;
        self.looping = false;
    }

    pub fn pickup(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Pickup;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(PICKUP_ANIMATION_FACTOR);
        self.looping = false;
    }

    /// Plays the casting motion for the duration reported by the server.
    ///
    /// Cast timing is tracked separately from the visible sprite action so
    /// that reactions such as hurt animations do not hide the cast bar.
    pub fn cast(&mut self, entity_type: EntityType, cast_time: NonZeroU32, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Skill;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.time = 0;
        self.duration = Some(cast_time.get());
        self.factor = None;
        self.looping = false;
        self.casting = Some(CastingState {
            start_time: client_tick,
            elapsed: 0,
            duration: cast_time,
        });
    }

    pub fn walk(&mut self, entity_type: EntityType, movement_speed: usize, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Walk;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(movement_speed as f32 * 100.0 / 150.0 / 5.0);
        self.looping = true;
    }

    pub fn dead(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        self.action_type = AnimationActionType::Die;
        self.action_base_offset = self.action_type.action_base_offset(entity_type);
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
        self.looping = false;
        self.casting = None;
    }

    pub fn is_attack(&self) -> bool {
        matches!(
            self.action_type,
            AnimationActionType::Attack1 | AnimationActionType::Attack2 | AnimationActionType::Attack3
        )
    }

    pub fn is_pickup(&self) -> bool {
        self.action_type == AnimationActionType::Pickup
    }

    pub fn is_casting(&self) -> bool {
        self.casting.is_some()
    }

    pub fn casting_progress(&self) -> Option<f32> {
        self.casting
            .as_ref()
            .map(|casting| (casting.elapsed as f32 / casting.duration.get() as f32).clamp(0.0, 1.0))
    }

    pub fn is_casting_complete(&self) -> bool {
        self.is_casting()
            && self
                .casting
                .as_ref()
                .is_some_and(|casting| casting.elapsed >= casting.duration.get())
    }

    /// Clears cast timing without changing the current sprite action.
    pub fn clear_cast(&mut self) {
        self.casting = None;
    }

    /// Clears server-side cast timing and returns the sprite to idle only when
    /// the casting action is still visible.
    pub fn finish_cast(&mut self, entity_type: EntityType, client_tick: ClientTick) {
        let was_casting = self.casting.take().is_some();

        if was_casting && self.action_type == AnimationActionType::Skill {
            self.idle(entity_type, client_tick);
            self.time = 0;
        }
    }

    pub fn is_walking(&self) -> bool {
        self.action_type == AnimationActionType::Walk
    }

    pub fn is_dead(&self) -> bool {
        self.action_type == AnimationActionType::Die
    }

    pub fn update(&mut self, client_tick: ClientTick) {
        self.time = client_tick.0.wrapping_sub(self.start_time.0);

        if let Some(casting) = self.casting.as_mut() {
            casting.elapsed = client_tick.0.wrapping_sub(casting.start_time.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use ragnarok_packets::ClientTick;

    use super::AnimationState;
    use crate::world::EntityType;

    #[test]
    fn cast_progress_tracks_server_duration_and_clamps() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(100));
        state.cast(EntityType::Player, NonZeroU32::new(400).unwrap(), ClientTick(100));

        assert_eq!(state.casting_progress(), Some(0.0));
        assert!(!state.is_casting_complete());

        state.update(ClientTick(300));
        assert_eq!(state.casting_progress(), Some(0.5));
        assert!(!state.is_casting_complete());

        state.update(ClientTick(600));
        assert_eq!(state.casting_progress(), Some(1.0));
        assert!(state.is_casting_complete());
    }

    #[test]
    fn finishing_a_cast_clears_cast_progress() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(100));
        state.cast(EntityType::Player, NonZeroU32::new(400).unwrap(), ClientTick(100));
        state.finish_cast(EntityType::Player, ClientTick(200));

        assert!(!state.is_casting());
        assert_eq!(state.casting_progress(), None);
    }

    #[test]
    fn finishing_a_cast_preserves_a_newer_action() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(100));
        state.cast(EntityType::Player, NonZeroU32::new(400).unwrap(), ClientTick(100));
        state.attack(EntityType::Player, 200, false, ClientTick(200));
        state.finish_cast(EntityType::Player, ClientTick(250));

        assert!(!state.is_casting());
        assert!(state.is_attack());
    }

    #[test]
    fn death_clears_an_active_cast() {
        let mut state = AnimationState::new(EntityType::Player, ClientTick(100));
        state.cast(EntityType::Player, NonZeroU32::new(400).unwrap(), ClientTick(100));
        state.dead(EntityType::Player, ClientTick(200));

        assert!(!state.is_casting());
        assert!(state.is_dead());
    }
}

#[derive(RustState, Clone, StateElement)]
pub struct AnimationData {
    pub animation_pair: Vec<AnimationPair>,
    pub animations: Vec<Animation>,
    pub delays: Vec<f32>,
    #[hidden_element]
    pub entity_type: EntityType,
}

impl Cacheable for AnimationData {
    fn size(&self) -> usize {
        // We cache animations only by count.
        0
    }
}

#[derive(RustState, Clone, StateElement)]
pub struct AnimationPair {
    pub sprites: Arc<Sprite>,
    pub actions: Arc<Actions>,
}

#[derive(RustState, Clone, StateElement)]
pub struct Animation {
    #[hidden_element]
    pub frames: Vec<AnimationFrame>,
}

#[derive(Clone)]
pub struct AnimationFrame {
    pub event: Option<ActionEvent>,
    pub offset: Vector2<i32>,
    pub top_left: Vector2<i32>,
    pub size: Vector2<i32>,
    pub frame_parts: Vec<AnimationFramePart>,
    #[cfg(feature = "debug")]
    pub horizontal_matrix: Matrix4<f32>,
    #[cfg(feature = "debug")]
    pub vertical_matrix: Matrix4<f32>,
}

#[derive(Clone)]
pub struct AnimationFramePart {
    pub animation_index: usize,
    pub sprite_number: usize,
    pub offset: Vector2<i32>,
    pub size: Vector2<i32>,
    pub mirror: bool,
    pub angle: f32,
    pub color: Color,
    pub affine_matrix: Matrix4<f32>,
}

impl Default for AnimationFramePart {
    fn default() -> AnimationFramePart {
        AnimationFramePart {
            animation_index: usize::MAX,
            sprite_number: usize::MAX,
            offset: Vector2::<i32>::zero(),
            size: Vector2::<i32>::zero(),
            mirror: Default::default(),
            angle: Default::default(),
            color: Default::default(),
            affine_matrix: Matrix4::<f32>::zero(),
        }
    }
}

impl AnimationData {
    pub fn is_animation_over(&self, animation_state: &AnimationState) -> bool {
        let animation_action_index = animation_state.action_type.action_base_offset(self.entity_type) * 8;

        let delay_index = animation_action_index % self.delays.len();
        let animation_index = animation_action_index % self.animations.len();

        let delay = self.delays[delay_index];
        let animation = &self.animations[animation_index];

        let factor = animation_state.factor.map(|factor| delay * factor).unwrap_or_else(|| delay * 50.0);

        let frame_time = animation_state
            .duration
            .map(|duration| animation_state.time * animation.frames.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);

        frame_time as usize > animation.frames.len()
    }

    pub fn get_frame(&self, animation_state: &AnimationState, camera: &dyn Camera, direction: Direction) -> &AnimationFrame {
        let camera_direction = camera.camera_direction();
        let direction = (camera_direction + u16::from(direction) as usize) & 7;
        let animation_action_index = animation_state.action_type.action_base_offset(self.entity_type) * 8 + direction;

        let delay_index = animation_action_index % self.delays.len();
        let animation_index = animation_action_index % self.animations.len();

        let delay = self.delays[delay_index];
        let animation = &self.animations[animation_index];

        let factor = animation_state.factor.map(|factor| delay * factor).unwrap_or_else(|| delay * 50.0);

        let frame_time = animation_state
            .duration
            .map(|duration| animation_state.time * animation.frames.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);

        let frame_index = match animation_state.looping {
            true => frame_time as usize % animation.frames.len(),
            false => (frame_time as usize).min(animation.frames.len().saturating_sub(1)),
        };

        // Remove Doridori animation from Player
        if self.entity_type == EntityType::Player && animation_state.action_type == AnimationActionType::Idle {
            &animation.frames[0]
        } else {
            &animation.frames[frame_index]
        }
    }

    pub fn calculate_world_matrix(
        &self,
        camera: &dyn Camera,
        frame: &AnimationFrame,
        entity_position: Point3<f32>,
        scale: f32,
    ) -> Matrix4<f32> {
        // Offset the image to below the ground by frame.offset.y.
        // Add 0.5 to change from center of pixel to the lower border of pixel
        let origin_y = -frame.offset.y as f32 + 0.5;
        // TODO - TBD : Change the entity z coordinate to 0.0.
        // Add 1.0 in z-coordinate, because the entity is at point with z = 1.0.
        // The operation is performed beforehand to correctly rotate the billboard.
        let origin = Point3::new(0.0, origin_y, 0.0) * SPRITE_SCALE / TILE_SIZE + Vector3::unit_z();
        let size = Vector2::new(frame.size.x as f32, frame.size.y as f32) * (SPRITE_SCALE / TILE_SIZE) * scale;
        camera.billboard_matrix(entity_position, origin, size)
    }

    pub fn get_texture_coordinates(&self) -> (Vector2<f32>, Vector2<f32>) {
        let cell_count = Vector2::new(1, 1);
        let cell_position = Vector2::new(0, 0);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        (texture_size, texture_position)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        camera: &dyn Camera,
        add_to_picker: bool,
        entity_id: EntityId,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        direction: Direction,
        fade_alpha: f32,
        scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position, scale);

        for (index, frame_part) in frame.frame_parts.iter().enumerate() {
            let animation_index = frame_part.animation_index;
            let sprite_number = frame_part.sprite_number;
            let texture = &self.animation_pair[animation_index].sprites.textures[sprite_number];

            let frame_size = Vector2::new(frame.size.x as f32, frame.size.y as f32);

            let (texture_size, texture_position) = self.get_texture_coordinates();
            let (depth_offset, curvature) = camera.calculate_depth_offset_and_curvature(&world_matrix, SPRITE_SCALE, SPRITE_SCALE);

            let position = world_matrix.transform_point(Point3::from_value(0.0));
            let distance = camera.distance_to(position);
            let color = frame_part.color * fade_alpha;

            instructions.push(EntityInstruction {
                world: world_matrix,
                frame_part_transform: frame_part.affine_matrix,
                texture_position,
                texture_size,
                frame_size,
                depth_offset,
                extra_depth_offset: 0.005 * index as f32,
                curvature,
                color,
                mirror: frame_part.mirror,
                entity_id,
                add_to_picker,
                texture: texture.clone(),
                distance,
            });
        }
    }

    #[cfg(feature = "debug")]
    #[allow(clippy::too_many_arguments)]
    pub fn render_debug(
        &self,
        instructions: &mut Vec<DebugRectangleInstruction>,
        camera: &dyn Camera,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        direction: Direction,
        color_external: Color,
        color_internal: Color,
        scale: f32,
    ) {
        let frame = self.get_frame(animation_state, camera, direction);
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position, scale);
        instructions.push(DebugRectangleInstruction {
            world: world_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_matrix * frame.horizontal_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_matrix * frame.vertical_matrix,
            color: color_external,
        });

        for frame_part in frame.frame_parts.iter() {
            instructions.push(DebugRectangleInstruction {
                world: world_matrix * frame_part.affine_matrix,
                color: color_internal,
            });
        }
    }
}
