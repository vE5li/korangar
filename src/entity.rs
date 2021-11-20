use std::sync::Arc;

use cgmath::{ Vector3, Vector2 };
use vulkano::device::Device;
use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::sync::GpuFuture;

use graphics::{ Renderer, Camera, ModelVertexBuffer, NativeModelVertex, Texture, Transform };
use map::Map;
use loaders::TextureLoader;

pub struct Entity {
    pub position: Vector3<f32>,

    steps: Vec<Vector2<usize>>,
    //position: Vector2<f32>,

    #[cfg(feature = "debug")]
    pub steps_vertex_buffer: Option<ModelVertexBuffer>,

    pub maximum_health_points: usize,
    pub maximum_spell_points: usize,
    pub maximum_activity_points: usize,
    pub current_health_points: usize,
    pub current_spell_points: usize,
    pub current_activity_points: usize,

    timer: f32,

    texture: Texture,
}

impl Entity {

    pub fn new(texture_loader: &mut TextureLoader, texture_future: &mut Box<dyn GpuFuture + 'static>) -> Self {

        let position = Vector3::new(400.0, 0.0, 400.0);
        let steps = Vec::new();

        #[cfg(feature = "debug")]
        let steps_vertex_buffer = None;

        let maximum_health_points = 10000;
        let maximum_spell_points = 200;
        let maximum_activity_points = 500;
        let current_health_points = 100;
        let current_spell_points = 50;
        let current_activity_points = 0;

        let timer = 0.0;

        let texture = texture_loader.get(String::from("assets/player.png"), texture_future); // 8 x 14

        return Self {
            position,
            steps,

            #[cfg(feature = "debug")]
            steps_vertex_buffer,

            maximum_health_points,
            maximum_spell_points,
            maximum_activity_points,
            current_health_points,
            current_spell_points,
            current_activity_points,

            timer,

            texture,
        };
    }

    pub fn move_from_to(&mut self, map: &Map, from: Vector2<usize>, to: Vector2<usize>) -> Vector3<f32> {

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

                if map.x_in_bounds(x + 1) && map.y_in_bounds(y + 1) {
                    if map.get_tile(&Vector2::new(x + 1, y)).is_walkable() && map.get_tile(&Vector2::new(x, y + 1)).is_walkable() {
                        successors.push(Pos(x + 1, y + 1));
                    }
                }

                if x > 0 && map.y_in_bounds(y + 1) {
                    if map.get_tile(&Vector2::new(x - 1, y)).is_walkable() && map.get_tile(&Vector2::new(x, y + 1)).is_walkable() {
                        successors.push(Pos(x - 1, y + 1));
                    }
                }

                if map.x_in_bounds(x + 1) && y > 0 {
                    if map.get_tile(&Vector2::new(x + 1, y)).is_walkable() && map.get_tile(&Vector2::new(x, y - 1)).is_walkable() {
                        successors.push(Pos(x + 1, y - 1));
                    }
                }

                if x > 0 && y > 0 {
                    if map.get_tile(&Vector2::new(x - 1, y)).is_walkable() && map.get_tile(&Vector2::new(x, y - 1)).is_walkable() {
                        successors.push(Pos(x - 1, y - 1));
                    }
                }

                let successors = successors.drain(..)
                    .filter(|Pos(x, y)| map.get_tile(&Vector2::new(*x, *y)).is_walkable())
                    .collect::<Vec<Pos>>();

                return successors;
            }

            fn to_vector(self) -> Vector2<usize> {
                return Vector2::new(self.0, self.1);
            }
        }

        let result = bfs(&Pos(from.x, from.y), |p| p.successors(map), |p| *p == Pos(to.x, to.y));

        if let Some(path) = result {
            self.steps = path.into_iter().map(|pos| pos.to_vector()).collect();
        }

        let height = 0.0; // interpolate height from tile
        self.position = Vector3::new(to.x as f32 * 5.0 + 2.5, height, to.y as f32 * 5.0 + 2.5);

        return self.position;
    }

    #[cfg(feature = "debug")]
    fn generate_step_texture_coordinates(steps: &Vec<Vector2<usize>>, step: &Vector2<usize>, index: usize) -> ([Vector2<f32>; 4], i32) {

        if steps.len() - 1 == index {
            return ([Vector2::new(0.0, 1.0), Vector2::new(1.0, 1.0), Vector2::new(1.0, 0.0), Vector2::new(0.0, 0.0)], 0);
        }

        let delta = steps[index + 1].map(|component| component as isize) - step.map(|component| component as isize);

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

        for (index, step) in self.steps.iter().enumerate() {

            let tile = map.get_tile(step);
            let offset = Vector2::new(step.x as f32 * 5.0, step.y as f32 * 5.0);

            let first_position = Vector3::new(offset.x, -tile.upper_left_height + 1.0, offset.y);
            let second_position = Vector3::new(offset.x + 5.0, -tile.upper_right_height + 1.0, offset.y);
            let third_position = Vector3::new(offset.x + 5.0, -tile.lower_right_height + 1.0, offset.y + 5.0);
            let fourth_position = Vector3::new(offset.x, -tile.lower_left_height + 1.0, offset.y + 5.0);

            let first_normal = NativeModelVertex::calculate_normal(first_position, second_position, third_position);
            let second_normal = NativeModelVertex::calculate_normal(fourth_position, first_position, third_position);

            let (texture_coordinates, texture_index) = Self::generate_step_texture_coordinates(&self.steps, step, index);

            native_steps_vertices.push(NativeModelVertex::new(first_position, first_normal, texture_coordinates[0], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(second_position, first_normal, texture_coordinates[1], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(third_position, first_normal, texture_coordinates[2], texture_index));

            native_steps_vertices.push(NativeModelVertex::new(first_position, second_normal, texture_coordinates[0], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(third_position, second_normal, texture_coordinates[2], texture_index));
            native_steps_vertices.push(NativeModelVertex::new(fourth_position, second_normal, texture_coordinates[3], texture_index));
        }

        let steps_vertices = NativeModelVertex::to_vertices(native_steps_vertices);
        let vertex_buffer = CpuAccessibleBuffer::from_iter(device, BufferUsage::all(), false, steps_vertices.into_iter()).unwrap();
        self.steps_vertex_buffer = Some(vertex_buffer);
    }

    pub fn update(&mut self, delta_time: f32) {

        self.timer += delta_time;

        if self.timer > 0.3 {
            self.timer -= 0.3;

            if self.current_health_points < self.maximum_health_points {
                self.current_health_points += 100;
            }

            if self.current_spell_points < self.maximum_spell_points {
                self.current_spell_points += 1;
            }

            if self.current_activity_points < self.maximum_activity_points {
                self.current_activity_points += 1;
            }
        }
    }

    pub fn render(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_entity(camera, self.texture.clone(), self.position, Vector3::new(0.0, 3.0, 0.0), Vector2::new(5.0, 10.0), Vector2::new(16, 8), Vector2::new(0, 0));
    }

    #[cfg(feature = "debug")]
    pub fn render_pathing(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        if let Some(vertex_buffer) = self.steps_vertex_buffer.clone() {
            renderer.render_pathing(camera, vertex_buffer, &Transform::new());
        }
    }
}
