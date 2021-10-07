pub struct DisplaySettings {
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
}

impl DisplaySettings {

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
        };
    }
}
