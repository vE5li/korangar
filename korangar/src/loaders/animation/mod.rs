use std::cmp::{max, min};
use std::collections::HashMap;
use std::sync::Arc;

use cgmath::{Deg, Vector2};
use derive_new::new;
use image::imageops::FilterType;
use image::{save_buffer, Pixel, Rgba, RgbaImage};
use imageproc::geometric_transformations::{rotate_about_center, Interpolation};
use korangar_interface::elements::PrototypeElement;
use num::Zero;
use ragnarok_formats::sprite::RgbaImageData;
use wgpu::{Device, Extent3d, Queue, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

use super::error::LoadError;
use crate::graphics::Texture;
use crate::loaders::{ActionLoader, Actions, AnimationState, Sprite, SpriteLoader};
use crate::EntityType;

// TODO: use cache later, the memory will be increase with this hashmap, until
// the program is out of memory.
#[derive(new)]
pub struct AnimationLoader {
    device: Arc<Device>,
    queue: Arc<Queue>,

    #[new(default)]
    // The string will be type of entity
    // 0_{body_id}_{head_id}
    // 1_{monster_id}
    // 2_{npc_id}
    cache: HashMap<String, AnimationData>,
}

impl AnimationLoader {
    pub fn load(
        &mut self,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        entity_filename: Vec<String>,
        entity_type: EntityType,
    ) -> Result<AnimationData, LoadError> {
        // Create animation pair with sprite and action
        let vec: Vec<AnimationPair> = entity_filename
            .iter()
            .map(|file_path| AnimationPair {
                sprites: sprite_loader.get(&format!("{file_path}.spr")).unwrap(),
                actions: action_loader.get(&format!("{file_path}.act")).unwrap(),
            })
            .collect();

        // The sprite is stored in animation_pair.sprites.rgba_images or
        // animation_pair.sprites.palette_images

        // For each animation, we collect all the framepart need to generate
        let mut animations_list: Vec<Vec<Vec<FramePart>>> = Vec::new();
        let mut animation_index: usize = 0;

        for animation_pair in vec.iter() {
            let mut animation_frames: Vec<Vec<FramePart>> = Vec::new();
            for action in animation_pair.actions.actions.iter() {
                let mut action_frames: Vec<FramePart> = Vec::new();
                for motion in action.motions.iter() {
                    let mut motion_frames: Vec<FramePart> = Vec::new();
                    if motion.sprite_clip_count == 0 {
                        continue;
                    }
                    for position in 0..motion.sprite_clip_count {
                        let frame_part: FramePart;
                        let pos = position as usize;
                        let sprite_number = motion.sprite_clips[pos].sprite_number;
                        if sprite_number == -1 {
                            continue;
                        }
                        // TODO: using the information from the sprite to directly infer if is rgba or
                        // palette image
                        let rgba_image_data = match animation_pair.sprites.palette_count {
                            0 => animation_pair.sprites.rgba_images[sprite_number as usize].clone(),
                            _ => animation_pair.sprites.palette_images[sprite_number as usize].clone(),
                        };

                        let mut rgba_image: RgbaImage = RgbaImage::from_raw(
                            rgba_image_data.width.into(),
                            rgba_image_data.height.into(),
                            rgba_image_data.data.clone(),
                        )
                        .unwrap();
                        // Apply color filter in the image
                        let color = match motion.sprite_clips[pos].color {
                            Some(value) => value,
                            None => 0,
                        };
                        if color != 0 {
                            let alpha = ((((color >> 24) & 0xFF) as u8) as f32 / 255.0) as f32;
                            let blue = ((((color >> 16) & 0xFF) as u8) as f32 / 255.0) as f32;
                            let green = ((((color >> 8) & 0xFF) as u8) as f32 / 255.0) as f32;
                            let red = ((((color) & 0xFF) as u8) as f32 / 255.0) as f32;

                            let height = rgba_image.height();
                            let width = rgba_image.width();
                            for y in 0..height {
                                for x in 0..width {
                                    let pixel: &mut Rgba<u8> = rgba_image.get_pixel_mut(x, y);
                                    pixel.0[0] = (pixel.0[0] as f32 * red) as u8;
                                    pixel.0[1] = (pixel.0[1] as f32 * green) as u8;
                                    pixel.0[2] = (pixel.0[2] as f32 * blue) as u8;
                                    pixel.0[3] = (pixel.0[3] as f32) as u8; // TODO: Multiply by alpha later
                                }
                            }
                        }

                        // Scale the image
                        // Try to match the first type of zoom, if doesn't match find the second method
                        let zoom = match motion.sprite_clips[pos].zoom {
                            Some(value) => (value, value).into(),
                            None => match motion.sprite_clips[pos].zoom2 {
                                Some(value) => value,
                                None => (1.0, 1.0).into(),
                            },
                        };
                        if (zoom.x - 1.0).abs() >= 0.0001 || (zoom.y - 1.0).abs() >= 0.0001 {
                            let new_width = (rgba_image_data.width as f32 * zoom.x).floor() as u32;
                            let new_height = (rgba_image_data.height as f32 * zoom.y).floor() as u32;
                            rgba_image = image::imageops::resize(&rgba_image, new_width, new_height, FilterType::Nearest);
                        }

                        // Rotate the image
                        // TODO: This rotate_about_center cut the parts that not inside the initial
                        // rotate side, need address the cut parts
                        let angle = match motion.sprite_clips[pos].angle {
                            Some(value) => value,
                            None => 0,
                        };
                        if angle == 0 {
                            let angle_radian = (angle as f32 / 360.0) * 2.0 * std::f32::consts::PI;
                            rgba_image = rotate_about_center(&rgba_image, angle_radian, Interpolation::Nearest, Rgba([0u8, 0u8, 0u8, 0u8]));
                        }

                        let rgba_image_data_modify = RgbaImageData {
                            width: rgba_image.width() as u16,
                            height: rgba_image.height() as u16,
                            data: rgba_image.into_raw(),
                        };

                        let offset = motion.sprite_clips[pos].position.map(|component| component);
                        let mirror = motion.sprite_clips[pos].mirror_on != 0;

                        let has_attach_point = match motion.attach_point_count {
                            Some(value) => value == 1,
                            None => false,
                        };
                        let mut attach_point = Vector2::<i32>::zero();

                        if has_attach_point {
                            attach_point = motion.attach_points[0].position;
                        }
                        let attach_point_parent = Vector2::<i32>::zero();

                        let sprite_type = match animation_index {
                            0 => SpriteType::Body,
                            1 => SpriteType::Head,
                            _ => SpriteType::Other,
                        };
                        frame_part = FramePart {
                            sprite_type,
                            rgba_data: rgba_image_data_modify.clone(),
                            upleft: Vector2::zero(),
                            offset,
                            attach_point,
                            has_attach_point,
                            mirror,
                            attach_point_parent,
                        };
                        motion_frames.push(frame_part);
                    }
                    if motion_frames.len() == 1 {
                        action_frames.push(motion_frames[0].clone());
                    } else {
                        let frame_part = Frame::merge_frame_part(&mut motion_frames);
                        action_frames.push(frame_part);
                    }
                }
                animation_frames.push(action_frames);
            }
            animations_list.push(animation_frames);
            animation_index += 1;
        }
        let action_size = vec[0].actions.actions.len();
        let animation_pair_size = vec.len();

        let mut animations: Vec<Animation> = Vec::new();

        for action_index in 0..action_size {
            let motion_size = vec[0].actions.actions[action_index].motions.len();
            let mut rgba_images: Vec<RgbaImageData> = Vec::new();
            let mut offsets: Vec<Vector2<i32>> = Vec::new();
            for motion_index in 0..motion_size {
                let mut generate: Vec<FramePart> = Vec::new();

                // When is a player, insert the head position
                // TODO: create a representation to map the head and body
                match entity_type {
                    EntityType::Player => {
                        animations_list[1][action_index][motion_index].attach_point_parent =
                            animations_list[0][action_index][motion_index].attach_point;
                    }
                    _ => {}
                }
                for animation_pair_index in 0..animation_pair_size {
                    if animations_list[animation_pair_index].len() <= action_index {
                        continue;
                    }
                    if animations_list[animation_pair_index][action_index].len() <= motion_index {
                        continue;
                    }
                    generate.push(animations_list[animation_pair_index][action_index][motion_index].clone());
                }
                let frame = Frame::merge_frame_part(&mut generate);

                rgba_images.push(frame.rgba_data);
                offsets.push(frame.offset);
            }

            // TODO: Remove this code after fixing the offset
            // The problem is that the offset is not in the correct proportion
            // To solve this primarly, we created images of the same size and same offset
            // This code resize the sprites in the correct size may be used later
            let mut max_width = 0;
            let mut max_height = 0;
            let mut min_offset_x = i32::MAX;
            let mut min_offset_y = i32::MAX;
            let mut max_offset_x = 0;
            let mut max_offset_y = 0;
            rgba_images.iter().zip(offsets.iter()).for_each(|(image_data, offset)| {
                max_width = max(max_width, image_data.width as i32);
                max_height = max(max_height, image_data.height as i32);
                min_offset_x = min(min_offset_x, offset.x as i32);
                min_offset_y = min(min_offset_y, offset.y as i32);
                max_offset_x = max(max_offset_x, offset.x as i32);
                max_offset_y = max(max_offset_y, offset.y as i32);
            });
            max_width += max_offset_x - min_offset_x + 2;
            max_height += max_offset_y - min_offset_y + 2;

            let rgba_images_fix_size = rgba_images.iter().zip(offsets.iter_mut()).map(|(image_data, offset)| {
                let mut rgba_new: RgbaImage = RgbaImage::new(max_width as u32, max_height as u32);
                let width = image_data.width as i32;
                let height = image_data.height as i32;

                let rgba_old: RgbaImage =
                    RgbaImage::from_raw(image_data.width as u32, image_data.height as u32, image_data.data.clone()).unwrap();

                for y in 0..height {
                    let upleft_old_y = offset.y as i32 - ((height - 1) / 2);
                    let upleft_new_y = min_offset_y - (max_height - (max_offset_y - min_offset_y + 2) - 1) / 2;
                    let new_y = y as i32 + upleft_old_y - upleft_new_y;
                    for x in 0..width {
                        let upleft_old_x = offset.x as i32 - ((width - 1) / 2);
                        let upleft_new_x = min_offset_x - (max_width - (max_offset_x - min_offset_x + 2) - 1) / 2;
                        let new_x = x as i32 + upleft_old_x - upleft_new_x;
                        let pixel: &mut Rgba<u8> = rgba_new.get_pixel_mut(new_x as u32, new_y as u32);
                        pixel.blend(rgba_old.get_pixel(x as u32, y as u32));
                    }
                }
                offset.x = min_offset_x;
                offset.y = min_offset_y;
                offset.x += max_offset_x - min_offset_x + 2;
                offset.y += max_offset_y - min_offset_y + 2;

                RgbaImageData {
                    width: rgba_new.width() as u16,
                    height: rgba_new.height() as u16,
                    data: rgba_new.into_raw(),
                }
            });

            // Create the textures using the animation loader functions.
            let label = match entity_type {
                EntityType::Player => format!("0_{}_{}_{}", entity_filename[0], entity_filename[1], action_index),
                EntityType::Monster => format!("1_{}_{}", entity_filename[0], action_index),
                EntityType::Npc => format!("2_{}_{}", entity_filename[0], action_index),
                _ => format!("3"),
            };

            let textures: Vec<Arc<Texture>> = rgba_images_fix_size
                .into_iter()
                .map(|image_data| {
                    let texture = Texture::new_with_data(
                        &self.device,
                        &self.queue,
                        &TextureDescriptor {
                            label: Some(&label),
                            size: Extent3d {
                                width: image_data.width as u32,
                                height: image_data.height as u32,
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: TextureDimension::D2,
                            format: TextureFormat::Rgba8UnormSrgb,
                            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
                            view_formats: &[],
                        },
                        &image_data.data,
                    );
                    Arc::new(texture)
                })
                .collect();
            animations.push(Animation { textures, offsets });
        }
        let animation_data = AnimationData {
            animations,
            delays: vec[0].actions.delays.clone(),
            entity_type,
        };
        let hash = match entity_type {
            EntityType::Player => format!("0_{}_{}", entity_filename[0], entity_filename[1]),
            EntityType::Monster => format!("1_{}", entity_filename[0]),
            EntityType::Npc => format!("2_{}", entity_filename[0]),
            _ => format!("3"),
        };
        self.cache.insert(hash.clone(), animation_data.clone());
        Ok(animation_data)
    }

    pub fn get(
        &mut self,
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        entity_filename: Vec<String>,
        entity_type: EntityType,
    ) -> Result<AnimationData, LoadError> {
        let hash = match entity_type {
            EntityType::Player => format!("0_{}_{}", entity_filename[0], entity_filename[1]),
            EntityType::Monster => format!("1_{}", entity_filename[0]),
            EntityType::Npc => format!("2_{}", entity_filename[0]),
            _ => format!("3"),
        };

        match self.cache.get(&hash) {
            Some(animation_data) => Ok(animation_data.clone()),
            None => self.load(sprite_loader, action_loader, entity_filename, entity_type),
        }
    }
}

#[derive(Clone)]
pub enum SpriteType {
    Head,
    Body,
    Other,
}

#[derive(Clone)]
pub struct FramePart {
    pub sprite_type: SpriteType,
    pub rgba_data: RgbaImageData,
    pub offset: Vector2<i32>,
    pub upleft: Vector2<i32>,
    pub has_attach_point: bool,
    pub attach_point: Vector2<i32>,
    pub attach_point_parent: Vector2<i32>,
    pub mirror: bool,
}

pub struct Frame {}

impl Frame {
    // This function generate image that will be overwrite in the order of the index
    // of the vector
    pub fn merge_frame_part(action_frames: &mut Vec<FramePart>) -> FramePart {
        for frame_part in action_frames.iter_mut() {
            let attach_point_offset = match &frame_part.has_attach_point {
                true => match &frame_part.sprite_type {
                    SpriteType::Head => -frame_part.attach_point + frame_part.attach_point_parent,
                    _ => Vector2::zero(),
                },
                false => Vector2::zero(),
            };

            // The origin is (0, 0)
            // Finding the half size of the image
            let half_size = Vector2::new(
                (frame_part.rgba_data.width as i32 - 1) / 2,
                (frame_part.rgba_data.height as i32 - 1) / 2,
            );

            // Adding the offset from attach point
            frame_part.offset += attach_point_offset;

            // For each retangle find the upleft corner
            frame_part.upleft = frame_part.offset - half_size;
        }
        // If there is no frame return an image
        if action_frames.is_empty() {
            return FramePart {
                sprite_type: SpriteType::Other,
                rgba_data: RgbaImageData {
                    width: 1,
                    height: 1,
                    data: vec![0x00, 0x00, 0x00, 0x00],
                },
                upleft: Vector2::zero(),
                offset: Vector2::zero(),
                has_attach_point: false,
                attach_point: Vector2::zero(),
                attach_point_parent: Vector2::zero(),
                mirror: false,
            };
        }
        // Find the upmost and leftmost coordinates
        let upleft_x = action_frames.iter().min_by_key(|frame_part| frame_part.upleft.x).unwrap().upleft.x;
        let upleft_y = action_frames.iter().min_by_key(|frame_part| frame_part.upleft.y).unwrap().upleft.y;

        // Find the downmost and rightmost coordinates
        let frame_part_x = action_frames
            .iter()
            .max_by_key(|frame_part| frame_part.upleft.x + frame_part.rgba_data.width as i32)
            .unwrap();
        let frame_part_y = action_frames
            .iter()
            .max_by_key(|frame_part| frame_part.upleft.y + frame_part.rgba_data.height as i32)
            .unwrap();

        // Calculate the new rectangle that is formed
        let mut new_width = frame_part_x.upleft.x + frame_part_x.rgba_data.width as i32;
        let mut new_height = frame_part_y.upleft.y + frame_part_y.rgba_data.height as i32;
        new_width -= upleft_x;
        new_height -= upleft_y;

        // Create a RgbaImage of the drawing
        let mut rgba: RgbaImage = RgbaImage::new(new_width as u32, new_height as u32);

        // Transform from RgbaImageData to RgbaImage
        let mut rgba_list: Vec<RgbaImage> = Vec::new();
        for index in 0..action_frames.len() {
            let temp: RgbaImage = RgbaImage::from_raw(
                action_frames[index].rgba_data.width.into(),
                action_frames[index].rgba_data.height.into(),
                action_frames[index].rgba_data.data.clone(),
            )
            .unwrap();
            rgba_list.push(temp);
        }

        // Insert the images in the new ImageBuffer
        // The order of for is important for cache
        for index in 0..rgba_list.len() {
            let height = rgba_list[index].height();
            let width = rgba_list[index].width();
            for y in 0..height {
                let new_y = y as i32 + action_frames[index].upleft.y - upleft_y;
                for x in 0..width {
                    let new_x = x as i32 + action_frames[index].upleft.x - upleft_x;
                    let mut change_x = x as i32;
                    if action_frames[index].mirror {
                        change_x = width as i32 - 1 - x as i32;
                    }
                    let pixel: &mut Rgba<u8> = rgba.get_pixel_mut(new_x as u32, new_y as u32);
                    pixel.blend(rgba_list[index].get_pixel(change_x as u32, y));
                }
            }
        }

        /* As the origin is (0,0), you get the upleft point of the retangle and
         * shift to the center, the offset is the difference between the origin and
         * this point */
        FramePart {
            sprite_type: SpriteType::Other,
            upleft: Vector2::zero(),
            offset: Vector2::new(
                upleft_x + (rgba.width() as i32 - 1) / 2,
                upleft_y + (rgba.height() as i32 - 1) / 2,
            ),
            rgba_data: RgbaImageData {
                width: rgba.width() as u16,
                height: rgba.height() as u16,
                data: rgba.into_raw(),
            },
            has_attach_point: false,
            attach_point: Vector2::zero(),
            attach_point_parent: Vector2::zero(),
            mirror: false,
        }
    }
}

#[derive(Clone, PrototypeElement)]
pub struct Animation {
    #[hidden_element]
    pub textures: Vec<Arc<Texture>>,
    pub offsets: Vec<Vector2<i32>>,
}

#[derive(PrototypeElement)]
pub struct AnimationPair {
    pub sprites: Arc<Sprite>,
    pub actions: Arc<Actions>,
}

#[derive(Clone, PrototypeElement)]
pub struct AnimationData {
    pub animations: Vec<Animation>,
    pub delays: Vec<f32>,
    #[hidden_element]
    pub entity_type: EntityType,
}

impl AnimationData {
    pub fn render(
        &self,
        animation_state: &AnimationState,
        camera_direction: usize,
        head_direction: usize,
    ) -> (Arc<Texture>, Vector2<f32>, bool) {
        let direction = (camera_direction + head_direction) % 8;
        let aa = animation_state.action * 8 + direction;
        let delay = self.delays[aa % self.delays.len()];
        let animation = &self.animations[aa % self.animations.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame = animation_state
            .duration
            .map(|duration| animation_state.time * animation.textures.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);
        // TODO: work out how to avoid losing digits when casting timg to an f32. When
        // fixed remove set_start_time in MouseCursor.
        let time = frame as usize % animation.textures.len();
        let texture;
        let position;
        // Remove Doridori animation from Player
        if self.entity_type == EntityType::Player && animation_state.action == 0 {
            texture = animation.textures[0].clone();
            let texture_size = texture.get_size();
            position = Vector2::new(
                animation.offsets[0].x as f32,
                animation.offsets[0].y as f32 + texture_size.height as f32 / 2.0,
            ) / 10.0;
        } else {
            texture = animation.textures[time].clone();
            let texture_size = texture.get_size();
            position = Vector2::new(
                animation.offsets[time].x as f32,
                animation.offsets[time].y as f32 + texture_size.height as f32 / 2.0,
            ) / 10.0;
        }

        (texture, position, false)
    }
}
