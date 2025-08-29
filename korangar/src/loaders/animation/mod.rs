use std::cmp::{max, min};
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::{Arc, Mutex};

#[cfg(feature = "debug")]
use cgmath::SquareMatrix;
use cgmath::{Array, Matrix4, Rad, Vector2};
use korangar_util::container::SimpleCache;
use num::Zero;

use super::error::LoadError;
use crate::loaders::{ActionLoader, SpriteLoader};
use crate::world::{ActionEvent, Animation, AnimationData, AnimationFrame, AnimationFramePart, AnimationPair};
use crate::{Color, EntityType};

const MAX_CACHE_COUNT: u32 = 1000;
const MAX_CACHE_SIZE: usize = 64 << 20;

pub struct AnimationLoader {
    cache: Mutex<SimpleCache<Vec<String>, Arc<AnimationData>>>,
}

impl AnimationLoader {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            )),
        }
    }

    pub fn load(
        &self,
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        entity_type: EntityType,
        entity_part_files: &[String],
    ) -> Result<Arc<AnimationData>, LoadError> {
        let animation_pairs: Vec<AnimationPair> = entity_part_files
            .iter()
            .map(|file_path| AnimationPair {
                sprites: sprite_loader.get_or_load(&format!("{file_path}.spr")).unwrap(),
                actions: action_loader.get_or_load(&format!("{file_path}.act")).unwrap(),
            })
            .collect();

        let mut animations_list: Vec<Vec<Vec<AnimationFrame>>> = Vec::new();

        // Each animation pair consists of pairs of sprite and action.
        // Each entity has multiple actions, and each action is composed of
        // several motions. Each motion references multiple sprites
        // that we want to combine.
        for (animation_index, animation_pair) in animation_pairs.iter().enumerate() {
            let mut animation_frames: Vec<Vec<AnimationFrame>> = Vec::new();

            for (action_index, action) in animation_pair.actions.actions.iter().enumerate() {
                let mut action_frames: Vec<AnimationFrame> = Vec::new();

                for (motion_index, motion) in action.motions.iter().enumerate() {
                    let mut motion_frames: Vec<AnimationFrame> = Vec::new();

                    if motion.sprite_clip_count == 0 {
                        continue;
                    }

                    for sprite_clip in motion.sprite_clips.iter() {
                        if sprite_clip.sprite_number == -1 {
                            continue;
                        }

                        let mut sprite_number = sprite_clip.sprite_number as usize;
                        // The sprite type is 0 for palette and 1 for BGRA.
                        let sprite_type = match sprite_clip.sprite_type {
                            Some(value) => value as usize,
                            None => 0,
                        };

                        if sprite_type == 1 {
                            sprite_number += animation_pair.sprites.palette_size;
                        }

                        let texture_size = animation_pair.sprites.textures[sprite_number].get_size();
                        let mut height = texture_size.height;
                        let mut width = texture_size.width;

                        let color = match sprite_clip.color {
                            Some(color) => {
                                let alpha = (((color >> 24) & 0xFF) as u8) as f32 / 255.0;
                                let blue = (((color >> 16) & 0xFF) as u8) as f32 / 255.0;
                                let green = (((color >> 8) & 0xFF) as u8) as f32 / 255.0;
                                let red = (((color) & 0xFF) as u8) as f32 / 255.0;

                                Color { red, green, blue, alpha }
                            }
                            None => Color {
                                red: 0.0,
                                green: 0.0,
                                blue: 0.0,
                                alpha: 0.0,
                            },
                        };

                        // Scale the image. Attempt to match the first type of zoom.
                        // If it doesn't match, use the second type of zoom.
                        let zoom = match sprite_clip.zoom {
                            Some(value) => (value, value).into(),
                            None => sprite_clip.zoom2.unwrap_or_else(|| (1.0, 1.0).into()),
                        };
                        if zoom != (1.0, 1.0).into() {
                            width = (width as f32 * zoom.x).ceil() as u32;
                            height = (height as f32 * zoom.y).ceil() as u32;
                        }

                        let angle = match sprite_clip.angle {
                            Some(value) => value as f32 / 360.0 * 2.0 * std::f32::consts::PI,
                            None => 0.0,
                        };

                        let mut offset = sprite_clip.position.map(|component| component);
                        let mirror = sprite_clip.mirror_on != 0;

                        // Attach points have a different offset calculation.
                        // Currently, this is hardcoded for the player heads. An `animation_index` of
                        // `0` corresponds to the head, and `1` for the body.
                        let has_attach_point = match motion.attach_point_count {
                            Some(value) => value == 1,
                            None => false,
                        };

                        if entity_type == EntityType::Player && has_attach_point && animation_index == 1 {
                            let parent_animation_pair = &animation_pairs[0];
                            let parent_action = &parent_animation_pair.actions.actions[action_index];
                            // TODO: Precompute the size of each motion from the animation pair.
                            // Determine the minimum motion size to iterate without going out of bound.
                            // This check resolves the game crash when using the Assassin class.
                            if parent_action.motions.len() <= motion_index {
                                continue;
                            }
                            let parent_motion = &parent_action.motions[motion_index];
                            let parent_attach_point = parent_motion.attach_points[0].position;
                            let attach_point = motion.attach_points[0].position;
                            let new_offset = -attach_point + parent_attach_point;
                            offset += new_offset;
                        }

                        let size = Vector2::new(width as i32, height as i32);
                        let frame_part = AnimationFramePart {
                            animation_index,
                            sprite_number,
                            size,
                            offset,
                            mirror,
                            angle,
                            color,
                            ..Default::default()
                        };

                        let event: Option<ActionEvent> = if let Some(event_id) = motion.event_id
                            && event_id != -1
                            && let Some(event) = animation_pair.actions.events.get(event_id as usize).copied()
                        {
                            Some(event)
                        } else {
                            None
                        };

                        let frame = AnimationFrame {
                            event,
                            size,
                            top_left: Vector2::zero(),
                            offset,
                            frame_parts: vec![frame_part],
                            #[cfg(feature = "debug")]
                            horizontal_matrix: Matrix4::identity(),
                            #[cfg(feature = "debug")]
                            vertical_matrix: Matrix4::identity(),
                        };

                        motion_frames.push(frame);
                    }

                    let frame = match motion_frames.len() {
                        1 => motion_frames[0].clone(),
                        _ => merge_frame(&mut motion_frames),
                    };

                    action_frames.push(frame);
                }

                animation_frames.push(action_frames);
            }

            animations_list.push(animation_frames);
        }

        let action_size = animation_pairs[0].actions.actions.len();
        let animation_pair_size = animation_pairs.len();

        let mut animations: Vec<Animation> = Vec::new();

        // Merge the sprites from each motion by combining the animation pair.
        for action_index in 0..action_size {
            let motion_size = animation_pairs[0].actions.actions[action_index].motions.len();
            let mut frames: Vec<AnimationFrame> = Vec::new();
            for motion_index in 0..motion_size {
                let mut generate: Vec<AnimationFrame> = Vec::new();

                for pair in &animations_list[0..animation_pair_size] {
                    if pair.len() <= action_index || pair[action_index].len() <= motion_index {
                        continue;
                    }
                    generate.push(pair[action_index][motion_index].clone());
                }
                let frame = merge_frame(&mut generate);
                frames.push(frame);
            }
            animations.push(Animation { frames });
        }

        // Generate bounding-box per action
        for animation in animations.iter_mut().take(action_size) {
            // The problem is that each frame of an action is not the same size
            // and without size consistency, the proportion differ between frames,
            // causing pixel offsets.
            // To resolve this, we resized the frames to ensure
            // identical frame size and origin point at (0,0) between frames.

            // First, identify the bounding box that encompasses all motion
            // frames.
            let mut min_top = i32::MAX;
            let mut max_bottom = 0;
            let mut min_left = i32::MAX;
            let mut max_right = 0;

            animation.frames.iter().for_each(|frame| {
                let center_x = (frame.size.x - 1) / 2;
                min_left = min(min_left, frame.offset.x - center_x);
                max_right = max(max_right, frame.offset.x + (frame.size.x - 1 - center_x));

                let center_y = (frame.size.y - 1) / 2;
                min_top = min(min_top, frame.offset.y - center_y);
                max_bottom = max(max_bottom, frame.offset.y + (frame.size.y - 1 - center_y));
            });

            // From here, the code start not using pixel coordinates (center of pixel
            // coordinates), but instead pixel border + pixel center coordinates system.
            // This also means that we start using float numbers.
            // During rendering, the pixel border is required to determine the coordinates
            // to display the billboard on the screen.
            // This is the main reason to change the coordinate system.
            // |x|x|x| - x represents the pixel center and | represents the pixel border.
            // 'x' is in the format of numbers ending with .0 (e.g., X.0).
            // '|' is in the format of numbers ending with .5 (e.g., X.5).
            animation.frames.iter_mut().for_each(|frame| {
                frame.size = calculate_new_size(min_top, max_bottom, min_left, max_right);
                // Set the bottom part of the image as (0, 0), to shift easily after rotation
                frame.offset = Vector2::new(0, -max_bottom);

                #[cfg(feature = "debug")]
                get_cross_matrix(frame);
                for frame_part in frame.frame_parts.iter_mut() {
                    let frame_size = vector2_i32_to_f32(frame.size);
                    let frame_origin = vector2_i32_to_f32(frame.offset);

                    // The offset is used to shift to the pixel corner.
                    let frame_top_left = Vector2::new(-frame_size.x / 2.0, -frame_size.y + 0.5);

                    let frame_part_top_left_shift = vector2_i32_to_f32(-((frame_part.size - Vector2::new(1, 1)) / 2));
                    let frame_part_offset = vector2_i32_to_f32(frame_part.offset);
                    // The offset is used to shift from the pixel center to the pixel corner.
                    let frame_part_top_left =
                        (frame_origin + frame_part_offset + frame_part_top_left_shift) - Vector2::<f32>::from_value(0.5);

                    // Generate the key points of the frame rectangle.
                    let texture_frame_center = Vector2::new(0.0, 1.0);
                    // Generate the key points of the frame part rectangle.
                    // In the variables, we removed the term frame_part,
                    // but we are still working with the frame part.
                    let new_vector = vector2_i32_to_f32(frame_part.size);
                    let top_left = frame_part_top_left - frame_top_left;
                    let bottom_left = top_left + new_vector.y * Vector2::<f32>::unit_y();
                    let bottom_right = top_left + new_vector;

                    let texture_top_left = convert_coordinates(top_left, frame_size);
                    let texture_bottom_left = convert_coordinates(bottom_left, frame_size);
                    let texture_bottom_right = convert_coordinates(bottom_right, frame_size);

                    let rotation_matrix = calculate_recenter_rotation_matrix(texture_frame_center, -frame_part.angle);
                    let scale_matrix = calculate_scale_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
                    let translation_matrix = calculate_translation_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
                    frame_part.affine_matrix = translation_matrix * scale_matrix * rotation_matrix;
                }
            });
        }

        let animation_data = Arc::new(AnimationData {
            delays: animation_pairs[0].actions.delays.clone(),
            animation_pair: animation_pairs,
            animations,
            entity_type,
        });

        self.cache
            .lock()
            .unwrap()
            .insert(entity_part_files.to_vec(), animation_data.clone())
            .unwrap();

        Ok(animation_data)
    }

    pub fn get(&self, entity_part_files: &[String]) -> Option<Arc<AnimationData>> {
        let mut lock = self.cache.lock().unwrap();
        lock.get(entity_part_files).cloned()
    }
}

fn vector2_i32_to_f32(vector: Vector2<i32>) -> Vector2<f32> {
    vector.map(|value| value as f32)
}

fn calculate_new_size(min_top: i32, max_bottom: i32, min_left: i32, max_right: i32) -> Vector2<i32> {
    // Create a rectangle centered on the y-axis, extending to the maximum of
    // max_left and max_right.
    let size_x = 2 * i32::abs(min_left).max(i32::abs(max_right)) + 1;

    // The size_y is defined to be odd, as it ranges from [0, 2k],
    // resulting in a size of 2k+1.
    let mut padding = 1;
    if (max_bottom - min_top) % 2 == 0 {
        padding = 0;
    }
    let size_y = max_bottom - min_top + 1 + padding;

    Vector2::new(size_x, size_y)
}

#[cfg(feature = "debug")]
fn get_cross_matrix(frame: &mut AnimationFrame) {
    let frame_size = vector2_i32_to_f32(frame.size);
    let frame_top_left = Vector2::new(-frame_size.x / 2.0, -frame_size.y + 0.5);
    let frame_origin = vector2_i32_to_f32(frame.offset);
    let bounding_box_origin = frame_origin - frame_top_left;

    let texture_top_left = convert_coordinates(Vector2::new(0.0, bounding_box_origin.y), frame_size);
    let texture_bottom_left = convert_coordinates(Vector2::new(0.0, bounding_box_origin.y), frame_size);
    let texture_bottom_right = convert_coordinates(Vector2::new(frame_size.x, bounding_box_origin.y), frame_size);
    let scale_matrix = calculate_scale_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
    let translation_matrix = calculate_translation_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
    frame.horizontal_matrix = translation_matrix * scale_matrix;

    let texture_top_left = convert_coordinates(Vector2::new(bounding_box_origin.x, 0.0), frame_size);
    let texture_bottom_left = convert_coordinates(Vector2::new(bounding_box_origin.x, frame_size.y), frame_size);
    let texture_bottom_right = convert_coordinates(Vector2::new(bounding_box_origin.x, frame_size.y), frame_size);
    let scale_matrix = calculate_scale_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
    let translation_matrix = calculate_translation_matrix(texture_top_left, texture_bottom_left, texture_bottom_right);
    frame.vertical_matrix = translation_matrix * scale_matrix;
}

fn calculate_recenter_rotation_matrix(texture_frame_center: Vector2<f32>, angle: f32) -> Matrix4<f32> {
    // 1 - Move to the center of the frame rectangle with coordinates
    // (-1, 2), (-1, 0), (1, 2), (1, 0).
    // 2 - Rotate the image by specified angle.
    // 3 - Return to the origin to apply the scaling.
    let translation_to_center_matrix = Matrix4::from_translation(texture_frame_center.extend(0.0));
    let rotation_matrix = Matrix4::from_angle_z(Rad(angle));
    let translation_to_origin_matrix = Matrix4::from_translation((-texture_frame_center).extend(0.0));
    translation_to_center_matrix * rotation_matrix * translation_to_origin_matrix
}

fn calculate_scale_matrix(
    texture_top_left: Vector2<f32>,
    texture_bottom_left: Vector2<f32>,
    texture_bottom_right: Vector2<f32>,
) -> Matrix4<f32> {
    // Scale the vertices (-1, 2), (-1, 0), (1, 2), (1, 0) to
    // match the texture coordinates.
    Matrix4::from_nonuniform_scale(
        (texture_bottom_right.x - texture_bottom_left.x) / 2.0,
        (texture_top_left.y - texture_bottom_left.y) / 2.0,
        1.0,
    )
}

fn calculate_translation_matrix(
    texture_top_left: Vector2<f32>,
    texture_bottom_left: Vector2<f32>,
    texture_bottom_right: Vector2<f32>,
) -> Matrix4<f32> {
    // Translate the scaled rectangle from the new texture center to the
    // texture center.
    let texture_center = (texture_top_left + texture_bottom_right) / 2.0;
    let texture_frame_new_center = Vector2::new(0.0, (texture_top_left.y - texture_bottom_left.y) / 2.0);
    Matrix4::from_translation(texture_center.extend(1.0) - texture_frame_new_center.extend(1.0))
}

/// This function converts to the "normalized" coordinates of a frame part
/// inside the frame's bounding rectangle, defined by the vertices [-1, 0],
/// [-1, 2], [1, 0], and [1, 2].
fn convert_coordinates(coordinates: Vector2<f32>, size: Vector2<f32>) -> Vector2<f32> {
    assert!(size.x != 0.0 && size.y != 0.0);
    let x = (coordinates.x / size.x - 1.0 / 2.0) * 2.0;
    let y = 2.0 - (coordinates.y / size.y) * 2.0;
    Vector2::<f32>::new(x, y)
}

/// This function generates a new frame by merging a list of frames.
fn merge_frame(frames: &mut [AnimationFrame]) -> AnimationFrame {
    for frame in frames.iter_mut() {
        // For an even side with a length of 4, the center coordinate is 1.
        // For an odd side with a length of 3, the center coordinate is 1.
        let half_size = (frame.size - Vector2::new(1, 1)) / 2;
        frame.top_left = frame.offset - half_size;
    }

    // If the function input contains no frame, return a 1-pixel image.
    if frames.is_empty() {
        let frame_part = AnimationFramePart {
            animation_index: usize::MAX,
            sprite_number: usize::MAX,
            size: Vector2::new(1, 1),
            offset: Vector2::zero(),
            mirror: false,
            color: Color {
                red: 0.0,
                blue: 0.0,
                green: 0.0,
                alpha: 0.0,
            },
            ..Default::default()
        };

        let frame = AnimationFrame {
            event: None,
            size: Vector2::new(1, 1),
            top_left: Vector2::zero(),
            offset: Vector2::zero(),
            frame_parts: vec![frame_part],
            #[cfg(feature = "debug")]
            horizontal_matrix: Matrix4::identity(),
            #[cfg(feature = "debug")]
            vertical_matrix: Matrix4::identity(),
        };

        return frame;
    }

    // Determine the upmost and leftmost coordinates.
    let top_left_x = frames.iter().min_by_key(|frame| frame.top_left.x).unwrap().top_left.x;
    let top_left_y = frames.iter().min_by_key(|frame| frame.top_left.y).unwrap().top_left.y;

    // Determine the bottommost and rightmost coordinates.
    let frame_x = frames.iter().max_by_key(|frame| frame.top_left.x + frame.size.x).unwrap();
    let frame_y = frames.iter().max_by_key(|frame| frame.top_left.y + frame.size.y).unwrap();

    // Calculate the new rectangle formed from the bounding box of
    // the given rectangles.
    let new_width = (frame_x.top_left.x + frame_x.size.x) - top_left_x;
    let new_height = (frame_y.top_left.y + frame_y.size.y) - top_left_y;

    let mut new_frame_parts = Vec::with_capacity(frames.iter().map(|frame| frame.frame_parts.len()).sum());
    for frame in frames.iter_mut() {
        new_frame_parts.append(&mut frame.frame_parts);
    }

    let event = frames.iter().filter_map(|frame| frame.event).next();

    // The origin is set at (0,0).
    //
    // The top-left point of the rectangle is calculated as
    // origin + offset - half_size.
    //
    // The center point of the rectangle is calculated as
    // top_left_point +  half_size.
    //
    // The new offset is calculated as
    // center_point - origin.
    AnimationFrame {
        event,
        size: Vector2::new(new_width, new_height),
        top_left: Vector2::zero(),
        offset: Vector2::new(top_left_x + (new_width - 1) / 2, top_left_y + (new_height - 1) / 2),
        frame_parts: new_frame_parts,
        #[cfg(feature = "debug")]
        horizontal_matrix: Matrix4::identity(),
        #[cfg(feature = "debug")]
        vertical_matrix: Matrix4::identity(),
    }
}
