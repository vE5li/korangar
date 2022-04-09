use derive_new::new;

#[derive(new)]
pub struct RenderSettings { 
    #[new(value = "true")]
    pub show_frames_per_second: bool,
    #[new(value = "true")]
    pub show_map: bool,
    #[new(value = "true")]
    pub show_objects: bool,
    #[new(value = "true")]
    pub show_entities: bool,
    #[new(value = "true")]
    pub show_ambient_light: bool,
    #[new(value = "true")]
    pub show_directional_light: bool,
    #[new(value = "true")]
    pub show_point_lights: bool,
    #[new(value = "true")]
    pub show_particle_lights: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub use_debug_camera: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_object_markers: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_light_markers: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_sound_markers: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_effect_markers: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_particle_markers: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_map_tiles: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_pathing: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_diffuse_buffer: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_normal_buffer: bool,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub show_depth_buffer: bool,
}

impl RenderSettings {

    pub fn toggle_show_frames_per_second(&mut self) {
        self.show_frames_per_second = !self.show_frames_per_second;
    }

    pub fn toggle_show_map(&mut self) {
        self.show_map = !self.show_map;
    }

    pub fn toggle_show_objects(&mut self) {
        self.show_objects = !self.show_objects;
    }

    pub fn toggle_show_entities(&mut self) {
        self.show_entities = !self.show_entities;
    }

    pub fn toggle_show_ambient_light(&mut self) {
        self.show_ambient_light = !self.show_ambient_light;
    }

    pub fn toggle_show_directional_light(&mut self) {
        self.show_directional_light = !self.show_directional_light;
    }

    pub fn toggle_show_point_lights(&mut self) {
        self.show_point_lights = !self.show_point_lights;
    }

    pub fn toggle_show_particle_lights(&mut self) {
        self.show_particle_lights = !self.show_particle_lights;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_use_debug_camera(&mut self) {
        self.use_debug_camera = !self.use_debug_camera;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_object_markers(&mut self) {
        self.show_object_markers = !self.show_object_markers;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_light_markers(&mut self) {
        self.show_light_markers = !self.show_light_markers;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_sound_markers(&mut self) {
        self.show_sound_markers = !self.show_sound_markers;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_effect_markers(&mut self) {
        self.show_effect_markers = !self.show_effect_markers;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_particle_markers(&mut self) {
        self.show_particle_markers = !self.show_particle_markers;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_map_tiles(&mut self) {
        self.show_map_tiles = !self.show_map_tiles;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_pathing(&mut self) {
        self.show_pathing = !self.show_pathing;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_diffuse_buffer(&mut self) {
        self.show_diffuse_buffer = !self.show_diffuse_buffer;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_normal_buffer(&mut self) {
        self.show_normal_buffer = !self.show_normal_buffer;
    }

    #[cfg(feature = "debug")]
    pub fn toggle_show_depth_buffer(&mut self) {
        self.show_depth_buffer = !self.show_depth_buffer;
    }

    #[cfg(feature = "debug")]
    pub fn show_buffers(&self) -> bool {
        return self.show_diffuse_buffer || self.show_normal_buffer || self.show_depth_buffer;
    }
}
