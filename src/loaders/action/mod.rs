use derive_new::new;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[cfg(feature = "debug")]
use crate::debug::*;
use crate::types::ByteStream;
use crate::traits::ByteConvertable;
use crate::loaders::GameFileLoader;
use crate::types::Version;

#[derive(Clone)]
pub struct Actions {}

#[derive(Debug, ByteConvertable)]
struct SpriteClip {
    pub x: i32,
    pub y: i32,
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

#[derive(Debug, ByteConvertable)]
struct AttachPoint {
    pub ignored: u32,
    pub x: i32,
    pub y: i32,
    pub attribute: u32,
}

#[derive(Debug, ByteConvertable)]
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

#[derive(Debug, ByteConvertable)]
struct Action {
    pub motion_count: u32,
    #[repeating(self.motion_count)]
    pub motions: Vec<Motion>,
}

#[derive(Debug, ByteConvertable)]
struct Event {
    #[length_hint(40)]
    pub name: String,
}

#[derive(Debug, ByteConvertable)]
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
    cache: HashMap<String, Rc<Actions>>,
}

impl ActionLoader {

    fn load(&mut self, path: &str) -> Result<Rc<Actions>, String> {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load actions from {}{}{}", MAGENTA, path, NONE));

        let bytes = self.game_file_loader.borrow_mut().get(&format!("data\\sprite\\{}", path))?;
        let mut byte_stream = ByteStream::new(&bytes);

        if byte_stream.string(2).as_str() != "AC" {
            return Err(format!("failed to read magic number from {}", path));
        }

        let actions_data = ActionsData::from_bytes(&mut byte_stream, None);
        //println!("{:#?}", actions_data);

        let sprite = Rc::new(Actions {});
        self.cache.insert(path.to_string(), Rc::clone(&sprite));

        #[cfg(feature = "debug")]
        timer.stop();

        Ok(sprite)
    }

    pub fn get(&mut self, path: &str) -> Result<Rc<Actions>, String> {
        match self.cache.get(path) {
            Some(sprite) => Ok(sprite.clone()),
            None => self.load(path),
        }
    }
}
