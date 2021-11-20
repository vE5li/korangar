mod tile;
mod object;
mod light;
mod sound;
mod effect;

use cgmath::Vector2;

use graphics::{ Renderer, RenderSettings, Camera, ModelVertexBuffer, Texture, Transform, Color };

pub use self::tile::{ Tile, TileType };
pub use self::object::model;
pub use self::object::Object;
pub use self::light::LightSource;
pub use self::sound::SoundSource;
pub use self::effect::EffectSource;

#[derive(Copy, Clone, Debug)]
pub enum MarkerIdentifier {
    Object(usize),
    LightSource(usize),
    SoundSource(usize),
    EffectSource(usize),
    Particle(usize, usize),
}

pub struct Map {
    width: usize,
    height: usize,
    tiles: Vec<Tile>,
    ground_vertex_buffer: ModelVertexBuffer,
    ground_textures: Vec<Texture>,
    objects: Vec<Object>,
    light_sources: Vec<LightSource>,
    sound_sources: Vec<SoundSource>,
    effect_sources: Vec<EffectSource>,
    tile_vertex_buffer: Option<ModelVertexBuffer>, // make debug only
    ambient_light_color: Color,
}

impl Map {

    pub fn new(width: usize, height: usize, tiles: Vec<Tile>, ground_vertex_buffer: ModelVertexBuffer, ground_textures: Vec<Texture>, objects: Vec<Object>, light_sources: Vec<LightSource>, sound_sources: Vec<SoundSource>, effect_sources: Vec<EffectSource>, tile_vertex_buffer: Option<ModelVertexBuffer>, ambient_light_color: Color) -> Self {
        return Self { width, height, tiles, ground_vertex_buffer, ground_textures, objects, light_sources, sound_sources, effect_sources, tile_vertex_buffer, ambient_light_color };
    }

    pub fn update(&self, delta_time: f32) {
        self.effect_sources.iter().for_each(|effect_source| effect_source.update(delta_time));
    }

    pub fn x_in_bounds(&self, x: usize) -> bool {
        return x <= self.width;
    }

    pub fn y_in_bounds(&self, y: usize) -> bool {
        return y <= self.height;
    }

    pub fn get_tile(&self, position: &Vector2<usize>) -> &Tile {
        return &self.tiles[position.x + position.y * self.width];
    }

    pub fn render_geomitry(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_map {
            renderer.render_geomitry(camera, self.ground_vertex_buffer.clone(), &self.ground_textures, &Transform::new());
        }

        if render_settings.show_objects {
            //self.objects[5].render_geometry(renderer, camera);
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
            renderer.ambient_light(self.ambient_light_color);
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
    pub fn hovered_marker(&self, renderer: &Renderer, camera: &dyn Camera, render_settings: &RenderSettings, mouse_position: Vector2<f32>) -> Option<MarkerIdentifier> {

        let mut nearest_marker = None;
        let mut smallest_distance = f32::MAX;

        if render_settings.show_object_markers {
            for (index, object) in self.objects.iter().enumerate() {
                if let Some(new_distance) = object.hovered(renderer, camera, mouse_position, smallest_distance) {
                    nearest_marker = Some(MarkerIdentifier::Object(index));
                    smallest_distance = new_distance;
                }
            }
        }

        if render_settings.show_light_markers {
            for (index, light_source) in self.light_sources.iter().enumerate() {
                if let Some(new_distance) = light_source.hovered(renderer, camera, mouse_position, smallest_distance) {
                    nearest_marker = Some(MarkerIdentifier::LightSource(index));
                    smallest_distance = new_distance;
                }
            }
        }

        if render_settings.show_sound_markers {
            for (index, sound_source) in self.sound_sources.iter().enumerate() {
                if let Some(new_distance) = sound_source.hovered(renderer, camera, mouse_position, smallest_distance) {
                    nearest_marker = Some(MarkerIdentifier::SoundSource(index));
                    smallest_distance = new_distance;
                }
            }
        }

        if render_settings.show_effect_markers {
            for (index, effect_source) in self.effect_sources.iter().enumerate() {
                if let Some(new_distance) = effect_source.hovered(renderer, camera, mouse_position, smallest_distance) {
                    nearest_marker = Some(MarkerIdentifier::EffectSource(index));
                    smallest_distance = new_distance;
                }
            }
        }

        if render_settings.show_particle_markers {
            for (effect_index, effect_source) in self.effect_sources.iter().enumerate() {
                if let Some((new_distance, particle_index)) = effect_source.particle_hovered(renderer, camera, mouse_position, smallest_distance) {
                    nearest_marker = Some(MarkerIdentifier::Particle(effect_index, particle_index));
                    smallest_distance = new_distance;
                }
            }
        }

        return nearest_marker;
    }

    #[cfg(feature = "debug")]
    pub fn marker_information(&self, marker_identifier: &MarkerIdentifier) -> String {
        match marker_identifier {
            MarkerIdentifier::Object(index) => return self.objects[*index].information(),
            MarkerIdentifier::LightSource(index) => panic!(), //return self.light_sources[index].information(),
            MarkerIdentifier::SoundSource(index) => panic!(), //return self.sound_sources[index].information(),
            MarkerIdentifier::EffectSource(index) => panic!(), //return self.effect_sources[index].information(),
            MarkerIdentifier::Particle(index, particle_index) => panic!(),
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_markers(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_object_markers {
            //self.objects[5].render_marker(renderer, camera);
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
