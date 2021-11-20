use maths::*;
use graphics::{ Renderer, Camera, ModelVertexBuffer, Texture, Transform };

#[derive(Clone, Debug)]
pub struct BoundingBox {
    pub smallest: Vector3<f32>,
    pub biggest: Vector3<f32>,
    pub offset: Vector3<f32>,
    pub range: Vector3<f32>,
}

impl BoundingBox {

    pub fn new(smallest: Vector3<f32>, biggest: Vector3<f32>, offset: Vector3<f32>, range: Vector3<f32>) -> Self {
        return Self { smallest, biggest, offset, range };
    }
}

#[derive(Clone)]
pub struct Node {
    pub name: String,
    pub parent_name: Option<String>,
    pub child_nodes: Vec<Node>,
    pub textures: Vec<Texture>,
    pub transform: Transform,
    pub vertex_buffer: ModelVertexBuffer,
    pub bounding_box: BoundingBox,
    pub offset_matrix: Matrix3<f32>,
    pub offset_translation: Vector3<f32>,
    pub position: Vector3<f32>,
    pub rotation: Vector3<Rad<f32>>,
    pub scale: Vector3<f32>,
}

impl Node {

    pub fn new(name: String, parent_name: Option<String>, textures: Vec<Texture>, transform: Transform, vertex_buffer: ModelVertexBuffer, bounding_box: BoundingBox,
        offset_matrix: Matrix3<f32>,
        offset_translation: Vector3<f32>,
        position: Vector3<f32>,
        rotation: Vector3<Rad<f32>>,
        scale: Vector3<f32>,
    ) -> Self {

        let child_nodes = Vec::new();

        return Self { name, parent_name, child_nodes, textures, transform, vertex_buffer, bounding_box,
            offset_matrix,
            offset_translation,
            position,
            rotation,
            scale,
         };
    }

    #[cfg(feature = "debug")]
    pub fn information(&self) -> String {
        return format!("\nname: {}\nnode count: {}\ntransform: {}", self.name, self.child_nodes.len(), self.transform.information());
    }

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera, parent_transform: &Transform) {
        //let combined_transform = *parent_transform + self.transform;
        renderer.render_node(camera, self, parent_transform);
        self.child_nodes.iter().for_each(|node| node.render_geometry(renderer, camera, parent_transform));
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, renderer: &mut Renderer, camera: &dyn Camera, parent_transform: &Transform) {
        let combined_transform = *parent_transform + self.transform + Transform::node_scale(self.bounding_box.range);
        renderer.render_bounding_box(camera, &combined_transform);
        self.child_nodes.iter().for_each(|node| node.render_bounding_box(renderer, camera, &combined_transform));
    }
}
