use std::sync::Arc;

use cgmath::Matrix4;
use derive_new::new;
use korangar_interface::elements::PrototypeElement;
use korangar_interface::windows::PrototypeWindow;
use ragnarok_formats::transform::Transform;
use ragnarok_packets::ClientTick;

#[cfg(feature = "debug")]
use super::MarkerIdentifier;
use super::Model;
#[cfg(feature = "debug")]
use crate::graphics::Color;
#[cfg(feature = "debug")]
use crate::graphics::DebugAabbInstruction;
use crate::graphics::ModelInstruction;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;
#[cfg(feature = "debug")]
use crate::Camera;

#[derive(PrototypeElement, PrototypeWindow, new)]
pub struct Object {
    pub name: Option<String>,
    pub model_name: String,
    pub model: Arc<Model>,
    pub transform: Transform,
}

impl Object {
    pub fn render_geometry(&self, instructions: &mut Vec<ModelInstruction>, client_tick: ClientTick) {
        self.model.render_geometry(instructions, &self.transform, client_tick);
    }

    pub fn get_bounding_box_matrix(&self) -> Matrix4<f32> {
        self.model.get_bounding_box_matrix(&self.transform)
    }

    #[cfg(feature = "debug")]
    pub fn render_bounding_box(&self, instructions: &mut Vec<DebugAabbInstruction>, color: Color) {
        self.model.render_bounding_box(instructions, &self.transform, color);
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(
        &self,
        renderer: &mut impl MarkerRenderer,
        camera: &dyn Camera,
        marker_identifier: MarkerIdentifier,
        hovered: bool,
    ) {
        renderer.render_marker(camera, marker_identifier, self.transform.position, hovered);
    }
}
