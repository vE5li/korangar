mod lighting;

#[cfg(feature = "debug")]
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use cgmath::{Deg, Matrix4, Point3, SquareMatrix, Vector3};
use korangar_audio::AudioEngine;
use korangar_collision::{AABB, Frustum, KDTree, Sphere};
use korangar_container::{SimpleKey, SimpleSlab, create_simple_key};
#[cfg(feature = "debug")]
use korangar_debug::logging::Colorize;
#[cfg(feature = "debug")]
use option_ext::OptionExt;
#[cfg(feature = "debug")]
use ragnarok_formats::map::EffectSource;
#[cfg(feature = "debug")]
use ragnarok_formats::map::MapData;
use ragnarok_formats::map::{LightSource, SoundSource, Tile, TileFlags};
#[cfg(feature = "debug")]
use ragnarok_formats::transform::Transform;
use ragnarok_packets::{ClientTick, TilePosition};
use rust_state::RustState;
use wgpu::Queue;

pub use self::lighting::Lighting;
use super::{Camera, Entity, Object, PointLightId, PointLightManager, ResourceSet, ResourceSetBuffer, SubMesh, Video};
#[cfg(feature = "debug")]
use super::{LightSourceExt, Model, PointLightSet};
#[cfg(feature = "debug")]
use crate::graphics::{
    DebugAabbInstruction, DebugCircleInstruction, DebugRectangleInstruction, ModelBatch, RenderOptions, ScreenPosition, ScreenSize,
};
use crate::graphics::{EntityInstruction, IndicatorInstruction, ModelInstruction, Texture, TextureSet, WaterInstruction, WaterVertex};
use crate::loaders::GAT_TILE_SIZE;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
use crate::world::pathing::Traversable;
use crate::{Buffer, Color, GameFileLoader, ModelVertex, TileVertex};

create_simple_key!(ObjectKey, "Key to an object inside the map");
create_simple_key!(LightSourceKey, "Key to an light source inside the map");

#[cfg(feature = "debug")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MarkerIdentifier {
    Object(u32),
    LightSource(u32),
    SoundSource(u32),
    EffectSource(u32),
    Particle(u16, u16),
    Entity(u32),
    Shadow(u32),
}

#[cfg(feature = "debug")]
impl MarkerIdentifier {
    pub const SIZE: f32 = 1.5;
}

pub struct WaterPlane {
    water_opacity: f32,
    wave_height: f32,
    wave_speed: Deg<f32>,
    wave_pitch: Deg<f32>,
    texture_cycling_interval: u32,
    texture_repeat: f32,
    water_textures: Vec<Arc<Texture>>,
    vertex_buffer: Arc<Buffer<WaterVertex>>,
    index_buffer: Arc<Buffer<u32>>,
}

impl WaterPlane {
    pub fn new(
        water_opacity: f32,
        wave_height: f32,
        wave_speed: Deg<f32>,
        wave_pitch: Deg<f32>,
        texture_cycling_interval: u32,
        texture_repeat: f32,
        water_textures: Vec<Arc<Texture>>,
        vertex_buffer: Arc<Buffer<WaterVertex>>,
        index_buffer: Arc<Buffer<u32>>,
    ) -> Self {
        Self {
            water_opacity,
            wave_height,
            wave_speed,
            wave_pitch,
            texture_cycling_interval,
            texture_repeat,
            water_textures,
            vertex_buffer,
            index_buffer,
        }
    }
}

#[derive(RustState)]
pub struct Map {
    width: u16,
    height: u16,
    level_bound: AABB,
    lighting: Lighting,
    water_plane: Option<WaterPlane>,
    tiles: Vec<Tile>,
    sub_meshes: Vec<SubMesh>,
    vertex_buffer: Arc<Buffer<ModelVertex>>,
    index_buffer: Arc<Buffer<u32>>,
    texture_set: Arc<TextureSet>,
    objects: SimpleSlab<ObjectKey, Object>,
    light_sources: SimpleSlab<LightSourceKey, LightSource>,
    sound_sources: Vec<SoundSource>,
    #[cfg(feature = "debug")]
    effect_sources: Vec<EffectSource>,
    tile_picker_vertex_buffer: Buffer<TileVertex>,
    tile_picker_index_buffer: Buffer<u32>,
    #[cfg(feature = "debug")]
    tile_vertex_buffer: Arc<Buffer<ModelVertex>>,
    #[cfg(feature = "debug")]
    tile_index_buffer: Arc<Buffer<u32>>,
    #[cfg(feature = "debug")]
    tile_submeshes: Vec<SubMesh>,
    object_kdtree: KDTree<ObjectKey, AABB>,
    light_source_kdtree: KDTree<LightSourceKey, Sphere>,
    background_music_track_name: Option<String>,
    videos: Mutex<Vec<Video>>,
    #[cfg(feature = "debug")]
    map_data: MapData,
}

impl Map {
    #[cfg(not(feature = "debug"))]
    pub fn new(
        width: u16,
        height: u16,
        level_bound: AABB,
        lighting: Lighting,
        water_plane: Option<WaterPlane>,
        tiles: Vec<Tile>,
        sub_meshes: Vec<SubMesh>,
        vertex_buffer: Arc<Buffer<ModelVertex>>,
        index_buffer: Arc<Buffer<u32>>,
        texture_set: Arc<TextureSet>,
        objects: SimpleSlab<ObjectKey, Object>,
        light_sources: SimpleSlab<LightSourceKey, LightSource>,
        sound_sources: Vec<SoundSource>,
        tile_picker_vertex_buffer: Buffer<TileVertex>,
        tile_picker_index_buffer: Buffer<u32>,
        object_kdtree: KDTree<ObjectKey, AABB>,
        light_source_kdtree: KDTree<LightSourceKey, Sphere>,
        background_music_track_name: Option<String>,
        videos: Mutex<Vec<Video>>,
    ) -> Self {
        Self {
            width,
            height,
            level_bound,
            lighting,
            water_plane,
            tiles,
            sub_meshes,
            vertex_buffer,
            index_buffer,
            texture_set,
            objects,
            light_sources,
            sound_sources,
            tile_picker_vertex_buffer,
            tile_picker_index_buffer,
            object_kdtree,
            light_source_kdtree,
            background_music_track_name,
            videos,
        }
    }

    #[cfg(feature = "debug")]
    pub fn new(
        width: u16,
        height: u16,
        level_bound: AABB,
        lighting: Lighting,
        water_plane: Option<WaterPlane>,
        tiles: Vec<Tile>,
        sub_meshes: Vec<SubMesh>,
        vertex_buffer: Arc<Buffer<ModelVertex>>,
        index_buffer: Arc<Buffer<u32>>,
        texture_set: Arc<TextureSet>,
        objects: SimpleSlab<ObjectKey, Object>,
        light_sources: SimpleSlab<LightSourceKey, LightSource>,
        sound_sources: Vec<SoundSource>,
        effect_sources: Vec<EffectSource>,
        tile_picker_vertex_buffer: Buffer<TileVertex>,
        tile_picker_index_buffer: Buffer<u32>,
        tile_vertex_buffer: Arc<Buffer<ModelVertex>>,
        tile_index_buffer: Arc<Buffer<u32>>,
        tile_submeshes: Vec<SubMesh>,
        object_kdtree: KDTree<ObjectKey, AABB>,
        light_source_kdtree: KDTree<LightSourceKey, Sphere>,
        background_music_track_name: Option<String>,
        videos: Mutex<Vec<Video>>,
        map_data: MapData,
    ) -> Self {
        Self {
            width,
            height,
            level_bound,
            lighting,
            water_plane,
            tiles,
            sub_meshes,
            vertex_buffer,
            index_buffer,
            texture_set,
            objects,
            light_sources,
            sound_sources,
            effect_sources,
            tile_picker_vertex_buffer,
            tile_picker_index_buffer,
            tile_vertex_buffer,
            tile_index_buffer,
            tile_submeshes,
            object_kdtree,
            light_source_kdtree,
            background_music_track_name,
            videos,
            map_data,
        }
    }
}

impl Map {
    fn average_tile_height(tile: &Tile) -> f32 {
        (tile.southwest_corner_height + tile.southeast_corner_height + tile.northwest_corner_height + tile.northeast_corner_height) / 4.0
    }

    pub fn get_world_position(&self, position: TilePosition) -> Option<Point3<f32>> {
        let height = Self::average_tile_height(self.get_tile(position)?);

        Some(Point3::new(
            position.x as f32 * GAT_TILE_SIZE + (GAT_TILE_SIZE / 2.0),
            height,
            position.y as f32 * GAT_TILE_SIZE + (GAT_TILE_SIZE / 2.0),
        ))
    }

    pub fn get_tile(&self, position: TilePosition) -> Option<&Tile> {
        self.tiles.get(position.x as usize + position.y as usize * self.width as usize)
    }

    pub fn background_music_track_name(&self) -> Option<&str> {
        self.background_music_track_name.as_deref()
    }

    pub fn get_texture_set(&self) -> &Arc<TextureSet> {
        &self.texture_set
    }

    pub fn get_model_vertex_buffer(&self) -> &Arc<Buffer<ModelVertex>> {
        &self.vertex_buffer
    }

    pub fn get_model_index_buffer(&self) -> &Arc<Buffer<u32>> {
        &self.index_buffer
    }

    pub fn get_tile_picker_vertex_buffer(&self) -> &Buffer<TileVertex> {
        &self.tile_picker_vertex_buffer
    }

    pub fn get_level_bound(&self) -> AABB {
        self.level_bound
    }

    pub fn get_tile_picker_index_buffer(&self) -> &Buffer<u32> {
        &self.tile_picker_index_buffer
    }

    pub fn set_ambient_sound_sources(&self, audio_engine: &AudioEngine<GameFileLoader>) {
        // We increase the range of the ambient sound,
        // so that it can ease better into the world.
        const AMBIENT_SOUND_MULTIPLIER: f32 = 1.5;

        // This is the only correct place to clear the ambient sound.
        audio_engine.clear_ambient_sound();

        for sound in self.sound_sources.iter() {
            let sound_effect_key = audio_engine.load(&sound.sound_file);

            audio_engine.add_ambient_sound(
                sound_effect_key,
                sound.position,
                sound.range * AMBIENT_SOUND_MULTIPLIER,
                sound.volume,
                sound.cycle,
            );
        }

        audio_engine.prepare_ambient_sound_world();
    }

    // We want to make sure that the object set also captures the lifetime of the
    // map, so we never have a stale object set.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn cull_objects_with_frustum<'a>(
        &'a self,
        camera: &dyn Camera,
        object_set: &'a mut ResourceSetBuffer<ObjectKey>,
        #[cfg(feature = "debug")] enabled: bool,
    ) -> ResourceSet<'a, ObjectKey> {
        #[cfg(feature = "debug")]
        if !enabled {
            return object_set.create_set(|visible_objects| {
                self.objects.iter().for_each(|(object_key, _)| visible_objects.push(object_key));
            });
        }

        let frustum = Frustum::new(camera.view_projection_matrix(), true);

        object_set.create_set(|visible_objects| {
            self.object_kdtree.query(&frustum, visible_objects);
        })
    }

    // We want to make sure that the object set also captures the lifetime of the
    // map, so we never have a stale object set.
    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn cull_objects_in_sphere<'a>(
        &'a self,
        sphere: Sphere,
        object_set: &'a mut ResourceSetBuffer<ObjectKey>,
        #[cfg(feature = "debug")] enabled: bool,
    ) -> ResourceSet<'a, ObjectKey> {
        #[cfg(feature = "debug")]
        if !enabled {
            return object_set.create_set(|visible_objects| {
                self.objects.iter().for_each(|(object_key, _)| visible_objects.push(object_key));
            });
        }

        object_set.create_set(|visible_objects| {
            self.object_kdtree.query(&sphere, visible_objects);
        })
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_objects(
        &self,
        instructions: &mut Vec<ModelInstruction>,
        object_set: &ResourceSet<ObjectKey>,
        animation_timer_ms: f32,
        camera: &dyn Camera,
    ) {
        for object_key in object_set.iterate_visible().copied() {
            if let Some(object) = self.objects.get(object_key) {
                object.render_geometry(instructions, animation_timer_ms, camera);
            }
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_ground(&self, instructions: &mut Vec<ModelInstruction>) {
        self.sub_meshes.iter().for_each(|mesh| {
            instructions.push(ModelInstruction {
                model_matrix: Matrix4::identity(),
                index_offset: mesh.index_offset,
                index_count: mesh.index_count,
                base_vertex: mesh.base_vertex,
                texture_index: mesh.texture_index,
                distance: f32::MAX,
                transparent: mesh.transparent,
            });
        });
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_water<'a>(&'a self, water_instruction: &mut Option<WaterInstruction<'a>>, animation_timer_ms: f32) {
        if let Some(water_plane) = self.water_plane.as_ref() {
            let frame = animation_timer_ms / (1000.0 / 60.0);

            let waveform_phase_shift = frame * water_plane.wave_speed.0;
            let waveform_amplitude = water_plane.wave_height;
            let waveform_frequency = water_plane.wave_pitch;
            let water_opacity = water_plane.water_opacity;

            let water_texture_index = (frame as u32 / water_plane.texture_cycling_interval) % water_plane.water_textures.len() as u32;

            *water_instruction = Some(WaterInstruction {
                water_texture: &water_plane.water_textures[water_texture_index as usize],
                water_vertex_buffer: &water_plane.vertex_buffer,
                water_index_buffer: &water_plane.index_buffer,
                texture_repeat: water_plane.texture_repeat,
                waveform_phase_shift,
                waveform_amplitude,
                waveform_frequency,
                water_opacity,
            });
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_entities(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        entities: &[Entity],
        camera: &dyn Camera,
        client_tick: ClientTick,
    ) {
        entities
            .iter()
            .enumerate()
            .for_each(|(index, entity)| entity.render(instructions, camera, index != 0, client_tick));
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_dead_entities(
        &self,
        instructions: &mut Vec<EntityInstruction>,
        entities: &[Entity],
        camera: &dyn Camera,
        client_tick: ClientTick,
    ) {
        entities
            .iter()
            .for_each(|entity| entity.render(instructions, camera, false, client_tick));
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_entities_debug(&self, instructions: &mut Vec<DebugRectangleInstruction>, entities: &[Entity], camera: &dyn Camera) {
        entities.iter().for_each(|entity| {
            entity.render_debug(instructions, camera);
        });
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_bounding(
        &self,
        instructions: &mut Vec<DebugAabbInstruction>,
        frustum_culling: bool,
        object_set: &ResourceSet<ObjectKey>,
    ) {
        let intersection_set: HashSet<ObjectKey> = object_set.iterate_visible().copied().collect();

        self.objects.iter().for_each(|(object_key, object)| {
            let intersects = intersection_set.contains(&object_key);

            let color = match !frustum_culling || intersects {
                true => Color::rgb_u8(255, 255, 0),
                false => Color::rgb_u8(255, 0, 255),
            };

            let bounding_box = object.calculate_object_aabb();
            let offset = bounding_box.size().y / 2.0;
            let position = bounding_box.center() - Vector3::new(0.0, offset, 0.0);
            let transform = Transform::position(position);
            let world_matrix = Model::calculate_bounding_box_matrix(&bounding_box, &transform);

            instructions.push(DebugAabbInstruction {
                world: world_matrix,
                color,
            });
        });
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn render_walk_indicator(&self, instruction: &mut Option<IndicatorInstruction>, color: Color, position: TilePosition) {
        const OFFSET: f32 = 1.0;

        // Since the picker buffer is always one frame behind the current scene, a map
        // transition can cause the picked tile to be out of bounds. To avoid a
        // panic we ensure the coordinates are in bounds.
        if position.x >= self.width || position.y >= self.height {
            return;
        }

        let Some(tile) = self.get_tile(position) else {
            #[cfg(feature = "debug")]
            korangar_debug::logging::print_debug!("[{}] walk indicator out of map bounds", "error".red());
            return;
        };

        if tile.flags.contains(TileFlags::WALKABLE) {
            let base_x = position.x as f32 * GAT_TILE_SIZE;
            let base_y = position.y as f32 * GAT_TILE_SIZE;

            let upper_left = Point3::new(base_x, tile.southwest_corner_height + OFFSET, base_y);
            let upper_right = Point3::new(base_x + GAT_TILE_SIZE, tile.southeast_corner_height + OFFSET, base_y);
            let lower_left = Point3::new(base_x, tile.northwest_corner_height + OFFSET, base_y + GAT_TILE_SIZE);
            let lower_right = Point3::new(
                base_x + GAT_TILE_SIZE,
                tile.northeast_corner_height + OFFSET,
                base_y + GAT_TILE_SIZE,
            );

            *instruction = Some(IndicatorInstruction {
                upper_left,
                upper_right,
                lower_left,
                lower_right,
                color,
            });
        }
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn ambient_light_color(&self) -> Color {
        self.lighting.ambient_light_color()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn directional_light(&self) -> (Vector3<f32>, Color) {
        self.lighting.directional_light()
    }

    #[cfg_attr(feature = "debug", korangar_debug::profile)]
    pub fn register_point_lights(
        &self,
        point_light_manager: &mut PointLightManager,
        light_source_set_buffer: &mut ResourceSetBuffer<LightSourceKey>,
        camera: &dyn Camera,
    ) {
        let frustum = Frustum::new(camera.view_projection_matrix(), true);

        let set = light_source_set_buffer.create_set(|buffer| {
            self.light_source_kdtree.query(&frustum, buffer);
        });

        for light_source_key in set.iterate_visible().copied() {
            let light_source = self.light_sources.get(light_source_key).unwrap();

            point_light_manager.register(
                PointLightId::new(light_source_key.key()),
                light_source.position,
                light_source.color.into(),
                light_source.range,
            );
        }
    }

    #[cfg(feature = "debug")]
    pub fn get_map_data(&self) -> &MapData {
        &self.map_data
    }

    #[cfg(feature = "debug")]
    pub fn get_object(&self, key: u32) -> &Object {
        self.objects.get(ObjectKey::new(key)).expect("object key should be valid")
    }

    #[cfg(feature = "debug")]
    pub fn get_light_source(&self, key: u32) -> &LightSource {
        self.light_sources
            .get(LightSourceKey::new(key))
            .expect("light source key should be valid")
    }

    #[cfg(feature = "debug")]
    pub fn get_sound_source(&self, index: u32) -> &SoundSource {
        &self.sound_sources[index as usize]
    }

    #[cfg(feature = "debug")]
    pub fn get_effect_source(&self, index: u32) -> &EffectSource {
        &self.effect_sources[index as usize]
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_overlay_tiles(
        &self,
        model_instructions: &mut Vec<ModelInstruction>,
        model_batches: &mut Vec<ModelBatch>,
        tile_texture_set: &Arc<TextureSet>,
    ) {
        let offset = model_instructions.len();
        let count = self.tile_submeshes.len();

        self.tile_submeshes.iter().for_each(|mesh| {
            model_instructions.push(ModelInstruction {
                model_matrix: Matrix4::identity(),
                index_offset: mesh.index_offset,
                index_count: mesh.index_count,
                base_vertex: mesh.base_vertex,
                texture_index: mesh.texture_index,
                distance: f32::MAX,
                transparent: mesh.transparent,
            });
        });

        model_batches.push(ModelBatch {
            offset,
            count,
            texture_set: tile_texture_set.clone(),
            vertex_buffer: self.tile_vertex_buffer.clone(),
            index_buffer: self.tile_index_buffer.clone(),
        });
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_entity_pathing(
        &self,
        model_instructions: &mut Vec<ModelInstruction>,
        model_batches: &mut Vec<ModelBatch>,
        entities: &[Entity],
        path_texture_set: &Arc<TextureSet>,
    ) {
        entities.iter().for_each(|entity| {
            if let Some(pathing) = entity.get_pathing() {
                let offset = model_instructions.len();

                pathing.submeshes.iter().for_each(|mesh| {
                    model_instructions.push(ModelInstruction {
                        model_matrix: Matrix4::identity(),
                        index_offset: mesh.index_offset,
                        index_count: mesh.index_count,
                        base_vertex: mesh.base_vertex,
                        texture_index: mesh.texture_index,
                        distance: f32::MAX,
                        transparent: mesh.transparent,
                    });
                });

                model_batches.push(ModelBatch {
                    offset,
                    count: pathing.submeshes.len(),
                    texture_set: path_texture_set.clone(),
                    vertex_buffer: pathing.vertex_buffer.clone(),
                    index_buffer: pathing.index_buffer.clone(),
                });
            }
        });
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_markers(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        render_options: &RenderOptions,
        entities: &[Entity],
        point_light_set: &PointLightSet,
        hovered_marker_identifier: Option<MarkerIdentifier>,
    ) {
        use super::SoundSourceExt;
        use crate::EffectSourceExt;

        if render_options.show_object_markers {
            self.objects.iter().for_each(|(object_key, object)| {
                let marker_identifier = MarkerIdentifier::Object(object_key.key());

                object.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_light_markers {
            self.light_sources.iter().for_each(|(key, light_source)| {
                let marker_identifier = MarkerIdentifier::LightSource(key.key());

                light_source.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_sound_markers {
            self.sound_sources.iter().enumerate().for_each(|(index, sound_source)| {
                let marker_identifier = MarkerIdentifier::SoundSource(index as u32);

                sound_source.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_effect_markers {
            self.effect_sources.iter().enumerate().for_each(|(index, effect_source)| {
                let marker_identifier = MarkerIdentifier::EffectSource(index as u32);

                effect_source.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_entity_markers {
            entities.iter().enumerate().for_each(|(index, entity)| {
                let marker_identifier = MarkerIdentifier::Entity(index as u32);

                entity.render_marker(
                    renderer,
                    camera,
                    marker_identifier,
                    hovered_marker_identifier.contains(&marker_identifier),
                )
            });
        }

        if render_options.show_shadow_markers {
            point_light_set
                .with_shadow_iterator()
                .enumerate()
                .for_each(|(index, light_source)| {
                    let marker_identifier = MarkerIdentifier::Shadow(index as u32);

                    renderer.render_marker(
                        camera,
                        marker_identifier,
                        light_source.position,
                        hovered_marker_identifier.contains(&marker_identifier),
                    );
                });
        }
    }

    #[cfg(feature = "debug")]
    #[korangar_debug::profile]
    pub fn render_marker_overlay(
        &self,
        aabb_instructions: &mut Vec<DebugAabbInstruction>,
        circle_instructions: &mut Vec<DebugCircleInstruction>,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        point_light_set: &PointLightSet,
        animation_timer_ms: f32,
    ) {
        let animation_seconds = animation_timer_ms / 1000.0;
        let offset = (f32::sin(animation_seconds * 5.0) + 0.5).clamp(0.0, 1.0);
        let overlay_color = Color::rgb(1.0, offset, 1.0 - offset);

        match marker_identifier {
            MarkerIdentifier::Object(key) => self
                .objects
                .get(ObjectKey::new(key))
                .unwrap()
                .render_bounding_box(aabb_instructions, overlay_color),

            MarkerIdentifier::LightSource(key) => {
                let light_source = self.light_sources.get(LightSourceKey::new(key)).unwrap();

                if let Some((screen_position, screen_size)) =
                    Self::calculate_circle_screen_position_size(camera, light_source.position, light_source.range)
                {
                    circle_instructions.push(DebugCircleInstruction {
                        position: light_source.position,
                        color: overlay_color,
                        screen_position,
                        screen_size,
                    });
                };
            }
            MarkerIdentifier::SoundSource(index) => {
                let sound_source = &self.sound_sources[index as usize];

                if let Some((screen_position, screen_size)) =
                    Self::calculate_circle_screen_position_size(camera, sound_source.position, sound_source.range)
                {
                    circle_instructions.push(DebugCircleInstruction {
                        position: sound_source.position,
                        color: overlay_color,
                        screen_position,
                        screen_size,
                    });
                };
            }
            MarkerIdentifier::EffectSource(_index) => {}
            MarkerIdentifier::Particle(_index, _particle_index) => {}
            MarkerIdentifier::Entity(_index) => {}
            MarkerIdentifier::Shadow(index) => {
                let point_light = point_light_set.with_shadow_iterator().nth(index as usize).unwrap();

                if let Some((screen_position, screen_size)) =
                    Self::calculate_circle_screen_position_size(camera, point_light.position, point_light.range)
                {
                    circle_instructions.push(DebugCircleInstruction {
                        position: point_light.position,
                        color: overlay_color,
                        screen_position,
                        screen_size,
                    });
                };
            }
        }
    }

    #[cfg(feature = "debug")]
    fn calculate_circle_screen_position_size(
        camera: &dyn Camera,
        position: Point3<f32>,
        extent: f32,
    ) -> Option<(ScreenPosition, ScreenSize)> {
        let corner_offset = (extent.powf(2.0) * 2.0).sqrt();
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, corner_offset);

        if top_left_position.w < 0.1 && bottom_right_position.w < 0.1 && camera.distance_to(position) > extent {
            return None;
        }

        let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);
        Some((screen_position, screen_size))
    }

    pub fn advance_videos(&self, queue: &Queue, delta_time: f64) {
        let mut videos = self.videos.lock().unwrap();

        for video in videos.iter_mut() {
            if video.should_show_next_frame(delta_time) {
                video.update_texture(queue);
            }

            video.check_for_next_frame();
        }
    }
}

impl Traversable for Map {
    fn is_walkable(&self, position: TilePosition) -> bool {
        self.get_tile(position)
            .map(|tile| tile.flags.contains(TileFlags::WALKABLE))
            .unwrap_or(false)
    }

    fn is_snipeable(&self, position: TilePosition) -> bool {
        self.get_tile(position)
            .map(|tile| tile.flags.contains(TileFlags::SNIPABLE))
            .unwrap_or(false)
    }
}
