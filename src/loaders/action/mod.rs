use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use cgmath::Vector2;
use derive_new::new;
use procedural::*;

use super::Sprite;
#[cfg(feature = "debug")]
use crate::debug::*;
use crate::graphics::Texture;
use crate::loaders::{ByteConvertable, ByteStream, GameFileLoader, Version};

//pub enum Animations {
//}

pub struct AnimationState {
    pub action: usize,
}

#[derive(PrototypeElement)]
pub struct Actions {
    actions: Vec<Action>,
    delays: Vec<f32>,
}

impl Actions {

    /*pub fn update(&mut self, client_tick: u32) {

        let mut time = client_tick - self.start_time;

        if self.duration > 0 && time > self.duration {

            self.action = self.next_action;
            self.start_time = client_tick;
            self.duration = 0;

            time = 0;
        }

        self.time = time;
    }*/

    pub fn render(&self, sprite: &Sprite, animation_state: &AnimationState, camera_direction: usize) -> (Texture, bool) {

        let direction = camera_direction % 8;
        let aa = animation_state.action * 8 + direction;
        let a = &self.actions[aa % self.actions.len()];
        let fs = &a.motions[0];

        (
            sprite.textures[fs.sprite_clips[0].sprite_number as usize].clone(),
            fs.sprite_clips[0].mirror_on != 0,
        )

        /*let direction = 0;
        let camera_direction = 0;
        let fdir = (direction + camera_direction) % 8;

        let aa = self.action * 8 + fdir;
        let a = &self.actions[aa % self.actions.len()];
        let delay = self.delays[aa % self.delays.len()];

        let factor = match self.factor {
            0 => delay as usize,
            factor => delay as usize * factor as usize / 100,
        };

        let frame = match self.duration > 0 {
            true => self.time as usize * a.motions.len() / self.duration as usize,
            false => self.time as usize / factor,
        };

        let fs = &a.motions[frame % a.motions.len()];

        sprite.textures[fs.sprite_clips[0].sprite_number as usize].clone()*/
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
    pub x_zoom: Option<f32>,
    #[version_equals_or_above(2, 4)]
    pub y_zoom: Option<f32>,
    #[version_equals_or_above(2, 0)]
    pub angle: Option<i32>,
    #[version_equals_or_above(2, 0)]
    pub sprite_type: Option<u32>,
    #[version_equals_or_above(2, 5)]
    pub width: Option<u32>,
    #[version_equals_or_above(2, 5)]
    pub height: Option<u32>,
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

#[derive(new)]
pub struct ActionLoader {
    game_file_loader: Rc<RefCell<GameFileLoader>>,
    #[new(default)]
    cache: HashMap<String, Arc<Actions>>,
}

impl ActionLoader {

    fn load(&mut self, path: &str) -> Result<Arc<Actions>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load actions from {}{}{}", MAGENTA, path, NONE));

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\sprite\\{}", path))?;
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

    pub fn get(&mut self, path: &str) -> Result<Arc<Actions>, String> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path),
        }
    }
}
