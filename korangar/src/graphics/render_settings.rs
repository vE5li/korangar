use std::num::NonZeroU32;

use derive_new::new;

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
    pub show_diffuse_buffer: bool,
    #[new(default)]
    pub show_normal_buffer: bool,
    #[new(default)]
    pub show_water_buffer: bool,
    #[new(default)]
    pub show_depth_buffer: bool,
    #[new(default)]
    pub show_shadow_buffer: bool,
    #[new(default)]
    pub show_picker_buffer: bool,
    #[new(default)]
    pub show_font_atlas: bool,
    #[new(default)]
    pub show_point_shadow: Option<NonZeroU32>,
}

impl RenderSettings {
    pub fn show_buffers(&self) -> bool {
        self.show_diffuse_buffer
            || self.show_normal_buffer
            || self.show_water_buffer
            || self.show_depth_buffer
            || self.show_shadow_buffer
            || self.show_picker_buffer
            || self.show_font_atlas
            || self.show_point_shadow.is_some()
    }
}
