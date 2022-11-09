mod tile;

use cgmath::{Array, EuclideanSpace, Matrix4, Point3, SquareMatrix, Vector2, Vector3};
use collision::{Aabb3, Frustum, Relation};
use derive_new::new;
use procedural::*;

pub use self::tile::{Tile, TileType};
use crate::graphics::*;
#[cfg(feature = "debug")]
use crate::interface::PrototypeWindow;
use crate::loaders::Version;
use crate::network::ClientTick;
use crate::world::*;

// MOVE
fn get_value(day_timer: f32, offset: f32, p: f32) -> f32 {
    let sin = (day_timer + offset).sin();
    sin.abs().powf(2.0 - p) / sin
}

fn get_channels(day_timer: f32, offset: f32, ps: [f32; 3]) -> Vector3<f32> {
    let red = get_value(day_timer, offset, ps[0]);
    let green = get_value(day_timer, offset, ps[1]);
    let blue = get_value(day_timer, offset, ps[2]);
    Vector3::new(red, green, blue)
}

fn color_from_channel(base_color: Color, channels: Vector3<f32>) -> Color {
    Color::rgb(
        (base_color.red_f32() * channels.x) as u8,
        (base_color.green_f32() * channels.y) as u8,
        (base_color.blue_f32() * channels.z) as u8,
    )
}

fn get_ambient_light_color(ambient_color: Color, day_timer: f32) -> Color {
    let sun_offset = 0.0;
    let ambient_channels = (get_channels(day_timer, sun_offset, [0.3, 0.2, 0.2]) * 0.35 + Vector3::from_value(0.65)) * 255.0;
    color_from_channel(ambient_color, ambient_channels)
}

fn get_directional_light_color_intensity(directional_color: Color, intensity: f32, day_timer: f32) -> (Color, f32) {
    let sun_offset = 0.0;
    let moon_offset = std::f32::consts::PI;

    let directional_channels = get_channels(day_timer, sun_offset, [0.8, 0.0, 0.25]) * 255.0;

    if directional_channels.x.is_sign_positive() {
        let directional_color = color_from_channel(directional_color, directional_channels);
        return (directional_color, f32::min(intensity * 1.2, 1.0));
    }

    let directional_channels = get_channels(day_timer, moon_offset, [0.3; 3]) * 255.0;
    let directional_color = color_from_channel(Color::rgb(150, 150, 255), directional_channels);

    (directional_color, f32::min(intensity * 1.2, 1.0))
}

pub fn get_light_direction(day_timer: f32) -> Vector3<f32> {
    let sun_offset = -std::f32::consts::FRAC_PI_2;
    let c = (day_timer + sun_offset).cos();
    let s = (day_timer + sun_offset).sin();

    match c.is_sign_positive() {
        true => Vector3::new(s, c, -0.5),
        false => Vector3::new(s, -c, -0.5),
    }
}
//

#[derive(Debug, PrototypeElement, new)]
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

#[derive(Debug, PrototypeElement, new)]
pub struct LightSettings {
    #[new(value = "0")]
    pub light_longitude: isize,
    #[new(value = "0")]
    pub light_latitude: isize,
    #[new(value = "Color::monochrome(255)")]
    pub diffuse_color: Color,
    #[new(value = "Color::monochrome(255)")]
    pub ambient_color: Color,
    #[new(value = "1.0")]
    pub light_intensity: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MarkerIdentifier {
    Object(usize),
    LightSource(usize),
    SoundSource(usize),
    EffectSource(usize),
    Particle(usize, usize),
    Entity(usize),
}

impl MarkerIdentifier {
    pub const SIZE: f32 = 1.5;
}

#[derive(PrototypeElement, PrototypeWindow, new)]
#[window_title("Map Viewer")]
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
    pub fn x_in_bounds(&self, x: usize) -> bool {
        x <= self.width
    }

    pub fn y_in_bounds(&self, y: usize) -> bool {
        y <= self.height
    }

    pub fn get_world_position(&self, position: Vector2<usize>) -> Vector3<f32> {
        let height = self.get_tile(position).average_height();
        Vector3::new(position.x as f32 * 5.0 + 2.5, height, position.y as f32 * 5.0 + 2.5)
    }

    pub fn get_tile(&self, position: Vector2<usize>) -> &Tile {
        &self.tiles[position.x + position.y * self.width]
    }

    pub fn render_ground<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, time: f32)
    where
        T: Renderer + GeometryRenderer,
    {
        renderer.render_geometry(
            render_target,
            camera,
            self.ground_vertex_buffer.clone(),
            &self.ground_textures,
            Matrix4::identity(),
            time,
        );
    }

    pub fn render_objects<T>(
        &self,
        render_target: &mut T::Target,
        renderer: &T,
        camera: &dyn Camera,
        client_tick: ClientTick,
        time: f32,
        #[cfg(feature = "debug")] frustum_culling: bool,
    ) where
        T: Renderer + GeometryRenderer,
    {
        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let frustum = Frustum::from_matrix4(projection_matrix * view_matrix).unwrap();
        let standard_box = OrientedBox::default();

        for object in &self.objects {
            #[cfg(feature = "debug")]
            if !frustum_culling {
                object.render_geometry(render_target, renderer, camera, client_tick, time);
                continue;
            }

            let bounding_box_matrix = object.get_bounding_box_matrix();
            let oriented_bounding_box = standard_box.transform(bounding_box_matrix);
            let bounding_box = BoundingBox::new(oriented_bounding_box.corners);
            let collision_bounding_box = Aabb3 {
                min: Point3::from_vec(bounding_box.smallest),
                max: Point3::from_vec(bounding_box.biggest),
            };
            let culled = matches!(frustum.contains(&collision_bounding_box), Relation::Out);

            if !culled {
                object.render_geometry(render_target, renderer, camera, client_tick, time);
            };
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        player_camera: &dyn Camera,
        frustum_culling: bool,
    ) {
        let (view_matrix, projection_matrix) = player_camera.view_projection_matrices();
        let frustum = Frustum::from_matrix4(projection_matrix * view_matrix).unwrap();
        let standard_box = OrientedBox::default();

        for object in &self.objects {
            let bounding_box_matrix = object.get_bounding_box_matrix();
            let oriented_bounding_box = standard_box.transform(bounding_box_matrix);
            let bounding_box = BoundingBox::new(oriented_bounding_box.corners);
            let collision_bounding_box = Aabb3 {
                min: Point3::from_vec(bounding_box.smallest),
                max: Point3::from_vec(bounding_box.biggest),
            };
            let culled = matches!(frustum.contains(&collision_bounding_box), Relation::Out);

            let color = match !frustum_culling || !culled {
                true => Color::rgb(255, 255, 0),
                false => Color::rgb(255, 0, 255),
            };

            let offset = bounding_box.size().y / 2.0;
            let position = bounding_box.center() - Vector3::new(0.0, offset, 0.0);
            let transform = Transform::position(position);

            renderer.render_bounding_box(render_target, camera, &transform, &bounding_box, color);
        }
    }

    pub fn render_tiles(&self, render_target: &mut <PickerRenderer as Renderer>::Target, renderer: &PickerRenderer, camera: &dyn Camera) {
        renderer.render_tiles(render_target, camera, self.tile_picker_vertex_buffer.clone());
    }

    pub fn render_water(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        day_timer: f32,
    ) {
        if let Some(water_vertex_buffer) = &self.water_vertex_buffer {
            renderer.render_water(render_target, camera, water_vertex_buffer.clone(), day_timer);
        }
    }

    pub fn ambient_light(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, day_timer: f32) {
        let ambient_color = get_ambient_light_color(self.light_settings.ambient_color, day_timer);
        renderer.ambient_light(render_target, ambient_color);
    }

    pub fn directional_light(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        light_image: ImageBuffer,
        light_matrix: Matrix4<f32>,
        day_timer: f32,
    ) {
        let light_direction = get_light_direction(day_timer);
        let (directional_color, intensity) = get_directional_light_color_intensity(
            self.light_settings.diffuse_color,
            self.light_settings.light_intensity,
            day_timer,
        );

        renderer.directional_light(
            render_target,
            camera,
            light_image,
            light_matrix,
            light_direction,
            directional_color,
            intensity,
        );
    }

    pub fn point_lights(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        self.light_sources
            .iter()
            .for_each(|light_source| light_source.render_light(render_target, renderer, camera));
    }

    pub fn water_light(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        renderer.water_light(render_target, camera, self.water_settings.water_level);
    }

    #[cfg(feature = "debug")]
    pub fn to_prototype_window(&self) -> &dyn PrototypeWindow {
        self
    }

    #[cfg(feature = "debug")]
    pub fn render_overlay_tiles(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
    ) {
        renderer.render_overlay_tiles(render_target, camera, self.tile_vertex_buffer.clone());
    }

    #[cfg(feature = "debug")]
    pub fn resolve_marker<'a>(&'a self, entities: &'a [Entity], marker_identifier: MarkerIdentifier) -> &dyn PrototypeWindow {
        match marker_identifier {
            MarkerIdentifier::Object(index) => &self.objects[index],
            MarkerIdentifier::LightSource(index) => &self.light_sources[index],
            MarkerIdentifier::SoundSource(index) => &self.sound_sources[index],
            MarkerIdentifier::EffectSource(index) => &self.effect_sources[index],
            MarkerIdentifier::Particle(..) => panic!(),
            // TODO: implement properly
            MarkerIdentifier::Entity(index) => &entities[index],
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_markers<T>(
        &self,
        render_target: &mut T::Target,
        renderer: &T,
        camera: &dyn Camera,
        render_settings: &RenderSettings,
        entities: &[Entity],
        hovered_marker_identifier: Option<MarkerIdentifier>,
    ) where
        T: Renderer + MarkerRenderer,
    {
        if render_settings.show_object_markers {
            self.objects.iter().enumerate().for_each(|(index, object)| {
                let marker_identifier = MarkerIdentifier::Object(index);

                object.render_marker(
                    render_target,
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_settings.show_light_markers {
            self.light_sources.iter().enumerate().for_each(|(index, light_source)| {
                let marker_identifier = MarkerIdentifier::LightSource(index);

                light_source.render_marker(
                    render_target,
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_settings.show_sound_markers {
            self.sound_sources.iter().enumerate().for_each(|(index, sound_source)| {
                let marker_identifier = MarkerIdentifier::SoundSource(index);

                sound_source.render_marker(
                    render_target,
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_settings.show_effect_markers {
            self.effect_sources.iter().enumerate().for_each(|(index, effect_source)| {
                let marker_identifier = MarkerIdentifier::EffectSource(index);

                effect_source.render_marker(
                    render_target,
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_settings.show_entity_markers {
            entities.iter().enumerate().for_each(|(index, entity)| {
                let marker_identifier = MarkerIdentifier::Entity(index);

                entity.render_marker(
                    render_target,
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker_box(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
    ) {
        match marker_identifier {
            MarkerIdentifier::Object(index) => self.objects[index].render_bounding_box(render_target, renderer, camera),
            MarkerIdentifier::LightSource(_index) => {}
            MarkerIdentifier::SoundSource(_index) => {}
            MarkerIdentifier::EffectSource(_index) => {}
            MarkerIdentifier::Particle(_index, _particle_index) => {}
            MarkerIdentifier::Entity(_index) => {}
        }
    }
}
