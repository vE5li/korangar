use std::fmt::{Display, Formatter};
#[cfg(feature = "debug")]
use std::num::NonZeroU32;

use korangar_interface::components::drop_down::DropDownItem;
use korangar_interface::element::StateElement;
use serde::{Deserialize, Serialize};

use crate::graphics::ScreenSize;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum LimitFramerate {
    Unlimited,
    Limit(u16),
}

impl DropDownItem<LimitFramerate> for LimitFramerate {
    fn text(&self) -> &str {
        match self {
            LimitFramerate::Unlimited => "Unlimited",
            LimitFramerate::Limit(30) => "30 Hz",
            LimitFramerate::Limit(60) => "60 Hz",
            LimitFramerate::Limit(120) => "120 Hz",
            LimitFramerate::Limit(144) => "144 Hz",
            LimitFramerate::Limit(240) => "240 Hz",
            LimitFramerate::Limit(_) => unimplemented!(),
        }
    }

    fn value(&self) -> LimitFramerate {
        *self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum TextureSamplerType {
    Nearest,
    Linear,
    Anisotropic(u16),
}

impl DropDownItem<TextureSamplerType> for TextureSamplerType {
    fn text(&self) -> &str {
        match self {
            TextureSamplerType::Nearest => "Nearest",
            TextureSamplerType::Linear => "Linear",
            TextureSamplerType::Anisotropic(4) => "Anisotropic x4",
            TextureSamplerType::Anisotropic(8) => "Anisotropic x8",
            TextureSamplerType::Anisotropic(16) => "Anisotropic x16",
            TextureSamplerType::Anisotropic(_) => unimplemented!(),
        }
    }

    fn value(&self) -> TextureSamplerType {
        *self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowResolution {
    Normal,
    Ultra,
    Insane,
}

impl ShadowResolution {
    pub fn directional_shadow_resolution(self) -> u32 {
        match self {
            ShadowResolution::Normal => 2048,
            ShadowResolution::Ultra => 3072,
            ShadowResolution::Insane => 4096,
        }
    }

    pub fn point_shadow_resolution(self) -> u32 {
        match self {
            ShadowResolution::Normal => 128,
            ShadowResolution::Ultra => 256,
            ShadowResolution::Insane => 512,
        }
    }
}

impl DropDownItem<ShadowResolution> for ShadowResolution {
    fn text(&self) -> &str {
        match self {
            Self::Normal => "Normal",
            Self::Ultra => "Ultra",
            Self::Insane => "Insane",
        }
    }

    fn value(&self) -> ShadowResolution {
        *self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowDetail {
    Low,
    Medium,
    High,
    Ultra,
}

impl DropDownItem<ShadowDetail> for ShadowDetail {
    fn text(&self) -> &str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    fn value(&self) -> ShadowDetail {
        *self
    }
}

impl From<ShadowDetail> for u32 {
    fn from(value: ShadowDetail) -> Self {
        match value {
            ShadowDetail::Low => 1,
            ShadowDetail::Medium => 2,
            ShadowDetail::High => 3,
            ShadowDetail::Ultra => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, StateElement)]
pub enum ShadowMethod {
    Hard,
    SoftPCF,
    SoftPCSS,
}

impl DropDownItem<ShadowMethod> for ShadowMethod {
    fn text(&self) -> &str {
        match self {
            Self::Hard => "Hard",
            Self::SoftPCF => "Soft (PCF)",
            Self::SoftPCSS => "Soft (PCSS)",
        }
    }

    fn value(&self) -> ShadowMethod {
        *self
    }
}

impl From<ShadowMethod> for u32 {
    fn from(value: ShadowMethod) -> Self {
        match value {
            ShadowMethod::Hard => 0,
            ShadowMethod::SoftPCF => 1,
            ShadowMethod::SoftPCSS => 2,
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

impl DropDownItem<Msaa> for Msaa {
    fn text(&self) -> &str {
        match self {
            Msaa::Off => "Off",
            Msaa::X2 => "x2",
            Msaa::X4 => "x4",
            Msaa::X8 => "x8",
            Msaa::X16 => "x16",
        }
    }

    fn value(&self) -> Msaa {
        *self
    }
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

impl DropDownItem<Ssaa> for Ssaa {
    fn text(&self) -> &str {
        match self {
            Ssaa::Off => "Off",
            Ssaa::X2 => "x2",
            Ssaa::X3 => "x3",
            Ssaa::X4 => "x4",
        }
    }

    fn value(&self) -> Ssaa {
        *self
    }
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
}

impl DropDownItem<ScreenSpaceAntiAliasing> for ScreenSpaceAntiAliasing {
    fn text(&self) -> &str {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off",
            ScreenSpaceAntiAliasing::Fxaa => "FXAA",
        }
    }

    fn value(&self) -> ScreenSpaceAntiAliasing {
        *self
    }
}

impl Display for ScreenSpaceAntiAliasing {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenSpaceAntiAliasing::Off => "Off".fmt(f),
            ScreenSpaceAntiAliasing::Fxaa => "FXAA".fmt(f),
        }
    }
}

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Default, rust_state::RustState, StateElement)]
pub struct RenderOptions {
    pub show_frames_per_second: bool,
    pub frustum_culling: bool,
    pub show_bounding_boxes: bool,
    pub show_map: bool,
    pub show_objects: bool,
    pub show_entities: bool,
    pub show_entities_paper: bool,
    pub show_entities_debug: bool,
    pub show_water: bool,
    pub show_indicators: bool,
    pub enable_ambient_lighting: bool,
    pub enable_directional_lighting: bool,
    pub enable_point_lights: bool,
    pub enable_particle_lighting: bool,
    pub use_debug_camera: bool,
    pub show_wireframe: bool,
    pub show_object_markers: bool,
    pub show_light_markers: bool,
    pub show_sound_markers: bool,
    pub show_effect_markers: bool,
    pub show_particle_markers: bool,
    pub show_entity_markers: bool,
    pub show_shadow_markers: bool,
    pub show_map_tiles: bool,
    pub show_pathing: bool,
    pub show_picker_buffer: bool,
    pub show_directional_shadow_map: Option<NonZeroU32>,
    pub show_point_shadow_map: Option<NonZeroU32>,
    pub show_light_culling_count_buffer: bool,
    pub show_font_map: bool,
    pub show_sdsm_partitions: bool,
    pub show_rectangle_instructions: bool,
    pub show_glyph_instructions: bool,
    pub show_sprite_instructions: bool,
    pub show_sdf_instructions: bool,
}

#[cfg(feature = "debug")]
impl RenderOptions {
    pub fn new() -> Self {
        Self {
            show_frames_per_second: false,
            frustum_culling: true,
            show_bounding_boxes: false,
            show_map: true,
            show_objects: true,
            show_entities: true,
            show_entities_paper: false,
            show_entities_debug: false,
            show_water: true,
            show_indicators: true,
            enable_ambient_lighting: true,
            enable_directional_lighting: true,
            enable_point_lights: true,
            enable_particle_lighting: true,
            use_debug_camera: false,
            show_wireframe: false,
            show_object_markers: false,
            show_light_markers: false,
            show_sound_markers: false,
            show_effect_markers: false,
            show_particle_markers: false,
            show_entity_markers: false,
            show_shadow_markers: false,
            show_map_tiles: false,
            show_pathing: false,
            show_picker_buffer: false,
            show_directional_shadow_map: None,
            show_point_shadow_map: None,
            show_light_culling_count_buffer: false,
            show_sdsm_partitions: false,
            show_font_map: false,
            show_rectangle_instructions: false,
            show_glyph_instructions: false,
            show_sprite_instructions: false,
            show_sdf_instructions: false,
        }
    }
}

#[cfg(feature = "debug")]
impl RenderOptions {
    pub fn show_buffers(&self) -> bool {
        self.show_picker_buffer
            || self.show_directional_shadow_map.is_some()
            || self.show_point_shadow_map.is_some()
            || self.show_light_culling_count_buffer
            || self.show_sdsm_partitions
            || self.show_font_map
    }
}
