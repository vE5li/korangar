mod object;
mod light;
mod sound;
mod effect;

use graphics::{ Renderer, RenderSettings, Camera, VertexBuffer, Texture, Transform, Color };

pub use self::object::Object;
pub use self::object::model;
pub use self::light::LightSource;
pub use self::sound::SoundSource;
pub use self::effect::EffectSource;

pub struct Map {
    ground_vertex_buffer: VertexBuffer,
    ground_textures: Vec<Texture>,
    objects: Vec<Object>,
    light_sources: Vec<LightSource>,
    sound_sources: Vec<SoundSource>,
    effect_sources: Vec<EffectSource>,
    tile_vertex_buffer: Option<VertexBuffer>, // make debug only
}

impl Map {

    pub fn new(ground_vertex_buffer: VertexBuffer, ground_textures: Vec<Texture>, objects: Vec<Object>, light_sources: Vec<LightSource>, sound_sources: Vec<SoundSource>, effect_sources: Vec<EffectSource>, tile_vertex_buffer: Option<VertexBuffer>) -> Self {
        return Self { ground_vertex_buffer, ground_textures, objects, light_sources, sound_sources, effect_sources, tile_vertex_buffer };
    }

    pub fn update(&self, delta_time: f32) {
        self.effect_sources.iter().for_each(|effect_source| effect_source.update(delta_time));
    }

    pub fn render_geomitry(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_map {
            renderer.render_geomitry(camera, self.ground_vertex_buffer.clone(), &self.ground_textures, &Transform::new());
        }

        if render_settings.show_objects {
            self.objects.iter().for_each(|object| object.render_geometry(renderer, camera));
        }

        #[cfg(feature = "debug")]
        if render_settings.show_map_tiles {
            if let Some(vertex_buffer) = self.tile_vertex_buffer.clone() {
                renderer.render_map_tiles(camera, vertex_buffer, &Transform::new());
            }
        }
    }

    pub fn render_lights(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_ambient_light {
            renderer.ambient_light(Color::new(60, 60, 60));
        }

        if render_settings.show_directional_light {
            //renderer.directional_light(Vector3::new(0.0, -1.0, -0.7), Color::new(100, 100, 100));
        }

        if render_settings.show_point_lights {
            self.light_sources.iter().for_each(|light_source| light_source.render_lights(renderer, camera));
        }

        if render_settings.show_particle_lights {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_lights(renderer, camera));
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_markers(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_object_markers {
            self.objects.iter().for_each(|object| object.render_marker(renderer, camera));
        }

        if render_settings.show_light_markers {
            self.light_sources.iter().for_each(|light_source| light_source.render_marker(renderer, camera));
        }

        if render_settings.show_sound_markers {
            self.sound_sources.iter().for_each(|sound_source| sound_source.render_marker(renderer, camera));
        }

        if render_settings.show_effect_markers {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_marker(renderer, camera));
        }

        if render_settings.show_particle_markers {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_particle_markers(renderer, camera));
        }
    }
}
