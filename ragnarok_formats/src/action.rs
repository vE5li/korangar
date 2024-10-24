use cgmath::Vector2;
use ragnarok_bytes::ByteConvertable;

use crate::signature::Signature;
use crate::version::{MinorFirst, Version};

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct SpriteClip {
    pub position: Vector2<i32>,
    pub sprite_number: i32,
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
    #[new_derive]
    pub sprite_clip_count: u32,
    #[repeating(sprite_clip_count)]
    pub sprite_clips: Vec<SpriteClip>,
    #[version_equals_or_above(2, 0)]
    pub event_id: Option<i32>, // if version == 2.0 this maybe needs to be set to None ?
    // (after it is parsed)
    #[version_equals_or_above(2, 3)]
    pub attach_point_count: Option<u32>,
    #[repeating_option(attach_point_count)]
    pub attach_points: Vec<AttachPoint>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Action {
    #[new_derive]
    pub motion_count: u32,
    #[repeating(motion_count)]
    pub motions: Vec<Motion>,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct Event {
    #[length(40)]
    pub name: String,
}

#[derive(Debug, Clone, ByteConvertable)]
#[cfg_attr(feature = "interface", derive(korangar_interface::elements::PrototypeElement))]
pub struct ActionsData {
    #[new_default]
    pub signature: Signature<b"AC">,
    #[version]
    pub version: Version<MinorFirst>,
    // #[new_derive]
    pub action_count: u16,
    #[new_default]
    pub reserved: [u8; 10],
    #[repeating(action_count)]
    pub actions: Vec<Action>,
    #[version_equals_or_above(2, 1)]
    #[new_derive]
    pub event_count: Option<u32>,
    #[repeating_option(event_count)]
    pub events: Vec<Event>,
    #[version_equals_or_above(2, 2)]
    #[repeating(action_count)]
    pub delays: Option<Vec<f32>>,
}
