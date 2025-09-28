use cgmath::{Deg, Matrix3, Vector3};
use korangar_interface::element::StateElement;
use ragnarok_formats::map::LightSettings;
use rust_state::RustState;

use crate::graphics::Color;

#[derive(RustState, StateElement)]
pub struct Lighting {
    ambient_color: Color,
    diffuse_color: Color,
    light_latitude: f32,
    light_longitude: f32,
}

impl Lighting {
    pub fn new(settings: LightSettings) -> Self {
        Self {
            ambient_color: settings.ambient_color.unwrap().into(),
            diffuse_color: settings.diffuse_color.unwrap().into(),
            light_latitude: settings.light_latitude.unwrap() as f32,
            light_longitude: settings.light_longitude.unwrap() as f32,
        }
    }

    pub fn ambient_light_color(&self) -> Color {
        self.ambient_color
    }

    pub fn directional_light(&self) -> (Vector3<f32>, Color) {
        let rotation_around_x = Matrix3::from_angle_x(Deg(-self.light_latitude));
        let rotation_around_y = Matrix3::from_angle_y(Deg(self.light_longitude));
        let light_direction = rotation_around_y * (rotation_around_x * Vector3::new(0.0, 1.0, 0.0));

        (light_direction, self.diffuse_color)
    }
}
