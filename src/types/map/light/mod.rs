use derive_new::new;
use cgmath::{ Vector3, Vector2 };

use graphics::{ Renderer, Camera, Color };

#[derive(PrototypeElement, PrototypeWindow, new)]
pub struct LightSource {
    #[window_title("light source")]
    pub name: String,
    pub position: Vector3<f32>,
    pub color: Color,
    pub range: f32,
}

impl LightSource {

    pub fn offset(&mut self, offset: Vector3<f32>) {
        self.position += offset;
    }

    pub fn render_lights(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.point_light(camera, self.position, self.color, self.range);
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
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera, hovered: bool) {
        renderer.render_light_marker(camera, self.position, self.color, hovered);
    }
}
