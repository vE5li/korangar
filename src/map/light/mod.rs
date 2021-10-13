use cgmath::Vector3;

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
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera) {
        renderer.render_light_icon(camera, self.position, self.color);
    }
}
