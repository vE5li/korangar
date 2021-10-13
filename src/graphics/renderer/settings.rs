pub struct RenderSettings {
    pub show_frames_per_second: bool,
    pub show_map: bool,
    pub show_objects: bool,
    pub show_ambient_light: bool,
    pub show_directional_light: bool,
    pub show_point_lights: bool,
    pub show_particle_lights: bool,
    #[cfg(feature = "debug")]
    pub use_debug_camera: bool,
    #[cfg(feature = "debug")]
    pub show_object_markers: bool,
    #[cfg(feature = "debug")]
    pub show_light_markers: bool,
    #[cfg(feature = "debug")]
    pub show_sound_markers: bool,
    #[cfg(feature = "debug")]
    pub show_effect_markers: bool,
    #[cfg(feature = "debug")]
    pub show_particle_markers: bool,
    #[cfg(feature = "debug")]
    pub show_map_tiles: bool,
}

impl RenderSettings {

    pub fn new() -> Self {

        let show_frames_per_second = true;
        let show_map = true;
        let show_objects = true;
        let show_ambient_light = true;
        let show_directional_light = true;
        let show_point_lights = true;
        let show_particle_lights = true;
        #[cfg(feature = "debug")]
        let use_debug_camera = false;
        #[cfg(feature = "debug")]
        let show_object_markers = false;
        #[cfg(feature = "debug")]
        let show_light_markers = false;
        #[cfg(feature = "debug")]
        let show_sound_markers = false;
        #[cfg(feature = "debug")]
        let show_effect_markers = false;
        #[cfg(feature = "debug")]
        let show_particle_markers = false;
        #[cfg(feature = "debug")]
        let show_map_tiles = false;

        return Self {
            show_frames_per_second,
            show_map,
            show_objects,
            show_ambient_light,
            show_directional_light,
            show_point_lights,
            show_particle_lights,
            #[cfg(feature = "debug")]
            use_debug_camera,
            #[cfg(feature = "debug")]
            show_object_markers,
            #[cfg(feature = "debug")]
            show_light_markers,
            #[cfg(feature = "debug")]
            show_sound_markers,
            #[cfg(feature = "debug")]
            show_effect_markers,
            #[cfg(feature = "debug")]
            show_particle_markers,
            #[cfg(feature = "debug")]
            show_map_tiles,
        };
    }

    pub fn toggle_show_frames_per_second(&mut self) {
        self.show_frames_per_second = !self.show_frames_per_second;
    }

    pub fn toggle_show_map(&mut self) {
        self.show_map = !self.show_map;
    }

    pub fn toggle_show_objects(&mut self) {
        self.show_objects = !self.show_objects;
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
}
