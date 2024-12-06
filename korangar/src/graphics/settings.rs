use std::fmt::{Display, Formatter};
#[cfg(feature = "debug")]
use std::num::NonZeroU32;

#[cfg(feature = "debug")]
use derive_new::new;
use serde::{Deserialize, Serialize};

use crate::interface::layout::ScreenSize;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LimitFramerate {
    Unlimited,
    Limit(u16),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextureSamplerType {
    Nearest,
    Linear,
    Anisotropic(u16),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShadowDetail {
    Low,
    Medium,
    High,
    Ultra,
}

impl ShadowDetail {
    pub fn directional_shadow_resolution(self) -> u32 {
        match self {
            ShadowDetail::Low => 512,
            ShadowDetail::Medium => 1024,
            ShadowDetail::High => 2048,
            ShadowDetail::Ultra => 8192,
        }
    }

    pub fn point_shadow_resolution(self) -> u32 {
        match self {
            ShadowDetail::Low => 64,
            ShadowDetail::Medium => 128,
            ShadowDetail::High => 256,
            ShadowDetail::Ultra => 512,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Msaa {
    Off,
    X2,
    X4,
    X8,
    X16,
}

impl Display for Msaa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Msaa::Off => "Off".fmt(f),
            Msaa::X2 => "x2".fmt(f),
            Msaa::X4 => "x4".fmt(f),
            Msaa::X8 => "x8".fmt(f),
            Msaa::X16 => "x16".fmt(f),
        }
    }
}

impl From<u32> for Msaa {
    fn from(value: u32) -> Self {
        match value {
            1 => Msaa::Off,
            2 => Msaa::X2,
            4 => Msaa::X4,
            8 => Msaa::X8,
            16 => Msaa::X16,
            _ => panic!("Unknown sample count"),
        }
    }
}

impl Msaa {
    pub fn sample_count(self) -> u32 {
        match self {
            Msaa::Off => 1,
            Msaa::X2 => 2,
            Msaa::X4 => 4,
            Msaa::X8 => 8,
            Msaa::X16 => 16,
        }
    }

    pub fn multisampling_activated(self) -> bool {
        self != Msaa::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Ssaa {
    Off,
    X2,
    X3,
    X4,
}

impl Display for Ssaa {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Ssaa::Off => "Off".fmt(f),
            Ssaa::X2 => "x2".fmt(f),
            Ssaa::X3 => "x3".fmt(f),
            Ssaa::X4 => "x4".fmt(f),
        }
    }
}

impl Ssaa {
    pub fn calculate_size(self, base_size: ScreenSize) -> ScreenSize {
        match self {
            Ssaa::Off => base_size,
            Ssaa::X2 => base_size * f32::sqrt(2.0),
            Ssaa::X3 => base_size * f32::sqrt(3.0),
            Ssaa::X4 => base_size * 2.0,
        }
    }

    pub fn supersampling_activated(self) -> bool {
        self != Ssaa::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ScreenSpaceAntiAliasing {
    Off,
    Fxaa,
    Cmaa2,
}

impl Display for ScreenSpaceAntiAliasing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off".fmt(f),
            ScreenSpaceAntiAliasing::Fxaa => "FXAA".fmt(f),
            ScreenSpaceAntiAliasing::Cmaa2 => "CMAA2".fmt(f),
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, new)]
pub struct RenderSettings {
    #[new(value = "true")]
    pub show_frames_per_second: bool,
    #[new(value = "true")]
    pub frustum_culling: bool,
    #[new(default)]
    pub show_bounding_boxes: bool,
    #[new(value = "true")]
    pub show_map: bool,
    #[new(value = "true")]
    pub show_objects: bool,
    #[new(value = "true")]
    pub show_entities: bool,
    #[new(value = "true")]
    pub show_water: bool,
    #[new(value = "true")]
    pub show_indicators: bool,
    #[new(value = "true")]
    pub show_ambient_light: bool,
    #[new(value = "true")]
    pub show_directional_light: bool,
    #[new(value = "true")]
    pub show_point_lights: bool,
    #[new(value = "true")]
    pub show_particle_lights: bool,
    #[new(default)]
    pub use_debug_camera: bool,
    #[new(default)]
    pub show_wireframe: bool,
    #[new(default)]
    pub show_object_markers: bool,
    #[new(default)]
    pub show_light_markers: bool,
    #[new(default)]
    pub show_sound_markers: bool,
    #[new(default)]
    pub show_effect_markers: bool,
    #[new(default)]
    pub show_particle_markers: bool,
    #[new(default)]
    pub show_entity_markers: bool,
    #[new(default)]
    pub show_shadow_markers: bool,
    #[new(default)]
    pub show_map_tiles: bool,
    #[new(default)]
    pub show_pathing: bool,
    #[new(default)]
    pub show_picker_buffer: bool,
    #[new(default)]
    pub show_directional_shadow_map: bool,
    #[new(default)]
    pub show_point_shadow_map: Option<NonZeroU32>,
    #[new(default)]
    pub show_light_culling_count_buffer: bool,
    #[new(default)]
    pub show_font_atlas: bool,
}

#[cfg(feature = "debug")]
impl RenderSettings {
    pub fn show_buffers(&self) -> bool {
        self.show_directional_shadow_map
            || self.show_picker_buffer
            || self.show_point_shadow_map.is_some()
            || self.show_light_culling_count_buffer
            || self.show_font_atlas
    }
}
