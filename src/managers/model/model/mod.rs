mod node;
mod shading;

use graphics::{ Renderer, Camera, Transform };

pub use self::node::Node;
pub use self::shading::ShadingType;

pub struct Model {
    root_node: Node,
}

impl Model {

    pub fn new(root_node: Node) -> Self {
        return Self { root_node };
    }

    pub fn render_geomitry(&self, renderer: &mut Renderer, camera: &dyn Camera, root_transform: &Transform) {
        self.root_node.render_geomitry(renderer, camera, &root_transform);
    }
}
