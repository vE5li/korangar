use derive_new::new;
use maths::*;
use graphics::{ Renderer, Camera, ModelVertexBuffer, Texture, Transform };

#[derive(Clone, Debug, new)]
pub struct BoundingBox {
    pub smallest: Vector3<f32>,
    pub biggest: Vector3<f32>,
    pub offset: Vector3<f32>,
    pub range: Vector3<f32>,
}

#[derive(Clone, new)]
pub struct Node {
    pub name: String,
    pub parent_name: Option<String>,
    #[new(default)]
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
