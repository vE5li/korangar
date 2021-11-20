use cgmath::{ Vector3, Vector2 };

use graphics::{ Renderer, Camera, Color };

pub struct LightSource {
    position: Vector3<f32>,
    color: Color,
    range: f32,
}

impl LightSource {

    pub fn new(position: Vector3<f32>, color: Color, range: f32) -> Self {
        return Self { position, color, range };
    }

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
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_light_marker(camera, self.position, self.color);
    }
}
