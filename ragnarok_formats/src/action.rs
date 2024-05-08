use cgmath::Vector2;
use ragnarok_bytes::{ByteConvertable, FromBytes};

use crate::signature::Signature;
use crate::version::{MinorFirst, Version};

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SpriteClip {
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

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct AttachPoint {
    pub ignored: u32,
    pub position: Vector2<i32>,
    pub attribute: u32,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Motion {
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

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Action {
    pub motion_count: u32,
    #[repeating(self.motion_count)]
    pub motions: Vec<Motion>,
}

#[derive(Debug, Clone, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Event {
    #[length_hint(40)]
    pub name: String,
}

#[derive(Debug, Clone, FromBytes)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ActionsData {
    pub signature: Signature<b"AC">,
    #[version]
    pub version: Version<MinorFirst>,
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
