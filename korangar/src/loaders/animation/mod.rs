use std::cmp::{max, min};
use std::num::{NonZeroU32, NonZeroUsize};
use std::sync::Arc;

use cgmath::{Matrix4, Rad, Vector2};
use korangar_util::container::SimpleCache;
use num::Zero;

use super::error::LoadError;
use crate::loaders::{ActionLoader, SpriteLoader};
use crate::world::{Animation, AnimationData, AnimationFrame, AnimationFramePart, AnimationPair};
use crate::{Color, EntityType};

const MAX_CACHE_COUNT: u32 = 256;
const MAX_CACHE_SIZE: usize = 64 * 1024 * 1024;

pub struct AnimationLoader {
    cache: SimpleCache<Vec<String>, Arc<AnimationData>>,
}

impl AnimationLoader {
    pub fn new() -> Self {
        Self {
            cache: SimpleCache::new(
                NonZeroU32::new(MAX_CACHE_COUNT).unwrap(),
                NonZeroUsize::new(MAX_CACHE_SIZE).unwrap(),
            ),
        }
    }

    pub fn load(
        &mut self,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        entity_type: EntityType,
        entity_part_files: &[String],
    ) -> Result<Arc<AnimationData>, LoadError> {
        let animation_pairs: Vec<AnimationPair> = entity_part_files
            .iter()
            .map(|file_path| AnimationPair {
                sprites: sprite_loader.get(&format!("{file_path}.spr")).unwrap(),
                actions: action_loader.get(&format!("{file_path}.act")).unwrap(),
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
                        let frame = AnimationFrame {
                            size,
                            top_left: Vector2::zero(),
                            offset,
                            frame_parts: vec![frame_part],
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
        for action_index in 0..action_size {
            animations[action_index].frames.iter().for_each(|frame| {
                let center_x = (frame.size.x - 1) / 2;
                min_left = min(min_left, frame.offset.x - center_x);
                max_right = max(max_right, frame.offset.x + (frame.size.x - 1 - center_x));

                let center_y = (frame.size.y - 1) / 2;
                min_top = min(min_top, frame.offset.y - center_y);
                max_bottom = max(max_bottom, frame.offset.y + (frame.size.y - 1 - center_y));
            });
        }
        // The player can change the sprite and causes an offset from a image to
        // another.
        if entity_type == EntityType::Player {
            // Create a bounding box to standardize the size of the player sprite, ensuring
            // that every sprite has the same proportion and that pixels are rendered in
            // the correct position.
            // The player sprite is generally 110 pixels, with an additional 40 pixels added
            // for extra space, making the total height 150 pixels.
            let extra_size = 20;
            min_top = -80;
            max_bottom = 30;
            min_top -= extra_size;
            max_bottom += extra_size;
            min_left = -150;
            max_right = 150;
        }

        fn calculate_new_size(min_top: i32, max_bottom: i32, min_left: i32, max_right: i32) -> Vector2<i32> {
            // Create a rectangle centered on the y-axis, extending to the maximum of
            // max_left and max_right.
            let size_x = 2 * max(i32::abs(min_left), i32::abs(max_right)) + 1;

            // The size_y is defined to be odd, as it ranges from [0, 2k],
            // resulting in a size of 2k+1.
            let mut padding = 1;
            if max_bottom % 2 == min_top % 2 {
                padding = 0;
            }
            let size_y = max_bottom - min_top + padding + 1;

            return Vector2::new(size_x, size_y);
        }

        for action_index in 0..action_size {
            animations[action_index].frames.iter_mut().for_each(|frame| {
                frame.size = calculate_new_size(min_top, max_bottom, min_left, max_right);
                // Set the origin point at (0, 0) correctly by applying an offset in y by
                // max_bottom.
                frame.offset = Vector2::new(0, -max_bottom);
                for frame_part in frame.frame_parts.iter_mut() {
                    // Determine the top-left corner of the frame rectangle
                    // and the top-left corner of the frame part rectangle.
                    let frame_top_left = frame.offset - (frame.size - Vector2::new(1, 1)) / 2;
                    let frame_part_top_left = frame_part.offset - (frame_part.size - Vector2::new(1, 1)) / 2;

                    // Generate the key points of the frame rectangle.
                    let texture_frame_center = Vector2::new(0.0, 1.0);

                    // Generate the key points of the frame part rectangle.
                    // In the variables, we removed the term frame_part,
                    // but we are still working with the frame part.
                    let new_vector = frame_part.size;
                    let top_left = frame_part_top_left - frame_top_left;
                    let bottom_left = top_left + new_vector.y * Vector2::<i32>::unit_y();
                    let bottom_right = top_left + new_vector;

                    let texture_top_left = convert_coordinates(top_left, frame.size);
                    let texture_bottom_left = convert_coordinates(bottom_left, frame.size);
                    let texture_bottom_right = convert_coordinates(bottom_right, frame.size);

                    // 1 - Move to the center of the frame rectangle with coordinates
                    // (-1, 2), (-1, 0), (1, 2), (1, 0).
                    // 2 - Rotate the image by specified angle.
                    // 3 - Return to the origin to apply the scaling.
                    let translation_to_center_matrix = Matrix4::from_translation(texture_frame_center.extend(0.0));
                    let rotation_matrix = Matrix4::from_angle_z(Rad(-frame_part.angle));
                    let translation_to_origin_matrix = Matrix4::from_translation((-texture_frame_center).extend(0.0));
                    let final_rotation_matrix = translation_to_center_matrix * rotation_matrix * translation_to_origin_matrix;

                    // Scale the vertices (-1, 2), (-1, 0), (1, 2), (1, 0) to
                    // match the texture coordinates as specified above.
                    let scale_matrix = Matrix4::from_nonuniform_scale(
                        (texture_bottom_right.x - texture_bottom_left.x) / 2.0,
                        (texture_top_left.y - texture_bottom_left.y) / 2.0,
                        1.0,
                    );

                    // Translate the scaled rectangle from the new texture center to the
                    // texture center.
                    let texture_center = (texture_top_left + texture_bottom_right) / 2.0;
                    let texture_frame_new_center = Vector2::new(0.0, (texture_top_left.y - texture_bottom_left.y) / 2.0);
                    let translation_matrix = Matrix4::from_translation(texture_center.extend(1.0) - texture_frame_new_center.extend(1.0));

                    frame_part.affine_matrix = translation_matrix * scale_matrix * final_rotation_matrix;
                }
            });
        }

        let animation_data = Arc::new(AnimationData {
            delays: animation_pairs[0].actions.delays.clone(),
            animation_pair: animation_pairs,
            animations,
            entity_type,
        });

        self.cache.insert(entity_part_files.to_vec(), animation_data.clone()).unwrap();

        Ok(animation_data)
    }

    pub fn get(
        &mut self,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        entity_type: EntityType,
        entity_part_files: &[String],
    ) -> Result<Arc<AnimationData>, LoadError> {
        match self.cache.get(entity_part_files) {
            Some(animation_data) => Ok(animation_data.clone()),
            None => self.load(sprite_loader, action_loader, entity_type, entity_part_files),
        }
    }
}

/// This function converts to the "normalized" coordinates of a frame part
/// inside the frame's bounding rectangle, defined by the vertices [-1, 0],
/// [-1, 2], [1, 0], and [1, 2].
fn convert_coordinates(coordinates: Vector2<i32>, size: Vector2<i32>) -> Vector2<f32> {
    assert!(size.x != 0 && size.y != 0);
    let x = (coordinates.x as f32 / size.x as f32 - 1.0 / 2.0) * 2.0;
    let y = 2.0 - (coordinates.y as f32 / size.y as f32) * 2.0;
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
            size: Vector2::new(1, 1),
            top_left: Vector2::zero(),
            offset: Vector2::zero(),
            frame_parts: vec![frame_part],
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
        size: Vector2::new(new_width, new_height),
        top_left: Vector2::zero(),
        offset: Vector2::new(top_left_x + (new_width - 1) / 2, top_left_y + (new_height - 1) / 2),
        frame_parts: new_frame_parts,
    }
}
