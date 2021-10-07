use graphics::{ Renderer, Camera, VertexBuffer, Texture, Transform };

#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub parent_name: Option<String>,
    pub child_nodes: Vec<Node>,
    textures: Vec<Texture>,
    transform: Transform,
    vertex_count: usize,
    vertex_buffer: VertexBuffer,
}

impl Node {

    pub fn new(name: String, parent_name: Option<String>, textures: Vec<Texture>, transform: Transform, vertex_count: usize, vertex_buffer: VertexBuffer) -> Self {
        let child_nodes = Vec::new();
        return Self { name, parent_name, child_nodes, textures, transform, vertex_count, vertex_buffer };
    }

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera, parent_transform: &Transform) {
        let combined_transform =  *parent_transform + self.transform;
        renderer.render_geomitry(camera, self.vertex_buffer.clone(), &self.textures, &combined_transform);
        self.child_nodes.iter().for_each(|node| node.render_geometry(renderer, camera, &combined_transform));
    }
}
