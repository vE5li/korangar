use std::collections::HashMap;
use std::sync::Arc;

use cgmath::Vector2;
use derive_new::new;
use image::imageops::FilterType;
use image::{save_buffer, Pixel, Rgba, RgbaImage};
use korangar_interface::elements::PrototypeElement;
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

        // Get the actions for merging the sprites in one
        // Each action have a vector of framepart
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
                        let sprite_number: i32 = match motion.sprite_clips[pos].sprite_number != -1 {
                            true => motion.sprite_clips[pos].sprite_number as i32,
                            false => -1,
                        };
                        if sprite_number == -1 {
                            continue;
                        }
                        // TODO: using the information from the sprite to directly infer if is rgba or
                        // palette image
                        let rgba_image_data = match animation_pair.sprites.palette_count {
                            0 => animation_pair.sprites.rgba_images[sprite_number as usize].clone(),
                            _ => animation_pair.sprites.palette_images[sprite_number as usize].clone(),
                        };
                        let zoom = match motion.sprite_clips[pos].zoom {
                            Some(value) => value,
                            None => 1.0,
                        };

                        let rgba_image: RgbaImage = RgbaImage::from_raw(
                            rgba_image_data.width.into(),
                            rgba_image_data.height.into(),
                            rgba_image_data.data.clone(),
                        )
                        .unwrap();

                        let new_width = (rgba_image_data.width as f32 * zoom) as u32;
                        let new_height = (rgba_image_data.height as f32 * zoom) as u32;
                        let rgba_image_scale = image::imageops::resize(&rgba_image, new_width, new_height, FilterType::Lanczos3);

                        let rgba_image_data_scale = RgbaImageData {
                            width: rgba_image_scale.width() as u16,
                            height: rgba_image_scale.height() as u16,
                            data: rgba_image_scale.into_raw(),
                        };

                        let offset = motion.sprite_clips[pos].position.map(|component| component);
                        let mirror = motion.sprite_clips[pos].mirror_on != 0;

                        let has_attach_point = match motion.attach_point_count {
                            Some(value) => value == 1,
                            None => false,
                        };
                        let mut attach_point_x = 0;
                        let mut attach_point_y = 0;

                        if has_attach_point {
                            attach_point_x = motion.attach_points[0].position.x;
                            attach_point_y = motion.attach_points[0].position.y;
                        }
                        let attach_point_parent_x = 0;
                        let attach_point_parent_y = 0;

                        let sprite_type = match animation_index {
                            0 => SpriteType::Body,
                            1 => SpriteType::Head,
                            _ => SpriteType::Other,
                        };
                        frame_part = FramePart {
                            sprite_type,
                            rgba_data: rgba_image_data_scale.clone(),
                            offset_x: offset.x,
                            offset_y: offset.y,
                            attach_point_x,
                            attach_point_y,
                            has_attach_point,
                            mirror,
                            attach_point_parent_x,
                            attach_point_parent_y,
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
            let mut offsets: Vec<Vector2<f32>> = Vec::new();
            for motion_index in 0..motion_size {
                let mut generate: Vec<FramePart> = Vec::new();

                // When is a player, insert the head position
                // TODO: create a representation to map the head and body
                match entity_type {
                    EntityType::Player => {
                        animations_list[1][action_index][motion_index].attach_point_parent_x =
                            animations_list[0][action_index][motion_index].attach_point_x;
                        animations_list[1][action_index][motion_index].attach_point_parent_y =
                            animations_list[0][action_index][motion_index].attach_point_y;
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
                offsets.push(Vector2::new(frame.offset_x as f32, frame.offset_y as f32));
            }
            // 3 - Create the textures using the animation loader functions.
            let label = match entity_type {
                EntityType::Player => format!("0_{}_{}_{}", entity_filename[0], entity_filename[1], action_index),
                EntityType::Monster => format!("1_{}_{}", entity_filename[0], action_index),
                EntityType::Npc => format!("2_{}_{}", entity_filename[0], action_index),
                _ => format!("3"),
            };
            let textures: Vec<Arc<Texture>> = rgba_images
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
        self.cache.insert(entity_filename[0].clone(), animation_data.clone());
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
    pub offset_x: i32,
    pub offset_y: i32,
    pub has_attach_point: bool,
    pub attach_point_x: i32,
    pub attach_point_y: i32,
    pub attach_point_parent_x: i32,
    pub attach_point_parent_y: i32,
    pub mirror: bool,
}

pub struct Frame {
    pub texture: Arc<Texture>,
}

impl Frame {
    // The generate image will be overwrite in the order of the index of the vector
    pub fn merge_frame_part(action_frames: &mut Vec<FramePart>) -> FramePart {
        // Adjusting the values
        for frame_part in action_frames.iter_mut() {
            // A small offset when there is mirror image
            let mirror_offset = match frame_part.mirror {
                true => -1,
                false => 1,
            };
            let attach_point_offset_x = match &frame_part.has_attach_point {
                true => match &frame_part.sprite_type {
                    SpriteType::Head => -frame_part.attach_point_x + frame_part.attach_point_parent_x,
                    _ => 0,
                },
                false => 0,
            };
            let attach_point_offset_y = match &frame_part.has_attach_point {
                true => match &frame_part.sprite_type {
                    SpriteType::Head => -frame_part.attach_point_y + frame_part.attach_point_parent_y,
                    _ => 0,
                },
                false => 0,
            };
            // Correcting the mirror offset of the center of image
            let center_image_x: i32 = (frame_part.rgba_data.width as i32 + mirror_offset) / 2;
            let center_image_y: i32 = (frame_part.rgba_data.height as i32 + mirror_offset) / 2;

            // Correcting the origin from the center of image to the left upper corner of
            // image
            frame_part.offset_x = frame_part.offset_x - center_image_x + attach_point_offset_x;
            frame_part.offset_y = frame_part.offset_y - center_image_y + attach_point_offset_y;
        }
        if action_frames.is_empty() {
            return FramePart {
                sprite_type: SpriteType::Other,
                rgba_data: RgbaImageData {
                    width: 1,
                    height: 1,
                    data: vec![0x00, 0x00, 0x00, 0x00],
                },
                offset_x: 0,
                offset_y: 0,
                has_attach_point: false,
                attach_point_x: 0,
                attach_point_y: 0,
                attach_point_parent_x: 0,
                attach_point_parent_y: 0,
                mirror: false,
            };
        }
        // Get the minimal offset to find the new pixel (0, 0)
        let offset_x = action_frames.iter().min_by_key(|frame_part| frame_part.offset_x).unwrap().offset_x;
        let offset_y = action_frames.iter().min_by_key(|frame_part| frame_part.offset_y).unwrap().offset_y;

        // The new size of the rgba
        let frame_part_x = action_frames
            .iter()
            .max_by_key(|frame_part| frame_part.offset_x + frame_part.rgba_data.width as i32)
            .unwrap();
        let frame_part_y = action_frames
            .iter()
            .max_by_key(|frame_part| frame_part.offset_y + frame_part.rgba_data.height as i32)
            .unwrap();

        let mut new_width = frame_part_x.offset_x + frame_part_x.rgba_data.width as i32;
        let mut new_height = frame_part_y.offset_y + frame_part_y.rgba_data.height as i32;
        new_width -= offset_x;
        new_height -= offset_y;

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
                let new_y = (y as i32) + action_frames[index].offset_y - offset_y;
                for x in 0..width {
                    let new_x = x as i32 + action_frames[index].offset_x - offset_x;
                    let mut change_x = x as i32;
                    if action_frames[index].mirror {
                        change_x = width as i32 - 1 - x as i32;
                    }
                    let pixel: &mut Rgba<u8> = rgba.get_pixel_mut(new_x as u32, new_y as u32);
                    pixel.blend(rgba_list[index].get_pixel(change_x as u32, y));
                    //rgba.put_pixel(new_x as u32, new_y as u32, pixel);
                }
            }
        }
        FramePart {
            sprite_type: SpriteType::Other,
            offset_x: offset_x + rgba.width() as i32 / 2,  // Convert back to the center
            offset_y: offset_y + rgba.height() as i32 / 2, // Convert back to the center
            rgba_data: RgbaImageData {
                width: rgba.width() as u16,
                height: rgba.height() as u16,
                data: rgba.into_raw(),
            },

            has_attach_point: false,
            attach_point_x: 0,
            attach_point_y: 0,
            attach_point_parent_x: 0,
            attach_point_parent_y: 0,
            mirror: false,
        }
    }

    #[cfg(feature = "debug")]
    pub fn image_save(image_new: RgbaImageData, sprite_number: i32) {
        save_buffer(
            format!("image_{}.png", sprite_number),
            &image_new.data,
            image_new.width.into(),
            image_new.height.into(),
            image::ExtendedColorType::Rgba8,
        )
        .unwrap();
    }
}

#[derive(Clone, PrototypeElement)]
pub struct Animation {
    #[hidden_element]
    pub textures: Vec<Arc<Texture>>, // The vector of frames generated from animation pair
    pub offsets: Vec<Vector2<f32>>,
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
                animation.offsets[0].x,
                animation.offsets[0].y + texture_size.height as f32 / 2.0,
            ) / 10.0;
        } else {
            texture = animation.textures[time].clone();
            let texture_size = texture.get_size();
            position = Vector2::new(
                animation.offsets[time].x,
                animation.offsets[time].y + texture_size.height as f32 / 2.0,
            ) / 10.0;
        }

        (texture, position, false)
    }
}
