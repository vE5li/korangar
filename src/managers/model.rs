use std::collections::HashMap;
use std::fs::read_to_string;
use std::sync::Arc;

use vulkano::buffer::{ BufferUsage, CpuAccessibleBuffer };
use vulkano::device::Device;

#[cfg(feature = "debug")]
use debug::*;

use cgmath::{ Vector2, Vector3, InnerSpace };
use graphics::{ VertexBuffer, Vertex };

struct PartialVertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub texture_coordinates: Vector2<f32>,
}

impl PartialVertex {

    pub fn new(position: Vector3<f32>, normal: Vector3<f32>, texture_coordinates: Vector2<f32>) -> Self {
        return Self {
            position: position,
            normal: normal,
            texture_coordinates: texture_coordinates,
        }
    }
}

fn partial_vertex(vertex_positions: &Vec<Vector3<f32>>, normals: &Vec<Vector3<f32>>, texture_coordinates: &Vec<Vector2<f32>>, word: &str) -> PartialVertex {

    let mut components = word.split("/");
    let position_index: usize = components.next().expect("missing vertex position index").parse().expect("failed to parse vertex position index");
    let texture_index: usize = components.next().expect("missing vertex texture index").parse().expect("failed to parse vertex texture index");
    let normal_index: usize = components.next().expect("missing vertex normal index").parse().expect("failed to parse vertex normal index");

    let position_entry = &vertex_positions[position_index - 1];
    let texture_entry = &texture_coordinates[texture_index - 1];
    let normal_entry = &normals[normal_index - 1];

    return PartialVertex::new(*position_entry, *normal_entry, *texture_entry);
}

fn calculate_tangent_bitangent(first_partial: &PartialVertex, second_partial: &PartialVertex, third_partial: &PartialVertex) -> (Vector3<f32>, Vector3<f32>) {

    let delta_position_1 = second_partial.position - first_partial.position;
    let delta_position_2 = third_partial.position - first_partial.position;
    let delta_texture_coordinates_1 = second_partial.texture_coordinates - first_partial.texture_coordinates;
    let delta_texture_coordinates_2 = third_partial.texture_coordinates - first_partial.texture_coordinates;

    let r = 1.0 / (delta_texture_coordinates_1.x * delta_texture_coordinates_2.y - delta_texture_coordinates_1.y * delta_texture_coordinates_2.x);
    let tangent = (delta_position_1 * delta_texture_coordinates_2.y - delta_position_2 * delta_texture_coordinates_1.y) * r;
    let bitangent = (delta_position_2 * delta_texture_coordinates_1.x - delta_position_1 * delta_texture_coordinates_2.x) * r;

    return (tangent.normalize(), bitangent.normalize());
}

fn vertex_from_partial(partial_vertex: PartialVertex, tangent: Vector3<f32>, bitangent: Vector3<f32>) -> Vertex {
    return Vertex::new(partial_vertex.position, partial_vertex.normal, tangent, bitangent, partial_vertex.texture_coordinates);
}

pub struct ModelManager {
    cache: HashMap<String, VertexBuffer>,
    device: Arc<Device>,
}

impl ModelManager {

    pub fn new(device: Arc<Device>) -> Self {
        return Self {
            cache: HashMap::new(),
            device: device,
        }
    }

    fn load(&mut self, path: String) -> VertexBuffer {

        #[cfg(feature = "debug")]
        let timer = Timer::new_dynamic(format!("load model from {}{}{}", magenta(), path, none()));

        let mut vertex_positions: Vec<Vector3<f32>> = Vec::new();
        let mut normals: Vec<Vector3<f32>> = Vec::new();
        let mut texture_coordinates: Vec<Vector2<f32>> = Vec::new();
        let mut vertices: Vec<Vertex> = Vec::new();

        let contents = read_to_string(&path).expect("something went wrong reading the file");
        let mut lines = contents.split("\n");

        while let Some(line) = lines.next() {
            let mut words = line.split(" ");
            let line_type = words.next().expect("failed to get line type");

            if line_type.is_empty() {
                continue;
            }

            match line_type {

                "v" => {
                    let x = words.next().expect("failed to get x coordinate").parse().unwrap();
                    let y = words.next().expect("failed to get y coordinate").parse().unwrap();
                    let z = words.next().expect("failed to get z coordinate").parse().unwrap();
                    vertex_positions.push(Vector3::new(x, y, z));
                }

                "vn" => {
                    let nx = words.next().expect("failed to get normal x coordinate").parse().unwrap();
                    let ny = words.next().expect("failed to get normal y coordinate").parse().unwrap();
                    let nz = words.next().expect("failed to get normal z coordinate").parse().unwrap();
                    normals.push(Vector3::new(nx, ny, nz));
                }

                "vt" => {
                    let u = words.next().expect("failed to get u coordinate").parse().unwrap();
                    let v = words.next().expect("failed to get v coordinate").parse().unwrap();
                    texture_coordinates.push(Vector2::new(u, v));
                }

                "f" => {
                    let first = words.next().expect("failed to get first vertex");
                    let first_partial = partial_vertex(&vertex_positions, &normals, &texture_coordinates, first);
                    let second = words.next().expect("failed to get second vertex");
                    let second_partial = partial_vertex(&vertex_positions, &normals, &texture_coordinates, second);
                    let third = words.next().expect("failed to get third vertex");
                    let third_partial = partial_vertex(&vertex_positions, &normals, &texture_coordinates, third);
                    let (tangent, bitangent) = calculate_tangent_bitangent(&first_partial, &second_partial, &third_partial);

                    vertices.push(vertex_from_partial(first_partial, tangent, bitangent));
                    vertices.push(vertex_from_partial(second_partial, tangent, bitangent));
                    vertices.push(vertex_from_partial(third_partial, tangent, bitangent));
                }

                "o" | "#" | "s" => continue,

                invalid => println!("invalid type {:?}", invalid),
            }
        }

        let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, vertices.into_iter()).unwrap();
        self.cache.insert(path, vertex_buffer.clone());

        #[cfg(feature = "debug")]
        timer.stop();
        
        return vertex_buffer;
    }

    pub fn get(&mut self, path: String) -> VertexBuffer {
        match self.cache.get(&path) {
            Some(model) => return model.clone(),
            None => return self.load(path),
        }
    }
}
