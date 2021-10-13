use cgmath::Vector3;

use graphics::Color;

#[derive(Clone, Debug)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub light_color: Color,
    pub light_range: f32,
}

impl Particle {

    pub fn new(position: Vector3<f32>, light_color: Color, light_range: f32) -> Self {
        return Self { position, light_color, light_range }
    }

    pub fn update(&mut self, delta_time: f32) -> bool {
        self.position.y += 10.0 * delta_time;

        return self.position.y < 30.0;
    }
}
