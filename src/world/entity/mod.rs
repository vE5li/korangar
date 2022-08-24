use std::sync::Arc;

use cgmath::{Array, Vector2, Vector3, VectorSpace};
use derive_new::new;
use procedural::*;
#[cfg(feature = "debug")]
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
#[cfg(feature = "debug")]
use vulkano::device::Device;
use vulkano::sync::GpuFuture;

use crate::database::Database;
use crate::graphics::{Camera, Color, DeferredRenderer, EntityRenderer, MarkerRenderer, Renderer};
#[cfg(feature = "debug")]
use crate::graphics::{ModelVertexBuffer, NativeModelVertex};
use crate::loaders::{ActionLoader, Actions, Sprite, SpriteLoader};
use crate::network::{CharacterInformation, EntityData, StatusType};
use crate::world::Map;

pub enum ResourceState<T> {
    Avalible(T),
    Unavalible,
    Requested,
}

impl<T> ResourceState<T> {

    pub fn as_option(&self) -> Option<&T> {
        match self {
            ResourceState::Avalible(value) => Some(value),
            _requested_or_unavalible => None,
        }
    }
}

#[derive(Clone, new, PrototypeElement)]
pub struct Movement {
    #[hidden_element]
    steps: Vec<(Vector2<usize>, u32)>,
    starting_timestamp: u32,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub steps_vertex_buffer: Option<ModelVertexBuffer>,
}

pub struct Common {
    pub entity_id: u32,
    pub job_id: usize,
    pub health_points: usize,
    pub maximum_health_points: usize,
    pub movement_speed: usize,

    pub active_movement: Option<Movement>,
    pub sprite: Arc<Sprite>,
    pub actions: Arc<Actions>,
    pub position: Vector3<f32>,
    details: ResourceState<String>,
}

impl Common {

    pub fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        map: &Map,
        database: &Database,
        entity_data: EntityData,
    ) -> Self {

        let entity_id = entity_data.entity_id;
        let job_id = entity_data.job as usize;
        let position = map.get_world_position(entity_data.position);

        let movement_speed = entity_data.movement_speed as usize;
        let health_points = entity_data.health_points as usize;
        let maximum_health_points = entity_data.maximum_health_points as usize;

        let active_movement = None;
        let file_path = format!("npc\\{}", database.job_name_from_id(job_id));
        let sprite = sprite_loader.get(&format!("{}.spr", file_path), texture_future).unwrap();
        let actions = action_loader.get(&format!("{}.act", file_path)).unwrap();
        let details = ResourceState::Unavalible;

        Self {
            position,
            entity_id,
            job_id,
            active_movement,
            movement_speed,
            health_points,
            maximum_health_points,
            sprite,
            actions,
            details,
        }
    }

    pub fn set_position(&mut self, map: &Map, position: Vector2<usize>) {

        self.position = map.get_world_position(position);
        self.active_movement = None;
    }

    pub fn update(&mut self, map: &Map, _delta_time: f32, client_tick: u32) {
        if let Some(active_movement) = self.active_movement.take() {

            let last_step = active_movement.steps.last().unwrap();

            if client_tick > last_step.1 {

                let position = Vector2::new(last_step.0.x, last_step.0.y);
                self.set_position(map, position);
            } else {

                let mut last_step_index = 0;
                while active_movement.steps[last_step_index + 1].1 < client_tick {
                    last_step_index += 1;
                }

                let last_step = active_movement.steps[last_step_index];
                let next_step = active_movement.steps[last_step_index + 1];

                let last_step_position = map.get_world_position(last_step.0);
                let next_step_position = map.get_world_position(next_step.0);

                let clamped_tick = u32::max(last_step.1, client_tick);
                let total = next_step.1 - last_step.1;
                let offset = clamped_tick - last_step.1;

                let movement_elapsed = (1.0 / total as f32) * offset as f32;
                let position = last_step_position.lerp(next_step_position, movement_elapsed);

                self.position = position;
                self.active_movement = active_movement.into();
            }
        }
    }

    pub fn move_from_to(&mut self, map: &Map, from: Vector2<usize>, to: Vector2<usize>, starting_timestamp: u32) {

        use pathfinding::prelude::bfs;

        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        struct Pos(usize, usize);

        impl Pos {

            fn successors(&self, map: &Map) -> Vec<Pos> {

                let &Pos(x, y) = self;
                let mut successors = Vec::new();

                if map.x_in_bounds(x + 1) {
                    successors.push(Pos(x + 1, y));
                }

                if x > 0 {
                    successors.push(Pos(x - 1, y));
                }

                if map.y_in_bounds(y + 1) {
                    successors.push(Pos(x, y + 1));
                }

                if y > 0 {
                    successors.push(Pos(x, y - 1));
                }

                if map.x_in_bounds(x + 1)
                    && map.y_in_bounds(y + 1)
                    && map.get_tile(Vector2::new(x + 1, y)).is_walkable()
                    && map.get_tile(Vector2::new(x, y + 1)).is_walkable()
                {
                    successors.push(Pos(x + 1, y + 1));
                }

                if x > 0
                    && map.y_in_bounds(y + 1)
                    && map.get_tile(Vector2::new(x - 1, y)).is_walkable()
                    && map.get_tile(Vector2::new(x, y + 1)).is_walkable()
                {
                    successors.push(Pos(x - 1, y + 1));
                }

                if map.x_in_bounds(x + 1)
                    && y > 0
                    && map.get_tile(Vector2::new(x + 1, y)).is_walkable()
                    && map.get_tile(Vector2::new(x, y - 1)).is_walkable()
                {
                    successors.push(Pos(x + 1, y - 1));
                }

                if x > 0
                    && y > 0
                    && map.get_tile(Vector2::new(x - 1, y)).is_walkable()
                    && map.get_tile(Vector2::new(x, y - 1)).is_walkable()
                {
                    successors.push(Pos(x - 1, y - 1));
                }

                let successors = successors
                    .drain(..)
                    .filter(|Pos(x, y)| map.get_tile(Vector2::new(*x, *y)).is_walkable())
                    .collect::<Vec<Pos>>();

                successors
            }

            fn to_vector(self) -> Vector2<usize> {
                Vector2::new(self.0, self.1)
            }
        }

        let result = bfs(&Pos(from.x, from.y), |p| p.successors(map), |p| *p == Pos(to.x, to.y));

        if let Some(path) = result {

            let steps: Vec<(Vector2<usize>, u32)> = path
                .into_iter()
                .enumerate()
                .map(|(index, pos)| {

                    let arrival_timestamp = starting_timestamp + index as u32 * self.movement_speed as u32;
                    (pos.to_vector(), arrival_timestamp)
                })
                .collect();

            self.active_movement = Movement::new(steps, starting_timestamp).into();
        }
    }

    #[cfg(feature = "debug")]
    fn generate_step_texture_coordinates(
        steps: &Vec<(Vector2<usize>, u32)>,
        step: Vector2<usize>,
        index: usize,
    ) -> ([Vector2<f32>; 4], i32) {

        if steps.len() - 1 == index {

            return (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                ],
                0,
            );
        }

        let delta = steps[index + 1].0.map(|component| component as isize) - step.map(|component| component as isize);

        match delta {

            Vector2 { x: 1, y: 0 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                ],
                1,
            ),

            Vector2 { x: -1, y: 0 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
                1,
            ),

            Vector2 { x: 0, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                1,
            ),

            Vector2 { x: 0, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                1,
            ),

            Vector2 { x: 1, y: 1 } => (
                [
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                ],
                2,
            ),

            Vector2 { x: -1, y: 1 } => (
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                ],
                2,
            ),

            Vector2 { x: 1, y: -1 } => (
                [
                    Vector2::new(1.0, 1.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 0.0),
                    Vector2::new(0.0, 1.0),
                ],
                2,
            ),

            Vector2 { x: -1, y: -1 } => (
                [
                    Vector2::new(1.0, 0.0),
                    Vector2::new(1.0, 1.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(0.0, 0.0),
                ],
                2,
            ),

            _other => panic!("incorrent pathing"),
        }
    }

    #[cfg(feature = "debug")]
    pub fn generate_steps_vertex_buffer(&mut self, device: Arc<Device>, map: &Map) {

        let mut native_steps_vertices = Vec::new();
        let mut active_movement = self.active_movement.as_mut().unwrap();

        for (index, (step, _)) in active_movement.steps.iter().cloned().enumerate() {

            let tile = map.get_tile(step);
            let offset = Vector2::new(step.x as f32 * 5.0, step.y as f32 * 5.0);

            let first_position = Vector3::new(offset.x, tile.upper_left_height + 1.0, offset.y);
            let second_position = Vector3::new(offset.x + 5.0, tile.upper_right_height + 1.0, offset.y);
            let third_position = Vector3::new(offset.x + 5.0, tile.lower_right_height + 1.0, offset.y + 5.0);
            let fourth_position = Vector3::new(offset.x, tile.lower_left_height + 1.0, offset.y + 5.0);

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

            let (texture_coordinates, texture_index) = Self::generate_step_texture_coordinates(&active_movement.steps, step, index);

            native_steps_vertices.push(NativeModelVertex::new(
                first_position,
                first_normal,
                texture_coordinates[0],
                texture_index,
            ));
            native_steps_vertices.push(NativeModelVertex::new(
                second_position,
                first_normal,
                texture_coordinates[1],
                texture_index,
            ));
            native_steps_vertices.push(NativeModelVertex::new(
                third_position,
                first_normal,
                texture_coordinates[2],
                texture_index,
            ));

            native_steps_vertices.push(NativeModelVertex::new(
                first_position,
                second_normal,
                texture_coordinates[0],
                texture_index,
            ));
            native_steps_vertices.push(NativeModelVertex::new(
                third_position,
                second_normal,
                texture_coordinates[2],
                texture_index,
            ));
            native_steps_vertices.push(NativeModelVertex::new(
                fourth_position,
                second_normal,
                texture_coordinates[3],
                texture_index,
            ));
        }

        let steps_vertices = NativeModelVertex::to_vertices(native_steps_vertices);
        let vertex_buffer = CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), false, steps_vertices.into_iter()).unwrap();
        active_movement.steps_vertex_buffer = Some(vertex_buffer);
    }

    pub fn render<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera)
    where
        T: Renderer + EntityRenderer,
    {

        renderer.render_entity(
            render_target,
            camera,
            self.sprite.textures[0].clone(),
            self.position,
            Vector3::new(0.0, 3.0, 0.0),
            Vector2::from_value(1.0),
            Vector2::new(1, 1),
            Vector2::new(0, 0),
            self.entity_id as usize,
        );
    }

    #[cfg(feature = "debug")]
    pub fn render_marker<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, hovered: bool)
    where
        T: Renderer + MarkerRenderer,
    {
        renderer.render_marker(render_target, camera, self.position, hovered);
    }
}

pub struct Player {
    common: Common,
    pub spell_points: usize,
    pub activity_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
}

impl Player {

    pub fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        map: &Map,
        database: &Database,
        character_information: CharacterInformation,
        player_position: Vector2<usize>,
    ) -> Self {

        let spell_points = character_information.spell_points as usize;
        let activity_points = 0;
        let maximum_spell_points = character_information.maximum_spell_points as usize;
        let maximum_activity_points = 0;
        let common = Common::new(
            sprite_loader,
            action_loader,
            texture_future,
            map,
            database,
            EntityData::from_character(character_information, player_position),
        );

        Self {
            common,
            spell_points,
            activity_points,
            maximum_spell_points,
            maximum_activity_points,
        }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn update_status(&mut self, status_type: StatusType) {
        match status_type {
            StatusType::MaximumHealthPoints(value) => self.common.maximum_health_points = value as usize,
            StatusType::MaximumSpellPoints(value) => self.maximum_spell_points = value as usize,
            StatusType::HealthPoints(value) => self.common.health_points = value as usize,
            StatusType::SpellPoints(value) => self.spell_points = value as usize,
            StatusType::ActivityPoints(value) => self.activity_points = value as usize,
            StatusType::MaximumActivityPoints(value) => self.maximum_activity_points = value as usize,
            _ => {}
        }
    }

    pub fn render_status(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: Vector2<f32>,
    ) {

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let clip_space_position = (projection_matrix * view_matrix) * self.common.position.extend(1.0);
        let screen_position = Vector2::new(
            clip_space_position.x / clip_space_position.w + 1.0,
            clip_space_position.y / clip_space_position.w + 1.0,
        );
        let screen_position = screen_position / 2.0;
        let final_position = Vector2::new(screen_position.x * window_size.x, screen_position.y * window_size.y + 5.0);

        renderer.render_bar(
            render_target,
            final_position,
            Color::rgb(67, 163, 83),
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
        renderer.render_bar(
            render_target,
            final_position + Vector2::new(0.0, 5.0),
            Color::rgb(67, 129, 163),
            self.maximum_spell_points as f32,
            self.spell_points as f32,
        );
        renderer.render_bar(
            render_target,
            final_position + Vector2::new(0.0, 10.0),
            Color::rgb(163, 96, 67),
            self.maximum_activity_points as f32,
            self.activity_points as f32,
        );
    }
}

pub struct Npc {
    common: Common,
}

impl Npc {

    pub fn new(
        sprite_loader: &mut SpriteLoader,
        action_loader: &mut ActionLoader,
        texture_future: &mut Box<dyn GpuFuture + 'static>,
        map: &Map,
        database: &Database,
        entity_data: EntityData,
    ) -> Self {

        let common = Common::new(sprite_loader, action_loader, texture_future, map, database, entity_data);

        Self { common }
    }

    pub fn get_common(&self) -> &Common {
        &self.common
    }

    pub fn get_common_mut(&mut self) -> &mut Common {
        &mut self.common
    }

    pub fn render_status(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: Vector2<f32>,
    ) {

        let (view_matrix, projection_matrix) = camera.view_projection_matrices();
        let clip_space_position = (projection_matrix * view_matrix) * self.common.position.extend(1.0);
        let screen_position = Vector2::new(
            clip_space_position.x / clip_space_position.w + 1.0,
            clip_space_position.y / clip_space_position.w + 1.0,
        );
        let screen_position = screen_position / 2.0;
        let final_position = Vector2::new(screen_position.x * window_size.x, screen_position.y * window_size.y + 5.0);

        renderer.render_bar(
            render_target,
            final_position,
            Color::rgb(67, 163, 83),
            self.common.maximum_health_points as f32,
            self.common.health_points as f32,
        );
    }
}

pub enum Entity {
    Player(Player),
    Npc(Npc),
}

impl Entity {

    fn get_common(&self) -> &Common {
        match self {
            Self::Player(player) => player.get_common(),
            Self::Npc(npc) => npc.get_common(),
        }
    }

    fn get_common_mut(&mut self) -> &mut Common {
        match self {
            Self::Player(player) => player.get_common_mut(),
            Self::Npc(npc) => npc.get_common_mut(),
        }
    }

    pub fn get_entity_id(&self) -> u32 {
        self.get_common().entity_id
    }

    pub fn get_job(&self) -> usize {
        self.get_common().job_id
    }

    pub fn are_details_unavalible(&self) -> bool {
        match &self.get_common().details {
            ResourceState::Unavalible => true,
            _requested_or_avalible => false,
        }
    }

    pub fn set_details_requested(&mut self) {
        self.get_common_mut().details = ResourceState::Requested;
    }

    pub fn set_details(&mut self, details: String) {
        self.get_common_mut().details = ResourceState::Avalible(details);
    }

    pub fn get_details(&self) -> Option<&String> {
        self.get_common().details.as_option()
    }

    pub fn get_position(&self) -> Vector3<f32> {
        self.get_common().position
    }

    pub fn set_position(&mut self, map: &Map, position: Vector2<usize>) {
        self.get_common_mut().set_position(map, position);
    }

    pub fn update_health(&mut self, health_points: usize, maximum_health_points: usize) {

        let common = self.get_common_mut();
        common.health_points = health_points;
        common.maximum_health_points = maximum_health_points;
    }

    pub fn update(&mut self, map: &Map, delta_time: f32, client_tick: u32) {
        self.get_common_mut().update(map, delta_time, client_tick);
    }

    pub fn move_from_to(&mut self, map: &Map, from: Vector2<usize>, to: Vector2<usize>, starting_timestamp: u32) {
        self.get_common_mut().move_from_to(map, from, to, starting_timestamp);
    }

    #[cfg(feature = "debug")]
    pub fn generate_steps_vertex_buffer(&mut self, device: Arc<Device>, map: &Map) {
        self.get_common_mut().generate_steps_vertex_buffer(device, map);
    }

    pub fn render<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera)
    where
        T: Renderer + EntityRenderer,
    {
        self.get_common().render(render_target, renderer, camera);
    }

    #[cfg(feature = "debug")]
    pub fn render_marker<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, hovered: bool)
    where
        T: Renderer + MarkerRenderer,
    {
        self.get_common().render_marker(render_target, renderer, camera, hovered);
    }

    pub fn render_status(
        &self,
        render_target: &mut <DeferredRenderer as Renderer>::Target,
        renderer: &DeferredRenderer,
        camera: &dyn Camera,
        window_size: Vector2<f32>,
    ) {
        match self {
            Self::Player(player) => player.render_status(render_target, renderer, camera, window_size),
            Self::Npc(npc) => npc.render_status(render_target, renderer, camera, window_size),
        }
    }
}
