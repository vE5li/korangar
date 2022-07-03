use derive_new::new;

#[cfg(feature = "debug")]
use crate::graphics::{ Renderer, Camera };
use crate::types::maths::*;

#[derive(PrototypeElement, PrototypeWindow, new)]
#[window_title("sound source")]
pub struct SoundSource {
    pub name: String,
    pub sound_file: String,
    pub position: Vector3<f32>,
    pub volume: f32,
    pub width: usize,
    pub height: usize,
    pub range: f32,
    pub cycle: f32,
}

impl SoundSource {

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    pub fn hovered(&self, renderer: &Renderer, camera: &dyn Camera, mouse_position: Vector2<f32>, smallest_distance: f32) -> Option<f32> {
        let distance = camera.distance_to(self.position);

        match distance < smallest_distance && renderer.marker_hovered(camera, self.position, mouse_position) {
            true => Some(distance),
            false => None,
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera, hovered: bool) {
        renderer.render_sound_marker(camera, self.position, hovered);
    }
}
