use std::collections::HashMap;
use std::ops::Mul;
use std::sync::Arc;

use cgmath::{Array, Vector2};
use procedural::*;
use vulkano::image::ImageAccess;

use super::Sprite;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::{Color, DeferredRenderer, Renderer, Texture};
use crate::interface::InterfaceSettings;
use crate::loaders::{ByteConvertable, ByteStream, GameFileLoader, Version};

//pub enum Animations {
//}

pub struct AnimationState {
    pub action: usize,
    pub start_time: u32,
    pub time: u32,
    pub duration: Option<u32>,
    pub factor: Option<f32>,
}

impl AnimationState {
    pub fn new(client_tick: u32) -> Self {
        Self {
            action: 0,
            start_time: client_tick,
            time: 0,
            duration: None,
            factor: None,
        }
    }

    pub fn idle(&mut self, client_tick: u32) {
        self.action = 0;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = None;
    }

    pub fn walk(&mut self, movement_speed: usize, client_tick: u32) {
        self.action = 1;
        self.start_time = client_tick;
        self.duration = None;
        self.factor = Some(movement_speed as f32 * 100.0 / 150.0);
    }

    pub fn update(&mut self, client_tick: u32) {
        let mut time = client_tick - self.start_time;

        // TODO: make everything have a duration so that we can update the start_time
        // from time to time so that animations won't start to drop frames as
        // soon as start_time - client_tick can no longer be stored in an f32
        // accurately. When fixed remove set_start_time in MouseCursor.
        if let Some(duration) = self.duration && time > duration {

            //self.action = self.next_action;
            self.start_time = client_tick;
            self.duration = None;

            time = 0;
        }

        self.time = time;
    }
}

#[derive(PrototypeElement)]
pub struct Actions {
    actions: Vec<Action>,
    delays: Vec<f32>,
}

impl Actions {
    pub fn render(
        &self,
        sprite: &Sprite,
        animation_state: &AnimationState,
        camera_direction: usize,
        head_direction: usize,
    ) -> (Texture, Vector2<f32>, bool) {
        let direction = (camera_direction + head_direction) % 8;
        let aa = animation_state.action * 8 + direction;
        let a = &self.actions[aa % self.actions.len()];
        let delay = self.delays[aa % self.delays.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame = animation_state
            .duration
            .map(|duration| animation_state.time * a.motions.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);
        // TODO: work out how to avoid losing digits when casting timg to an f32. When
        // fixed remove set_start_time in MouseCursor.

        let fs = &a.motions[frame as usize % a.motions.len()];

        let texture = sprite.textures[fs.sprite_clips[0].sprite_number as usize].clone();
        let texture_size = texture.image().dimensions().width_height().map(|component| component as f32);
        let offset = fs.sprite_clips[0].position.map(|component| component as f32);

        (
            texture,
            Vector2::new(-offset.x, offset.y + texture_size[1] / 2.0) / 10.0,
            fs.sprite_clips[0].mirror_on != 0,
        )
    }

    pub fn render2(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        sprite: &Sprite,
        animation_state: &AnimationState,
        position: Vector2<f32>,
        camera_direction: usize,
        color: Color,
        interface_settings: &InterfaceSettings,
    ) {
        let direction = camera_direction % 8;
        let aa = animation_state.action * 8 + direction;
        let a = &self.actions[aa % self.actions.len()];
        let delay = self.delays[aa % self.delays.len()];

        let factor = animation_state
            .factor
            .map(|factor| delay * (factor / 5.0))
            .unwrap_or_else(|| delay * 50.0);

        let frame = animation_state
            .duration
            .map(|duration| animation_state.time * a.motions.len() as u32 / duration)
            .unwrap_or_else(|| (animation_state.time as f32 / factor) as u32);
        // TODO: work out how to avoid losing digits when casting timg to an f32. When
        // fixed remove set_start_time in MouseCursor.

        let fs = &a.motions[frame as usize % a.motions.len()];

        for sprite_clip in &fs.sprite_clips {
            let texture = &sprite.textures[sprite_clip.sprite_number as usize];
            let offset = sprite_clip.position.map(|component| component as f32);
            let dimesions = sprite_clip
                .size
                .unwrap_or_else(|| texture.image().dimensions().width_height().into())
                .map(|component| component as f32);
            let zoom = sprite_clip.zoom.unwrap_or(1.0) * *interface_settings.scaling;
            let zoom2 = sprite_clip.zoom2.unwrap_or_else(|| Vector2::from_value(1.0));

            let final_size = dimesions.zip(zoom2, f32::mul) * zoom;
            let final_position = position + offset - final_size / 2.0;

            renderer.render_sprite(render_target, texture.clone(), final_position, final_size, color);
        }
    }
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
struct SpriteClip {
    pub position: Vector2<i32>,
    pub sprite_number: u32,
    pub mirror_on: u32,
    #[version_equals_or_above(2, 0)]
    pub color: Option<u32>,
    #[version_smaller(2, 4)]
    pub zoom: Option<f32>,
    #[version_equals_or_above(2, 4)]
    pub zoom2: Option<Vector2<f32>>,
    #[version_equals_or_above(2, 0)]
    pub angle: Option<i32>,
    #[version_equals_or_above(2, 0)]
    pub sprite_type: Option<u32>,
    #[version_equals_or_above(2, 5)]
    pub size: Option<Vector2<u32>>,
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
struct AttachPoint {
    pub ignored: u32,
    pub position: Vector2<i32>,
    pub attribute: u32,
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
struct Motion {
    pub range1: [i32; 4], // maybe just skip this?
    pub range2: [i32; 4], // maybe just skip this?
    pub sprite_clip_count: u32,
    #[repeating(self.sprite_clip_count)]
    pub sprite_clips: Vec<SpriteClip>,
    #[version_equals_or_above(2, 0)]
    pub event_id: Option<i32>, // if version == 2.0 this maybe needs to be set to None ?
    // (after it is parsed)
    #[version_equals_or_above(2, 3)]
    pub attach_point_count: Option<u32>,
    #[repeating(self.attach_point_count.unwrap_or_default())]
    pub attach_points: Vec<AttachPoint>,
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
struct Action {
    pub motion_count: u32,
    #[repeating(self.motion_count)]
    pub motions: Vec<Motion>,
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
struct Event {
    #[length_hint(40)]
    pub name: String,
}

#[derive(Debug, ByteConvertable, PrototypeElement)]
struct ActionsData {
    #[version]
    pub version: Version,
    pub action_count: u16,
    pub reserved: [u8; 10],
    #[repeating(self.action_count)]
    pub actions: Vec<Action>,
    #[version_equals_or_above(2, 1)]
    pub event_count: Option<u32>,
    #[repeating(self.event_count.unwrap_or_default())]
    pub events: Vec<Event>,
    #[version_equals_or_above(2, 2)]
    #[repeating(self.action_count)]
    pub delays: Option<Vec<f32>>,
}

#[derive(Default)]
pub struct ActionLoader {
    cache: HashMap<String, Arc<Actions>>,
}

impl ActionLoader {
    fn load(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Actions>, String> {
        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load actions from {}{}{}", MAGENTA, path, NONE));

        let bytes = game_file_loader.get(&format!("data\\sprite\\{}", path))?;
        let mut byte_stream = ByteStream::new(&bytes);

        if byte_stream.string(2).as_str() != "AC" {
            return Err(format!("failed to read magic number from {}", path));
        }

        let actions_data = ActionsData::from_bytes(&mut byte_stream, None);

        let delays = actions_data
            .delays
            .unwrap_or_else(|| actions_data.actions.iter().map(|_| 0.0).collect());

        let sprite = Arc::new(Actions {
            actions: actions_data.actions,
            delays,
        });

        self.cache.insert(path.to_string(), sprite.clone());

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str, game_file_loader: &mut GameFileLoader) -> Result<Arc<Actions>, String> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path, game_file_loader),
        }
    }
}
