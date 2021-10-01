use graphics::{ Renderer, Camera, VertexBuffer, Texture, Transform };

pub struct Map {
    ground_vertex_buffer: VertexBuffer,
    ground_textures: Vec<Texture>,
}

impl Map {

    pub fn new(ground_vertex_buffer: VertexBuffer, ground_textures: Vec<Texture>) -> Self {
        return Self { ground_vertex_buffer, ground_textures };
    }

    pub fn render_geomitry(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_geomitry(camera, self.ground_vertex_buffer.clone(), &self.ground_textures, &Transform::new());
    }
}
