use derive_new::new;
use cgmath::Vector3;

#[cfg(feature = "debug")]
use crate::graphics::{ Renderer, Camera };
use crate::graphics::Color;

#[derive(PrototypeElement, PrototypeWindow, new)]
#[window_title("particle")]
pub struct Particle {
    pub position: Vector3<f32>,
    pub light_color: Color,
    pub light_range: f32,
}

impl Particle {

    pub fn update(&mut self, delta_time: f32) -> bool {
        self.position.y += 10.0 * delta_time;

        self.position.y < 30.0
    }

    #[cfg(feature = "debug")]
    pub fn render_marker(&self, renderer: &mut Renderer, camera: &dyn Camera, hovered: bool) {
        renderer.render_particle_marker(camera, self.position, hovered);
    }
}
