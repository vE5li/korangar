use derive_new::new;
use procedural::toggle;

#[derive(toggle, new)]
pub struct RenderSettings {
    #[toggle]
    #[new(value = "true")]
    pub frame_limit: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_frames_per_second: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_map: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_objects: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_entities: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_water: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_interface: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_ambient_light: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_directional_light: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_point_lights: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_particle_lights: bool,
    #[toggle]
    #[new(value = "true")]
    pub show_directional_shadows: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub use_debug_camera: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_wireframe: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_object_markers: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_light_markers: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_sound_markers: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_effect_markers: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_particle_markers: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_entity_markers: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_map_tiles: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_pathing: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_diffuse_buffer: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_normal_buffer: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_water_buffer: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_depth_buffer: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_shadow_buffer: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_picker_buffer: bool,
    #[toggle]
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_font_atlas: bool,
}

#[cfg(feature = "debug")]
impl RenderSettings {

    pub fn show_buffers(&self) -> bool {

        self.show_diffuse_buffer
            || self.show_normal_buffer
            || self.show_water_buffer
            || self.show_depth_buffer
            || self.show_shadow_buffer
            || self.show_picker_buffer
            || self.show_font_atlas
    }
}
