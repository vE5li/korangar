use cgmath::Point3;

use crate::graphics::{Camera, MarkerInstruction};
use crate::renderer::MarkerRenderer;
use crate::world::MarkerIdentifier;

#[derive(Default)]
pub struct DebugMarkerRenderer {
    instructions: Vec<MarkerInstruction>,
}

impl DebugMarkerRenderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.instructions.clear();
    }

    pub fn get_instructions(&self) -> &[MarkerInstruction] {
        self.instructions.as_ref()
    }
}

impl MarkerRenderer for DebugMarkerRenderer {
    fn render_marker(&mut self, camera: &dyn Camera, marker_identifier: MarkerIdentifier, position: Point3<f32>, hovered: bool) {
        let (top_left_position, bottom_right_position) = camera.billboard_coordinates(position, MarkerIdentifier::SIZE);

        if top_left_position.w >= 0.1 && bottom_right_position.w >= 0.1 {
            let (screen_position, screen_size) = camera.screen_position_size(top_left_position, bottom_right_position);

            self.instructions.push(MarkerInstruction {
                screen_position,
                screen_size,
                identifier: marker_identifier,
                hovered,
            });
        }
    }
}
