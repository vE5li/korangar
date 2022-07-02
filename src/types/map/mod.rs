mod tile;
mod object;
mod light;
mod sound;
mod effect;

use derive_new::new;
use cgmath::Vector2;

use crate::types::Version;
use graphics::*;
#[cfg(feature = "debug")]
use crate::interface::traits::PrototypeWindow;

pub use self::tile::{ Tile, TileType };
pub use self::object::{ Object, model };
pub use self::light::LightSource;
pub use self::sound::SoundSource;
pub use self::effect::{ EffectSource, Particle };

#[derive(PrototypeElement, new)]
pub struct WaterSettings {
    #[new(value = "0.0")]
    pub water_level: f32,
    #[new(value = "0")]
    pub water_type: usize,
    #[new(value = "0.0")]
    pub wave_height: f32,
    #[new(value = "0.0")]
    pub wave_speed: f32,
    #[new(value = "0.0")]
    pub wave_pitch: f32,
    #[new(value = "0")]
    pub water_animation_speed: usize,
}

#[derive(PrototypeElement, new)]
pub struct LightSettings {
    #[new(value = "0")]
    pub light_longitude: isize,
    #[new(value = "0")]
    pub light_latitude: isize,
    #[new(value = "Color::monochrome(255)")]
    pub diffuse_color: Color,
    #[new(value = "Color::monochrome(255)")]
    pub ambient_color: Color,
}

#[derive(Copy, Clone, Debug)]
pub enum MarkerIdentifier {
    Object(usize),
    LightSource(usize),
    SoundSource(usize),
    EffectSource(usize),
    Particle(usize, usize),
}

#[derive(PrototypeElement, PrototypeWindow, new)]
#[window_title("map viewer")]
#[window_class("map_viewer")]
pub struct Map {
    resource_version: Version,
    ground_version: Version,
    width: usize,
    height: usize,
    water_settings: WaterSettings,
    light_settings: LightSettings,
    #[hidden_element]
    tiles: Vec<Tile>,
    #[hidden_element]
    ground_vertex_buffer: ModelVertexBuffer,
    #[hidden_element]
    water_vertex_buffer: Option<WaterVertexBuffer>,
    #[hidden_element]
    ground_textures: Vec<Texture>,
    objects: Vec<Object>,
    light_sources: Vec<LightSource>,
    sound_sources: Vec<SoundSource>,
    effect_sources: Vec<EffectSource>,
    #[hidden_element]
    tile_picker_vertex_buffer: TileVertexBuffer,
    #[hidden_element]
    tile_vertex_buffer: ModelVertexBuffer, // make debug only
}

impl Map {

    pub fn update(&self, delta_time: f32) {
        self.effect_sources.iter().for_each(|effect_source| effect_source.update(delta_time));
    }

    pub fn x_in_bounds(&self, x: usize) -> bool {
        x <= self.width
    }

    pub fn y_in_bounds(&self, y: usize) -> bool {
        y <= self.height
    }

    pub fn get_height_at(&self, position: Vector2<usize>) -> f32 {
        self.get_tile(position).average_height()
    }

    pub fn get_tile(&self, position: Vector2<usize>) -> &Tile {
        &self.tiles[position.x + position.y * self.width]
    }

    pub fn render_picker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_tiles(camera, self.tile_picker_vertex_buffer.clone());
    }

    pub fn render_geomitry(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_map {
            renderer.render_geomitry(camera, self.ground_vertex_buffer.clone(), &self.ground_textures, &Transform::new());
        }

        if render_settings.show_objects {
            self.objects.iter().for_each(|object| object.render_geometry(renderer, camera));
        }

        if let Some(water_vertex_buffer) = &self.water_vertex_buffer {
            renderer.render_water(camera, water_vertex_buffer.clone());
        }

        #[cfg(feature = "debug")]
        if render_settings.show_map_tiles {
            renderer.render_map_tiles(camera, self.tile_vertex_buffer.clone(), &Transform::new());
        }
    }

    pub fn render_lights(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings) {

        if render_settings.show_ambient_light {
            renderer.ambient_light(self.light_settings.ambient_color);
        }

        if render_settings.show_directional_light {
            renderer.directional_light(camera, cgmath::Vector3::new(0.0, -1.0, 1.0), self.light_settings.diffuse_color);
        }

        if render_settings.show_point_lights {
            self.light_sources.iter().for_each(|light_source| light_source.render_lights(renderer, camera));
        }

        if render_settings.show_particle_lights {
            self.effect_sources.iter().for_each(|effect_source| effect_source.render_lights(renderer, camera));
        }

        if render_settings.show_water {
            renderer.water_light(camera, self.water_settings.water_level);
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

        nearest_marker
    }

    #[cfg(feature = "debug")]
    pub fn resolve_marker(&self, marker_identifier: MarkerIdentifier) -> &dyn PrototypeWindow {
        match marker_identifier {
            MarkerIdentifier::Object(index) => &self.objects[index],
            MarkerIdentifier::LightSource(index) => &self.light_sources[index],
            MarkerIdentifier::SoundSource(index) => &self.sound_sources[index],
            MarkerIdentifier::EffectSource(index) => &self.effect_sources[index],
            MarkerIdentifier::Particle(index, particle_index) => &self.effect_sources[index].particles[particle_index],
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_markers(&self, renderer: &mut Renderer, camera: &dyn Camera, render_settings: &RenderSettings, marker_identifier: Option<MarkerIdentifier>) {

        if render_settings.show_object_markers {
            self.objects.iter().enumerate().for_each(|(index, object)| object.render_marker(renderer, camera, matches!(marker_identifier, Some(MarkerIdentifier::Object(x)) if x == index)));
        }

        if render_settings.show_light_markers {
            self.light_sources.iter().enumerate().for_each(|(index, light_source)| light_source.render_marker(renderer, camera, matches!(marker_identifier, Some(MarkerIdentifier::LightSource(x)) if x == index)));
        }

        if render_settings.show_sound_markers {
            self.sound_sources.iter().enumerate().for_each(|(index, sound_source)| sound_source.render_marker(renderer, camera, matches!(marker_identifier, Some(MarkerIdentifier::SoundSource(x)) if x == index)));
        }

        if render_settings.show_effect_markers {
            self.effect_sources.iter().enumerate().for_each(|(index, effect_source)| effect_source.render_marker(renderer, camera, matches!(marker_identifier, Some(MarkerIdentifier::EffectSource(x)) if x == index)));
        }

        if render_settings.show_particle_markers {
            for (index, effect_source) in self.effect_sources.iter().enumerate() {
                effect_source.particles.iter().enumerate().for_each(|(particle_index, particle)| particle.render_marker(renderer, camera, matches!(marker_identifier, Some(MarkerIdentifier::Particle(x, y)) if x == index && y == particle_index)));
            }
        }
    }
}
