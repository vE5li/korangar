mod node;
mod shading;

use graphics::{ Renderer, Camera, Transform };

pub use self::node::Node;
pub use self::node::BoundingBox;
pub use self::shading::ShadingType;

pub struct Model {
    root_node: Node,
    pub bounding_box: BoundingBox,
}

impl Model {

    pub fn new(root_node: Node, bounding_box: BoundingBox) -> Self {
        return Self { root_node, bounding_box };
    }

    #[cfg(feature = "debug")]
    pub fn information(&self) -> String {
        return format!("\nroot node: {}\nbounding box: {:?}", self.root_node.information(), self.bounding_box);
    }

    pub fn render_geometry(&self, renderer: &mut Renderer, camera: &dyn Camera, root_transform: &Transform) {
        self.root_node.render_geometry(renderer, camera, &root_transform);
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, renderer: &mut Renderer, camera: &dyn Camera, root_transform: &Transform) {
        let combined_transform = *root_transform + Transform::node_scale(self.bounding_box.range);
        renderer.render_bounding_box(camera, &combined_transform);
    }

    #[cfg(feature = "debug")]
    pub fn render_node_bounding_boxes(&self, renderer: &mut Renderer, camera: &dyn Camera, root_transform: &Transform) {
        self.root_node.render_bounding_box(renderer, camera, root_transform);
    }
}
