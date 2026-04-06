use cgmath::{Deg, Matrix3, Vector3};
use korangar_interface::element::StateElement;
use ragnarok_formats::map::LightSettings;
use rust_state::RustState;

use crate::graphics::Color;

// Some RO maps ship with lighting values that look reasonable in the original
// client but become unreadably dark in Korangar's modern renderer, so we keep
// a conservative floor for ambient and direct light here.
const MINIMUM_AMBIENT_LIGHT: Color = Color::rgb(0.7, 0.7, 0.7);
const MINIMUM_DIFFUSE_LIGHT: Color = Color::rgb(0.9, 0.9, 0.9);
const FILL_LIGHT_STRENGTH: f32 = 0.2;
const MINIMUM_LIGHT_LATITUDE: f32 = 60.0;

#[derive(RustState, StateElement)]
pub struct Lighting {
    ambient_color: Color,
    diffuse_color: Color,
    light_latitude: f32,
    light_longitude: f32,
}

impl Lighting {
    pub fn new(settings: LightSettings) -> Self {
        let ambient_color = normalize_ambient(settings.ambient_color.unwrap().into(), settings.diffuse_color.unwrap().into());
        let diffuse_color = normalize_diffuse(settings.diffuse_color.unwrap().into());

        Self {
            ambient_color,
            diffuse_color,
            // Very low sun angles produce long shadows and make already dark RO
            // maps much harder to read, so we clamp the latitude upwards.
            light_latitude: (settings.light_latitude.unwrap() as f32).max(MINIMUM_LIGHT_LATITUDE),
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

fn normalize_ambient(ambient_color: Color, diffuse_color: Color) -> Color {
    // A small fill component keeps midtones visible even when a map's ambient
    // settings were authored for the brighter behavior of the original client.
    clamp_color(max_color(ambient_color + diffuse_color * FILL_LIGHT_STRENGTH, MINIMUM_AMBIENT_LIGHT))
}

fn normalize_diffuse(diffuse_color: Color) -> Color {
    clamp_color(max_color(diffuse_color, MINIMUM_DIFFUSE_LIGHT))
}

fn max_color(left: Color, right: Color) -> Color {
    Color::rgb(
        left.red.max(right.red),
        left.green.max(right.green),
        left.blue.max(right.blue),
    )
}

fn clamp_color(color: Color) -> Color {
    Color::rgb(
        color.red.clamp(0.0, 1.0),
        color.green.clamp(0.0, 1.0),
        color.blue.clamp(0.0, 1.0),
    )
}
