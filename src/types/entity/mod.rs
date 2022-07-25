use derive_new::new;
use std::sync::Arc;
use cgmath::{ Vector3, Vector2, VectorSpace };
use vulkano::sync::GpuFuture;
#[cfg(feature = "debug")]
use vulkano::device::Device;
#[cfg(feature = "debug")]
use vulkano::buffer::{ CpuAccessibleBuffer, BufferUsage };

#[cfg(feature = "debug")]
use crate::graphics::{ ModelVertexBuffer, NativeModelVertex, Transform };
use crate::graphics::{ Renderer, EntityRenderer, Camera, MarkerRenderer, DeferredRenderer };
use crate::types::map::{ Map, MarkerIdentifier };
use crate::loaders::{ TextureLoader, SpriteLoader, ActionLoader };
use crate::loaders::{ Sprite, Actions};
use crate::database::Database;

#[derive(Clone, new, PrototypeElement)]
struct Movement {
    #[hidden_element]
    steps: Vec<(Vector2<usize>, u32)>,
    starting_timestamp: u32,
    #[cfg(feature = "debug")]
    #[new(default)]
    pub steps_vertex_buffer: Option<ModelVertexBuffer>,
}

#[derive(Clone, PrototypeWindow)]
pub struct Entity {
    pub position: Vector3<f32>,
    pub entity_id: usize,
    pub job_id: usize,

    active_movement: Option<Movement>,
    movement_speed: usize,

    pub maximum_health_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
    pub current_health_points: usize,
    pub current_spell_points: usize,
    pub current_activity_points: usize,

    sprite: Arc<Sprite>,
    actions: Arc<Actions>,

    timer: f32,
    counter: usize,
}

impl Entity {

    pub fn new(sprite_loader: &mut SpriteLoader, action_loader: &mut ActionLoader, texture_future: &mut Box<dyn GpuFuture + 'static>, map: &Map, database: &Database, entity_id: usize, job_id: usize, position: Vector2<usize>, _movement_speed: usize) -> Self {

        let position = Vector3::new(position.x as f32 * 5.0 + 2.5, map.get_height_at(position), position.y as f32 * 5.0 + 2.5);
        let active_movement = None;
        let movement_speed = 300;

        let maximum_health_points = 10000;
        let maximum_spell_points = 200;
        let maximum_activity_points = 500;
        let current_health_points = 100;
        let current_spell_points = 50;
        let current_activity_points = 0;

        let file_path = format!("npc\\{}", database.job_name_from_id(job_id));
        let sprite = sprite_loader.get(&format!("{}.spr", file_path), texture_future).unwrap();
        let actions = action_loader.get(&format!("{}.act", file_path)).unwrap();

        Self {
            position,
            entity_id,
            job_id,
            active_movement,
            movement_speed,
            maximum_health_points,
            maximum_spell_points,
            maximum_activity_points,
            current_health_points,
            current_spell_points,
            current_activity_points,
            sprite,
            actions,

            timer: 0.0,
            counter: 0,
        }
    }

    pub fn set_position(&mut self, map: &Map, position: Vector2<usize>) {
        self.position = Vector3::new(position.x as f32 * 5.0 + 2.5, map.get_height_at(position), position.y as f32 * 5.0 + 2.5);
        self.active_movement = None;
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

                if map.x_in_bounds(x + 1) && map.y_in_bounds(y + 1) && map.get_tile(Vector2::new(x + 1, y)).is_walkable() && map.get_tile(Vector2::new(x, y + 1)).is_walkable() {
                    successors.push(Pos(x + 1, y + 1));
                }

                if x > 0 && map.y_in_bounds(y + 1) && map.get_tile(Vector2::new(x - 1, y)).is_walkable() && map.get_tile(Vector2::new(x, y + 1)).is_walkable() {
                    successors.push(Pos(x - 1, y + 1));
                }

                if map.x_in_bounds(x + 1) && y > 0 && map.get_tile(Vector2::new(x + 1, y)).is_walkable() && map.get_tile(Vector2::new(x, y - 1)).is_walkable() {
                    successors.push(Pos(x + 1, y - 1));
                }

                if x > 0 && y > 0 && map.get_tile(Vector2::new(x - 1, y)).is_walkable() && map.get_tile(Vector2::new(x, y - 1)).is_walkable() {
                    successors.push(Pos(x - 1, y - 1));
                }

                let successors = successors.drain(..)
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
            let steps: Vec<(Vector2<usize>, u32)> = path.into_iter().enumerate().map(|(index, pos)| {
                let arrival_timestamp = starting_timestamp + index as u32 * (self.movement_speed as u32 / 2);
                (pos.to_vector(), arrival_timestamp)
            }).collect();

            self.active_movement = Movement::new(steps, starting_timestamp).into();
        }
    }

    #[cfg(feature = "debug")]
    fn generate_step_texture_coordinates(steps: &Vec<(Vector2<usize>, u32)>, step: Vector2<usize>, index: usize) -> ([Vector2<f32>; 4], i32) {

        if steps.len() - 1 == index {
            return ([Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0), Vector2::new(0.0, 0.0)], 0);
        }

        let delta = steps[index + 1].0.map(|component| component as isize) - step.map(|component| component as isize);

        match delta {
            Vector2 { x: 1, y: 0 } => ([Vector2::new(0.0, 0.0), Vector2::new(1.0, 0.0), Vector2::new(1.0, 1.0), Vector2::new(0.0, 1.0)], 1),
            Vector2 { x: -1, y: 0 } => ([Vector2::new(1.0, 0.0), Vector2::new(0.0, 0.0), Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0)], 1),
            Vector2 { x: 0, y: 1 } => ([Vector2::new(0.0, 0.0), Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0)], 1),
            Vector2 { x: 0, y: -1 } => ([Vector2::new(1.0, 0.0), Vector2::new(1.0, 1.0), Vector2::new(0.0, 1.0), Vector2::new(0.0, 0.0)], 1),
            Vector2 { x: 1, y: 1 } => ([Vector2::new(0.0, 1.0), Vector2::new(0.0, 0.0), Vector2::new(1.0, 0.0), Vector2::new(1.0, 1.0)], 2),
            Vector2 { x: -1, y: 1 } => ([Vector2::new(0.0, 0.0), Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0)], 2),
            Vector2 { x: 1, y: -1 } => ([Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0), Vector2::new(0.0, 0.0), Vector2::new(0.0, 1.0)], 2),
            Vector2 { x: -1, y: -1 } => ([Vector2::new(1.0, 0.0), Vector2::new(1.0, 1.0), Vector2::new(0.0, 1.0), Vector2::new(0.0, 0.0)], 2),
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

            native_steps_vertices.push(NativeModelVertex::new(first_position, first_normal, texture_coordinates[0], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(second_position, first_normal, texture_coordinates[1], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(third_position, first_normal, texture_coordinates[2], texture_index));

            native_steps_vertices.push(NativeModelVertex::new(first_position, second_normal, texture_coordinates[0], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(third_position, second_normal, texture_coordinates[2], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(fourth_position, second_normal, texture_coordinates[3], texture_index));
        }

        let steps_vertices = NativeModelVertex::to_vertices(native_steps_vertices);
        let vertex_buffer = CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), false, steps_vertices.into_iter()).unwrap();
        active_movement.steps_vertex_buffer = Some(vertex_buffer);
    }

    pub fn has_updates(&self) -> bool {
        self.active_movement.is_some()
    }

    pub fn update(&mut self, map: &Map, delta_time: f32, client_tick: u32) {

        if let Some(active_movement) = self.active_movement.take() {

            let last_step = active_movement.steps.last().unwrap();

            if client_tick > last_step.1 {
                self.set_position(map, Vector2::new(last_step.0.x, last_step.0.y));
            } else {

                let mut last_step_index = 0;
                while active_movement.steps[last_step_index + 1].1 < client_tick { 
                    last_step_index += 1;
                }

                let last_step = active_movement.steps[last_step_index];
                let next_step = active_movement.steps[last_step_index + 1];

                let last_step_position = Vector3::new(last_step.0.x as f32 * 5.0 + 2.5, map.get_height_at(last_step.0), last_step.0.y as f32 * 5.0 + 2.5);
                let next_step_position = Vector3::new(next_step.0.x as f32 * 5.0 + 2.5, map.get_height_at(next_step.0), next_step.0.y as f32 * 5.0 + 2.5);

                let clamped_tick = u32::max(last_step.1, client_tick);
                let total = next_step.1 - last_step.1;
                let offset = clamped_tick - last_step.1;

                let movement_elapsed = (1.0 / total as f32) * offset as f32;
                let current_position = last_step_position.lerp(next_step_position, movement_elapsed);

                self.position = current_position;
                self.active_movement = active_movement.into();
            }
        }


        self.timer += delta_time;
        if self.timer > 1.0 {
            self.timer -= 1.0;
            self.counter = (self.counter + 1) % self.sprite.textures.len();
        }
    }

    pub fn render<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera)
        where T: Renderer + EntityRenderer
    {
        renderer.render_entity(render_target, camera, self.sprite.textures[0].clone(), self.position, Vector3::new(0.0, 3.0, 0.0), Vector2::new(5.0, 10.0), Vector2::new(1, 1), Vector2::new(0, 0), self.entity_id);
    }

    #[cfg(feature = "debug")]
    pub fn render_marker<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, hovered: bool)
        where T: Renderer + MarkerRenderer
    {
        renderer.render_marker(render_target, camera, self.position, hovered);
    }

    #[cfg(feature = "debug")]
    pub fn render_pathing(&self, render_target: &mut <DeferredRenderer as Renderer>::Target, renderer: &DeferredRenderer, camera: &dyn Camera) {
        /*if let Some(active_movement) = &self.active_movement {
            let vertex_buffer = active_movement.steps_vertex_buffer.clone().unwrap();
            renderer.render_pathing(render_target, camera, vertex_buffer, &Transform::new());
        }*/
    }
}
