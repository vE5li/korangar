use entities::{ Object, LightSource, SoundSource, EffectSource };
use system::DisplaySettings;
use graphics::{ Renderer, Camera, VertexBuffer, Texture, Transform, Color };

pub struct Map {
    ground_vertex_buffer: VertexBuffer,
    ground_textures: Vec<Texture>,
    objects: Vec<Object>,
    light_sources: Vec<LightSource>,
    sound_sources: Vec<SoundSource>,
    effect_sources: Vec<EffectSource>,
}

impl Map {

    pub fn new(ground_vertex_buffer: VertexBuffer, ground_textures: Vec<Texture>, objects: Vec<Object>, light_sources: Vec<LightSource>, sound_sources: Vec<SoundSource>, effect_sources: Vec<EffectSource>) -> Self {
        return Self { ground_vertex_buffer, ground_textures, objects, light_sources, sound_sources, effect_sources };
    }

    pub fn update(&self, delta_time: f32) {
        self.effect_sources.iter().for_each(|effect_source| effect_source.update(delta_time));
    }

    pub fn render_geomitry(&self, renderer: &mut Renderer, camera: &dyn Camera, display_settings: &DisplaySettings) {

        if display_settings.show_map {
            renderer.render_geomitry(camera, self.ground_vertex_buffer.clone(), &self.ground_textures, &Transform::new());
        }

        if display_settings.show_objects {
            self.objects.iter().for_each(|object| object.render_geometry(renderer, camera));
        }
    }

    pub fn render_lights(&self, renderer: &mut Renderer, camera: &dyn Camera, display_settings: &DisplaySettings) {

        if display_settings.show_ambient_light {
            renderer.ambient_light(Color::new(60, 60, 60));
        }

        if display_settings.show_directional_light {
            //renderer.directional_light(Vector3::new(0.0, -1.0, -0.7), Color::new(100, 100, 100));
        }

        if display_settings.show_point_lights {
            self.light_sources.iter().for_each(|light_source| light_source.render_lights(renderer, camera));
        }

        if display_settings.show_particle_lights {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_lights(renderer, camera));
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_markers(&self, renderer: &mut Renderer, camera: &dyn Camera, display_settings: &DisplaySettings) {

        if display_settings.show_object_markers {
            self.objects.iter().for_each(|object| object.render_marker(renderer, camera));
        }

        if display_settings.show_object_markers {
            self.light_sources.iter().for_each(|light_source| light_source.render_marker(renderer, camera));
        }

        if display_settings.show_object_markers {
            self.sound_sources.iter().for_each(|sound_source| sound_source.render_marker(renderer, camera));
        }

        if display_settings.show_object_markers {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_marker(renderer, camera));
        }

        if display_settings.show_object_markers {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_particle_markers(renderer, camera));
        }
    }
}
