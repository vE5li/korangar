use std::sync::Arc;

use korangar_interface::element::StateElement;
use korangar_interface::window::StateWindow;
use korangar_util::collision::AABB;
use ragnarok_formats::transform::Transform;
use rust_state::RustState;

#[cfg(feature = "debug")]
use super::MarkerIdentifier;
use super::Model;
use crate::Camera;
#[cfg(feature = "debug")]
use crate::graphics::Color;
#[cfg(feature = "debug")]
use crate::graphics::DebugAabbInstruction;
use crate::graphics::ModelInstruction;
#[cfg(feature = "debug")]
use crate::renderer::MarkerRenderer;

#[derive(Clone, RustState, StateElement, StateWindow)]
pub struct Object {
    pub name: Option<String>,
    pub model_name: String,
    pub model: Arc<Model>,
    pub transform: Transform,
}

impl Object {
    pub fn new(name: Option<String>, model_name: String, model: Arc<Model>, transform: Transform) -> Self {
        Self {
            name,
            model_name,
            model,
            transform,
        }
    }
}

impl Object {
    pub fn render_geometry(&self, instructions: &mut Vec<ModelInstruction>, animation_timer_ms: f32, camera: &dyn Camera) {
        self.model
            .render_geometry(instructions, &self.transform, animation_timer_ms, camera);
    }

    pub fn calculate_object_aabb(&self) -> AABB {
        self.model.calculate_aabb(&self.transform)
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
