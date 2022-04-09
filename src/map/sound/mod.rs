use derive_new::new;
use cgmath::{ Vector3, Vector2 };

#[cfg(feature = "debug")]
use graphics::{ Renderer, Camera };

#[derive(Clone, new)]
pub struct SoundSource {
    pub position: Vector3<f32>,
    pub range: f32,
}

impl SoundSource {

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    #[cfg(feature = "debug")]
    pub fn hovered(&self, renderer: &Renderer, camera: &dyn Camera, mouse_position: Vector2<f32>, smallest_distance: f32) -> Option<f32> {
        let distance = camera.distance_to(self.position);

        match distance < smallest_distance && renderer.marker_hovered(camera, self.position, mouse_position) {
            true => return Some(distance),
            false => return None,
        }
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_sound_marker(camera, self.position);
    }
}
