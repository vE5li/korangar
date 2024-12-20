use std::sync::Arc;

use cgmath::{Array, EuclideanSpace, Matrix4, Point3, Transform, Vector2, Zero};
use korangar_interface::elements::PrototypeElement;
use korangar_util::container::Cacheable;
use ragnarok_packets::EntityId;

#[cfg(feature = "debug")]
use crate::graphics::DebugRectangleInstruction;
use crate::graphics::{Color, EntityInstruction};
use crate::loaders::{ActionType, Actions, AnimationState, Sprite};
use crate::world::{Camera, EntityType};

const TILE_SIZE: f32 = 10.0;
const SPRITE_SCALE: f32 = 1.4;

#[derive(Clone, PrototypeElement)]
pub struct AnimationData {
    pub animation_pair: Vec<AnimationPair>,
    pub animations: Vec<Animation>,
    pub delays: Vec<f32>,
    #[hidden_element]
    pub entity_type: EntityType,
}

impl Cacheable for AnimationData {
    fn size(&self) -> usize {
        size_of_val(self)
    }
}

#[derive(Clone, PrototypeElement)]
pub struct AnimationPair {
    pub sprites: Arc<Sprite>,
    pub actions: Arc<Actions>,
}

#[derive(Clone, PrototypeElement)]
pub struct Animation {
    #[hidden_element]
    pub frames: Vec<AnimationFrame>,
}

#[derive(Clone)]
pub struct AnimationFrame {
    pub offset: Vector2<i32>,
    pub top_left: Vector2<i32>,
    pub size: Vector2<i32>,
    pub frame_parts: Vec<AnimationFramePart>,
    #[cfg(feature = "debug")]
    pub offset_matrix: Matrix4<f32>,
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
    pub fn get_frame(&self, animation_state: &AnimationState, camera: &dyn Camera, head_direction: usize) -> &AnimationFrame {
        let camera_direction = camera.camera_direction();
        let direction = (camera_direction + head_direction) % 8;
        let aa = animation_state.action as usize * 8 + direction;
        let delay = self.delays[aa % self.delays.len()];
        let animation = &self.animations[aa % self.animations.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame_time = animation_state
            .duration
            .map(|duration| animation_state.time * animation.frames.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);

        // TODO: Work out how to avoid losing digits when casting time to an f32. When
        //       fixed remove set_start_time in MouseCursor.
        let time = frame_time as usize % animation.frames.len();
        let mut frame = &animation.frames[time];

        // Remove Doridori animation from Player
        if self.entity_type == EntityType::Player && animation_state.action == ActionType::Idle {
            frame = &animation.frames[0];
        }
        return frame;
    }

    pub fn calculate_world_matrix(&self, camera: &dyn Camera, frame: &AnimationFrame, entity_position: Point3<f32>) -> Matrix4<f32> {
        // The vertex position is calculated from the center of image, so we need
        // to add half of the height.
        let center_position = Vector2::new(-frame.offset.x as f32, frame.offset.y as f32 + ((frame.size.y - 1) / 2) as f32);
        let origin = Point3::from_vec(center_position.extend(0.0)) * SPRITE_SCALE / TILE_SIZE;
        let size = Vector2::new(frame.size.x as f32, frame.size.y as f32) * SPRITE_SCALE / TILE_SIZE;
        let world_matrix = camera.billboard_matrix(entity_position, origin, size);

        return world_matrix;
    }

    pub fn get_texture_coordinates(&self) -> (Vector2<f32>, Vector2<f32>) {
        let cell_count = Vector2::new(1, 1);
        let cell_position = Vector2::new(0, 0);
        let texture_size = Vector2::new(1.0 / cell_count.x as f32, 1.0 / cell_count.y as f32);
        let texture_position = Vector2::new(texture_size.x * cell_position.x as f32, texture_size.y * cell_position.y as f32);
        return (texture_size, texture_position);
    }

    pub fn render(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        camera: &dyn Camera,
        entity_id: EntityId,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        head_direction: usize,
        add_to_picker: bool,
    ) {
        let frame = self.get_frame(animation_state, camera, head_direction);
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position);

        for (index, frame_part) in frame.frame_parts.iter().enumerate() {
            let animation_index = frame_part.animation_index;
            let sprite_number = frame_part.sprite_number;
            let texture = &self.animation_pair[animation_index].sprites.textures[sprite_number];

            let frame_size = Vector2::new(frame.size.x as f32, frame.size.y as f32);

            let (texture_size, texture_position) = self.get_texture_coordinates();
            let (depth_offset, curvature) = camera.calculate_depth_offset_and_curvature(&world_matrix, SPRITE_SCALE, SPRITE_SCALE);

            let position = world_matrix.transform_point(Point3::from_value(0.0));
            let distance = camera.distance_to(position);

            instructions.push(EntityInstruction {
                world: world_matrix,
                frame_part_transform: frame_part.affine_matrix,
                texture_position,
                texture_size,
                frame_size,
                depth_offset,
                extra_depth_offset: 0.005 * index as f32,
                curvature,
                color: frame_part.color,
                mirror: frame_part.mirror,
                entity_id,
                add_to_picker,
                texture: texture.clone(),
                distance,
            });
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_debug(
        &self,
        instructions: &mut Vec<DebugRectangleInstruction>,
        camera: &dyn Camera,
        entity_position: Point3<f32>,
        animation_state: &AnimationState,
        head_direction: usize,
        color_external: Color,
        color_internal: Color,
    ) {
        let frame = self.get_frame(animation_state, camera, head_direction);
        let world_matrix = self.calculate_world_matrix(camera, frame, entity_position);
        let world_shift_offset = world_matrix * frame.offset_matrix;

        instructions.push(DebugRectangleInstruction {
            world: world_shift_offset,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_shift_offset * frame.horizontal_matrix,
            color: color_external,
        });
        instructions.push(DebugRectangleInstruction {
            world: world_shift_offset * frame.vertical_matrix,
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
