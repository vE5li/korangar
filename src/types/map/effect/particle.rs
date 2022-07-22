use derive_new::new;
use cgmath::Vector3;

#[cfg(feature = "debug")]
use crate::graphics::{ Renderer, Camera, MarkerRenderer };
use crate::graphics::Color;

#[derive(PrototypeElement, PrototypeWindow, new)]
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
    pub fn render_marker<T>(&self, render_target: &mut T::Target, renderer: &T, camera: &dyn Camera, hovered: bool)
        where T: Renderer + MarkerRenderer
    {
        renderer.render_marker(render_target, camera, self.position, hovered);
    }
}
