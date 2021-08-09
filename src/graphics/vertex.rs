#[derive(Default, Debug, Clone, Copy)]
pub struct Vertex {
    position: [f32; 3],
    texture_coordinates: [f32; 2],
}

impl Vertex {

    pub const fn new(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
        return Self {
            position: [x, y, z],
            texture_coordinates: [u, v],
        }
    }
}

vulkano::impl_vertex!(Vertex, position, texture_coordinates);

